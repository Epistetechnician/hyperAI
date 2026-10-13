//! Authenticated local Data catalog, query, cancellation, and saved-definition API checks.

use std::{error::Error, path::PathBuf, time::Duration, time::Instant};

use hyperai_foundation::Sha256Digest;
use hyperai_local_service::{
    AuthState,
    lifecycle::{self, ServiceRecordV1},
    server,
    storage::{self, HomePaths},
};
use reqwest::{Client, StatusCode, header};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::task::JoinHandle;

async fn launch(
    home: PathBuf,
) -> Result<
    (
        JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>,
        HomePaths,
        ServiceRecordV1,
        String,
    ),
    Box<dyn Error + Send + Sync>,
> {
    let server_home = home.clone();
    let task = tokio::spawn(async move { server::serve(Some(&server_home), 0).await });
    let paths = storage::paths_for_home(Some(&home))?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    for _ in 0..100 {
        if let Ok(record) = lifecycle::read_record(&paths)
            && client
                .get(format!("http://{}/healthz", record.address))
                .send()
                .await
                .is_ok_and(|response| response.status().is_success())
        {
            let token = AuthState::read_bearer_file(&paths.runtime.join("bearer.token"))?;
            return Ok((task, paths, record, token));
        }
        if task.is_finished() {
            return match task.await? {
                Ok(()) => Err("local service exited before readiness".into()),
                Err(error) => Err(format!("local service startup failed: {error}").into()),
            };
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Err("local service did not become ready".into())
}

async fn assert_query_rejected(
    client: &Client,
    base: &str,
    bearer: &str,
    query_id: &str,
    sql: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let response = client
        .post(format!("{base}/api/v1/data/query"))
        .bearer_auth(bearer)
        .json(&json!({
            "query_id": query_id,
            "request": {
                "dataset_id": "hyperai.synthetic_market_observations_json.v1",
                "sql": sql,
                "format": "json",
                "max_rows": null
            }
        }))
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY, "{sql}");
    let body: Value = response.json().await?;
    assert_eq!(body["code"], "DATA_QUERY_REJECTED", "{sql}");
    Ok(())
}

async fn assert_query_limits_rejected(
    client: &Client,
    base: &str,
    bearer: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for (query_id, sql, max_rows) in [
        (
            "sql-byte-limit-query",
            " ".repeat(4_097),
            serde_json::Value::Null,
        ),
        (
            "result-row-limit-query",
            "SELECT gpu_model FROM dataset".to_owned(),
            json!(1_001),
        ),
    ] {
        let response = client
            .post(format!("{base}/api/v1/data/query"))
            .bearer_auth(bearer)
            .json(&json!({
                "query_id": query_id,
                "request": {
                    "dataset_id": "hyperai.synthetic_market_observations_json.v1",
                    "sql": sql,
                    "format": "json",
                    "max_rows": max_rows
                }
            }))
            .send()
            .await?;
        assert_eq!(
            response.status(),
            StatusCode::PAYLOAD_TOO_LARGE,
            "{query_id}"
        );
        let body: Value = response.json().await?;
        assert_eq!(body["code"], "DATA_LIMIT_EXCEEDED", "{query_id}");
        assert!(body.get("rows").is_none(), "{query_id}");
    }
    Ok(())
}

fn write_control_character_snapshot(
    data_root: &std::path::Path,
    payload: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let manifest_path = data_root
        .join("manifests")
        .join("hyperai.synthetic_market_observations_json.v1.json");
    let mut manifest: Value = serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
    let relative_path = manifest["relative_path"]
        .as_str()
        .ok_or("synthetic manifest is missing its relative path")?;
    let mut contents = Vec::new();
    for (instance_id, gpu_model) in [
        ("synthetic-control-01", payload),
        ("synthetic-control-02", "A100"),
        ("synthetic-control-03", "H100"),
    ] {
        contents.extend(serde_json::to_vec(&json!({
            "instance_id": instance_id,
            "gpu_model": gpu_model,
            "region": "test",
            "asking_usd_micros_per_hour": 1,
            "available_count": 1,
            "observed_at": "2026-01-01T00:00:00Z"
        }))?);
        contents.push(b'\n');
    }
    std::fs::write(data_root.join(relative_path), &contents)?;
    manifest["byte_length"] = json!(contents.len());
    manifest["content_digest"] =
        serde_json::to_value(Sha256Digest::from_bytes(Sha256::digest(&contents).into()))?;
    manifest["row_count"] = json!(3);
    std::fs::write(manifest_path, serde_json::to_vec(&manifest)?)?;
    Ok(())
}

fn write_default_pool_stress_snapshot(
    data_root: &std::path::Path,
) -> Result<(usize, String), Box<dyn Error + Send + Sync>> {
    const TARGET_BYTES: usize = 15_990_000;
    const ROW_COUNT: usize = 1_000;
    let manifest_path = data_root
        .join("manifests")
        .join("hyperai.synthetic_market_observations_json.v1.json");
    let mut manifest: Value = serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
    let relative_path = manifest["relative_path"]
        .as_str()
        .ok_or("synthetic manifest is missing its relative path")?;
    let mut contents = Vec::with_capacity(TARGET_BYTES);
    let mut fixed_rows = Vec::with_capacity(ROW_COUNT);
    for row in 0..ROW_COUNT {
        fixed_rows.push(serde_json::to_vec(&json!({
            "instance_id": format!("synthetic-{row:04}-"),
            "gpu_model": "H100",
            "region": "us-east",
            "asking_usd_micros_per_hour": 1,
            "available_count": 1,
            "observed_at": "2026-01-01T00:00:00Z"
        }))?);
    }
    let fixed_bytes = fixed_rows.iter().map(|row| row.len() + 1).sum::<usize>();
    let padding_bytes = (TARGET_BYTES - fixed_bytes) / ROW_COUNT;
    let padding_remainder = (TARGET_BYTES - fixed_bytes) % ROW_COUNT;
    for row in 0..ROW_COUNT {
        let extra_padding = usize::from(row < padding_remainder);
        let padding = "x".repeat(padding_bytes + extra_padding);
        contents.extend_from_slice(&serde_json::to_vec(&json!({
            "instance_id": format!("synthetic-{row:04}-{padding}"),
            "gpu_model": "H100",
            "region": "us-east",
            "asking_usd_micros_per_hour": 1,
            "available_count": 1,
            "observed_at": "2026-01-01T00:00:00Z"
        }))?);
        contents.push(b'\n');
    }
    let byte_length = contents.len();
    let digest = Sha256Digest::from_bytes(Sha256::digest(&contents).into());
    std::fs::write(data_root.join(relative_path), &contents)?;
    manifest["byte_length"] = json!(byte_length);
    manifest["content_digest"] = serde_json::to_value(digest)?;
    manifest["row_count"] = json!(ROW_COUNT);
    std::fs::write(manifest_path, serde_json::to_vec(&manifest)?)?;
    Ok((byte_length, digest.to_hex()))
}

async fn assert_unhandled_select_clauses_rejected(
    client: &Client,
    base: &str,
    bearer: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for (query_id, sql) in [
        (
            "for-json-query",
            "SELECT gpu_model FROM dataset FOR JSON AUTO",
        ),
        ("for-xml-query", "SELECT gpu_model FROM dataset FOR XML RAW"),
        ("format-query", "SELECT gpu_model FROM dataset FORMAT JSON"),
        (
            "empty-distinct-on-query",
            "SELECT DISTINCT ON () gpu_model FROM dataset",
        ),
        (
            "distinct-on-query",
            "SELECT DISTINCT ON (gpu_model) gpu_model FROM dataset ORDER BY gpu_model",
        ),
        (
            "prewhere-query",
            "SELECT gpu_model FROM dataset PREWHERE gpu_model = 'GPU A'",
        ),
        ("top-query", "SELECT TOP (1) gpu_model FROM dataset"),
        (
            "lateral-view-query",
            "SELECT gpu_model FROM dataset LATERAL VIEW explode(array(1)) lv AS x",
        ),
        (
            "cluster-query",
            "SELECT gpu_model FROM dataset CLUSTER BY gpu_model",
        ),
        (
            "sort-query",
            "SELECT gpu_model FROM dataset SORT BY gpu_model",
        ),
        (
            "fetch-query",
            "SELECT gpu_model FROM dataset FETCH FIRST 1 ROWS ONLY",
        ),
    ] {
        assert_query_rejected(client, base, bearer, query_id, sql).await?;
    }
    Ok(())
}

async fn assert_scalar_function_modifiers_rejected(
    client: &Client,
    base: &str,
    bearer: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for (query_id, sql) in [
        (
            "scalar-filter-query",
            "SELECT ABS(asking_usd_micros_per_hour) FILTER (WHERE asking_usd_micros_per_hour < 0) FROM dataset",
        ),
        (
            "scalar-null-treatment-query",
            "SELECT ABS(asking_usd_micros_per_hour) IGNORE NULLS FROM dataset",
        ),
        (
            "scalar-argument-order-query",
            "SELECT ABS(asking_usd_micros_per_hour ORDER BY asking_usd_micros_per_hour) FROM dataset",
        ),
        (
            "scalar-distinct-query",
            "SELECT ABS(DISTINCT asking_usd_micros_per_hour) FROM dataset",
        ),
    ] {
        assert_query_rejected(client, base, bearer, query_id, sql).await?;
    }
    Ok(())
}

async fn assert_function_argument_shapes_rejected(
    client: &Client,
    base: &str,
    bearer: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for (query_id, sql) in [
        ("empty-abs-arguments-query", "SELECT ABS() FROM dataset"),
        ("empty-count-arguments-query", "SELECT COUNT() FROM dataset"),
        ("abs-wildcard-arguments-query", "SELECT ABS(*) FROM dataset"),
        ("sum-wildcard-arguments-query", "SELECT SUM(*) FROM dataset"),
        (
            "abs-qualified-wildcard-query",
            "SELECT ABS(dataset.*) FROM dataset",
        ),
        (
            "count-qualified-wildcard-query",
            "SELECT COUNT(dataset.*) FROM dataset",
        ),
        (
            "abs-named-arguments-query",
            "SELECT ABS(value := gpu_model) FROM dataset",
        ),
        (
            "count-named-arguments-query",
            "SELECT COUNT(value := gpu_model) FROM dataset",
        ),
        (
            "count-distinct-wildcard-query",
            "SELECT COUNT(DISTINCT *) FROM dataset",
        ),
        (
            "count-all-wildcard-query",
            "SELECT COUNT(ALL *) FROM dataset",
        ),
    ] {
        assert_query_rejected(client, base, bearer, query_id, sql).await?;
    }
    Ok(())
}

async fn assert_aggregate_argument_clauses_rejected(
    client: &Client,
    base: &str,
    bearer: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for (query_id, sql) in [
        (
            "aggregate-argument-limit-query",
            "SELECT SUM(available_count LIMIT 1) FROM dataset",
        ),
        (
            "aggregate-argument-null-clause-query",
            "SELECT SUM(available_count IGNORE NULLS) FROM dataset",
        ),
        (
            "aggregate-null-treatment-query",
            "SELECT SUM(available_count) IGNORE NULLS FROM dataset",
        ),
        (
            "aggregate-having-bound-query",
            "SELECT SUM(available_count HAVING MAX available_count) FROM dataset",
        ),
        (
            "aggregate-separator-query",
            "SELECT SUM(available_count SEPARATOR ',') FROM dataset",
        ),
        (
            "aggregate-overflow-query",
            "SELECT SUM(available_count ON OVERFLOW ERROR) FROM dataset",
        ),
        (
            "aggregate-json-null-query",
            "SELECT SUM(available_count ABSENT ON NULL) FROM dataset",
        ),
        (
            "aggregate-json-returning-query",
            "SELECT SUM(available_count RETURNING BIGINT) FROM dataset",
        ),
    ] {
        assert_query_rejected(client, base, bearer, query_id, sql).await?;
    }
    Ok(())
}

async fn assert_unallowlisted_special_expressions_rejected(
    client: &Client,
    base: &str,
    bearer: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for (query_id, sql) in [
        (
            "special-convert-query",
            "SELECT CONVERT(gpu_model USING UTF8) FROM dataset",
        ),
        (
            "special-cast-format-query",
            "SELECT CAST(gpu_model AS VARCHAR FORMAT 'x') FROM dataset",
        ),
        (
            "special-collate-query",
            "SELECT gpu_model COLLATE utf8_bin FROM dataset",
        ),
        (
            "special-is-normalized-query",
            "SELECT gpu_model IS NFC NORMALIZED FROM dataset",
        ),
        (
            "special-member-of-query",
            "SELECT gpu_model MEMBER OF (ARRAY['A100']) FROM dataset",
        ),
        (
            "special-extract-query",
            "SELECT EXTRACT(DAY FROM observed_at) FROM dataset",
        ),
        (
            "special-position-query",
            "SELECT POSITION('A' IN gpu_model) FROM dataset",
        ),
        (
            "special-substring-query",
            "SELECT SUBSTRING(gpu_model FROM 1 FOR 2) FROM dataset",
        ),
        ("special-trim-query", "SELECT TRIM(gpu_model) FROM dataset"),
        (
            "special-overlay-query",
            "SELECT OVERLAY(gpu_model PLACING 'X' FROM 1) FROM dataset",
        ),
        (
            "special-floor-query",
            "SELECT FLOOR(asking_usd_micros_per_hour TO DAY) FROM dataset",
        ),
        (
            "special-ceil-query",
            "SELECT CEIL(asking_usd_micros_per_hour, 2) FROM dataset",
        ),
    ] {
        assert_query_rejected(client, base, bearer, query_id, sql).await?;
    }
    Ok(())
}

async fn assert_json_parquet_queries(
    client: &Client,
    base: &str,
    bearer: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for (dataset_id, format_name) in [
        ("hyperai.synthetic_market_observations_json.v1", "json"),
        (
            "hyperai.synthetic_market_observations_parquet.v1",
            "parquet",
        ),
    ] {
        let result: Value = client
            .post(format!("{base}/api/v1/data/query"))
            .bearer_auth(bearer)
            .json(&json!({
                "query_id": format!("query-{format_name}"),
                "request": {
                    "dataset_id": dataset_id,
                    "sql": "SELECT gpu_model FROM dataset ORDER BY gpu_model",
                    "format": "json",
                    "max_rows": null
                }
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        assert_eq!(result["row_count"], 3);
        assert_eq!(result["rows"].as_array().map(Vec::len), Some(3));
        assert_eq!(
            result["effect_boundary"],
            "LOCAL_READ_ONLY_NO_NETWORK_NO_PROVIDER_NO_FUNDS"
        );
    }
    Ok(())
}

async fn assert_definition_catalogs(
    client: &Client,
    base: &str,
    bearer: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let model_catalog: Value = client
        .get(format!("{base}/api/v1/data/models"))
        .bearer_auth(bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        model_catalog["models"][0]["model"]["model_id"],
        "hourly-costs"
    );
    assert_eq!(model_catalog["models"][0]["input_state"], "current");
    let view_catalog: Value = client
        .get(format!("{base}/api/v1/data/views"))
        .bearer_auth(bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(view_catalog["views"][0]["view"]["view_id"], "cost-chart");
    assert_eq!(view_catalog["views"][0]["model_state"], "linked");
    Ok(())
}

async fn assert_synthetic_release(
    client: &Client,
    base: &str,
    bearer: &str,
    input_digest: &Value,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let release: Value = client
        .get(format!("{base}/api/v1/data/synthetic-release"))
        .bearer_auth(bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let repeated: Value = client
        .get(format!("{base}/api/v1/data/synthetic-release"))
        .bearer_auth(bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(release, repeated);
    assert_eq!(
        release["schema_version"],
        "hyperai.data.synthetic-release.v1"
    );
    assert_eq!(&release["bronze"]["snapshot_digest"], input_digest);
    assert_eq!(release["silver"].as_array().map(Vec::len), Some(3));
    assert_eq!(release["gold"].as_array().map(Vec::len), Some(3));
    let unknown_observation = release["silver"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["gpu_model"] == "MI300X"))
        .ok_or("synthetic unknown observation missing")?;
    assert!(unknown_observation["available_count"].is_null());
    let unknown_summary = release["gold"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["gpu_model"] == "MI300X"))
        .ok_or("synthetic unknown summary missing")?;
    assert_eq!(unknown_summary["unknown_availability_records"], 1);
    assert!(unknown_summary["available_units_total"].is_null());
    Ok(())
}

#[tokio::test]
async fn synthetic_seed_is_preview_bound_and_json_parquet_queries_share_the_api()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(4)).build()?;
    let base = format!("http://{}", record.address);

    let unauthenticated = client
        .get(format!("{base}/api/v1/data/catalog"))
        .send()
        .await?;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let empty: Value = client
        .get(format!("{base}/api/v1/data/catalog"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(empty["status"], "empty");
    assert_eq!(empty["datasets"].as_array().map(Vec::len), Some(0));
    let unauthenticated_release = client
        .get(format!("{base}/api/v1/data/synthetic-release"))
        .send()
        .await?;
    assert_eq!(unauthenticated_release.status(), StatusCode::UNAUTHORIZED);

    let preview: Value = client
        .get(format!("{base}/api/v1/data/synthetic/preview"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let seed_request = json!({ "preview_digest": preview["preview_digest"] });
    let missing_key = client
        .post(format!("{base}/api/v1/data/synthetic/seed"))
        .bearer_auth(&bearer)
        .json(&seed_request)
        .send()
        .await?;
    assert_eq!(missing_key.status(), StatusCode::BAD_REQUEST);

    let first: Value = client
        .post(format!("{base}/api/v1/data/synthetic/seed"))
        .bearer_auth(&bearer)
        .header("idempotency-key", "seed-v1")
        .json(&seed_request)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let replay: Value = client
        .post(format!("{base}/api/v1/data/synthetic/seed"))
        .bearer_auth(&bearer)
        .header("idempotency-key", "seed-v1")
        .json(&seed_request)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(first, replay);

    let catalog: Value = client
        .get(format!("{base}/api/v1/data/catalog"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(catalog["status"], "ready");
    assert_eq!(catalog["datasets"].as_array().map(Vec::len), Some(2));

    assert_synthetic_release(&client, &base, &bearer, &first["content_digest"]).await?;

    assert_json_parquet_queries(&client, &base, &bearer).await?;

    task.abort();
    let _ = task.await;
    Ok(())
}

#[tokio::test]
async fn query_api_rejects_planner_unsupported_sql_before_dataset_access()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, paths, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);
    let data_root = paths.root.join("data");
    assert!(!data_root.exists());
    assert_query_rejected(
        &client,
        &base,
        &bearer,
        "planner-preflight-in-unnest",
        "SELECT gpu_model IN UNNEST(ARRAY['A100']) FROM dataset",
    )
    .await?;
    assert!(!data_root.exists());
    assert_query_rejected(
        &client,
        &base,
        &bearer,
        "select-into-write-intent",
        "SELECT gpu_model INTO TABLE exported FROM dataset",
    )
    .await?;
    assert!(!data_root.exists());

    task.abort();
    let _ = task.await;
    Ok(())
}

#[tokio::test]
async fn query_api_rejects_unregistered_functions_and_cancel_unknown_query()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _paths, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);

    assert_query_rejected(
        &client,
        &base,
        &bearer,
        "typed-table-alias-query",
        "SELECT d.gpu_model FROM dataset AS d (gpu_model INT)",
    )
    .await?;
    assert_query_rejected(
        &client,
        &base,
        &bearer,
        "parametric-function-query",
        "SELECT ABS(999)(gpu_model) FROM dataset",
    )
    .await?;
    assert_query_rejected(
        &client,
        &base,
        &bearer,
        "odbc-unregistered-function-query",
        "SELECT { fn RANDOM() } FROM dataset",
    )
    .await?;
    assert_function_argument_shapes_rejected(&client, &base, &bearer).await?;
    assert_aggregate_argument_clauses_rejected(&client, &base, &bearer).await?;
    assert_scalar_function_modifiers_rejected(&client, &base, &bearer).await?;
    assert_unallowlisted_special_expressions_rejected(&client, &base, &bearer).await?;

    let rejected = client
        .post(format!("{base}/api/v1/data/query"))
        .bearer_auth(&bearer)
        .json(&json!({
            "query_id": "bad-query",
            "request": {
                "dataset_id": "hyperai.synthetic_market_observations_json.v1",
                "sql": "SELECT read_file('/etc/passwd') FROM dataset",
                "format": "json",
                "max_rows": null
            }
        }))
        .send()
        .await?;
    assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let rejected_body: Value = rejected.json().await?;
    assert_eq!(rejected_body["code"], "DATA_QUERY_REJECTED");
    assert!(
        rejected_body["message"]
            .as_str()
            .is_some_and(|message| !message.contains("read_file"))
    );

    let select_into = client
        .post(format!("{base}/api/v1/data/query"))
        .bearer_auth(&bearer)
        .json(&json!({
            "query_id": "select-into-query",
            "request": {
                "dataset_id": "hyperai.synthetic_market_observations_json.v1",
                "sql": "SELECT gpu_model INTO TABLE exported FROM dataset",
                "format": "json",
                "max_rows": null
            }
        }))
        .send()
        .await?;
    assert_eq!(select_into.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let select_into_body: Value = select_into.json().await?;
    assert_eq!(select_into_body["code"], "DATA_QUERY_REJECTED");

    let sampled = client
        .post(format!("{base}/api/v1/data/query"))
        .bearer_auth(&bearer)
        .json(&json!({
            "query_id": "sampled-query",
            "request": {
                "dataset_id": "hyperai.synthetic_market_observations_json.v1",
                "sql": "SELECT gpu_model FROM dataset TABLESAMPLE BERNOULLI (50)",
                "format": "json",
                "max_rows": null
            }
        }))
        .send()
        .await?;
    assert_eq!(sampled.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let sampled_body: Value = sampled.json().await?;
    assert_eq!(sampled_body["code"], "DATA_QUERY_REJECTED");

    assert_unhandled_select_clauses_rejected(&client, &base, &bearer).await?;

    let missing = client
        .post(format!("{base}/api/v1/data/query/unknown/cancel"))
        .bearer_auth(&bearer)
        .header(header::CONTENT_TYPE, "application/json")
        .body("{}")
        .send()
        .await?;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);

    task.abort();
    let _ = task.await;
    Ok(())
}

#[tokio::test]
async fn query_api_returns_typed_limit_errors_without_partial_results()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);

    assert_query_limits_rejected(&client, &base, &bearer).await?;

    task.abort();
    let _ = task.await;
    Ok(())
}

#[tokio::test]
async fn authenticated_thousand_row_aggregate_probes_default_memory_pool()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, paths, record, bearer) = launch(temporary.path().join("home")).await?;
    let data_root = paths.root.join("data");
    hyperai_data::seed_synthetic(&data_root)?;
    let (byte_length, snapshot_digest) = write_default_pool_stress_snapshot(&data_root)?;
    assert_eq!(byte_length, 15_990_000);
    eprintln!("authenticated pool candidate: bytes={byte_length}, digest={snapshot_digest}");

    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
    let base = format!("http://{}", record.address);
    let started = Instant::now();
    let response = client
        .post(format!("{base}/api/v1/data/query"))
        .bearer_auth(&bearer)
        .json(&json!({
            "query_id": "default-pool-api-probe",
            "request": {
                "dataset_id": "hyperai.synthetic_market_observations_json.v1",
                "sql": "SELECT COUNT(*) FROM dataset GROUP BY instance_id",
                "format": "json",
                "max_rows": 1000
            }
        }))
        .send()
        .await?;
    let elapsed = started.elapsed();
    let status = response.status();
    let body = response.bytes().await?;
    eprintln!(
        "authenticated pool candidate response: status={status}, elapsed_ms={}, response_bytes={}",
        elapsed.as_millis(),
        body.len()
    );
    assert!(body.len() < 256 * 1024, "response was {} bytes", body.len());
    if status == StatusCode::OK {
        let result: Value = serde_json::from_slice(&body)?;
        assert_eq!(result["snapshot_digest"], snapshot_digest);
        assert_eq!(result["row_count"], 1000);
        assert_eq!(result["rows"].as_array().map(Vec::len), Some(1000));
        assert!(
            result["rows"]
                .as_array()
                .is_some_and(|rows| { rows.iter().all(|row| row == &json!(["1"])) })
        );
        assert_eq!(result["limits"]["memory_bytes"], 64 * 1024 * 1024);
        assert!(
            result["output_bytes"]
                .as_u64()
                .is_some_and(|bytes| bytes < 256 * 1024)
        );
    } else {
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        assert!(
            elapsed < Duration::from_millis(1_900),
            "HTTP 413 occurred after {elapsed:?}, outside the pool-inference window"
        );
        let error: Value = serde_json::from_slice(&body)?;
        assert_eq!(error["code"], "DATA_LIMIT_EXCEEDED");
        assert!(error.get("rows").is_none());
        assert!(error.get("csv").is_none());
    }

    task.abort();
    let _ = task.await;
    Ok(())
}

#[tokio::test]
async fn csv_query_rejects_an_oversized_complete_json_response()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, paths, record, bearer) = launch(temporary.path().join("home")).await?;
    let data_root = paths.root.join("data");
    hyperai_data::seed_synthetic(&data_root)?;
    let payload = "\u{1}".repeat(50_000);
    write_control_character_snapshot(&data_root, &payload)?;

    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);
    let response = client
        .post(format!("{base}/api/v1/data/query"))
        .bearer_auth(&bearer)
        .json(&json!({
            "query_id": "csv-wire-response-limit",
            "request": {
                "dataset_id": "hyperai.synthetic_market_observations_json.v1",
                "sql": "SELECT gpu_model FROM dataset",
                "format": "csv",
                "max_rows": null
            }
        }))
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let body: Value = response.json().await?;
    assert_eq!(body["code"], "DATA_LIMIT_EXCEEDED");
    assert!(body.get("rows").is_none());
    assert!(body.get("csv").is_none());

    task.abort();
    let _ = task.await;
    Ok(())
}

#[tokio::test]
async fn model_and_view_writes_require_digest_preview_and_idempotency()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);
    let seed_preview: Value = client
        .get(format!("{base}/api/v1/data/synthetic/preview"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let seed: Value = client
        .post(format!("{base}/api/v1/data/synthetic/seed"))
        .bearer_auth(&bearer)
        .header("idempotency-key", "seed-model-test")
        .json(&json!({ "preview_digest": seed_preview["preview_digest"] }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let model = json!({
        "schema_version": "hyperai.data.model.v1",
        "model_id": "hourly-costs",
        "dataset_id": "hyperai.synthetic_market_observations_json.v1",
        "snapshot_digest": seed["content_digest"],
        "sql": "SELECT gpu_model, asking_usd_micros_per_hour FROM dataset"
    });
    let model_preview: Value = client
        .post(format!("{base}/api/v1/data/models/preview"))
        .bearer_auth(&bearer)
        .json(&model)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let model_request =
        json!({ "model": model, "preview_digest": model_preview["preview_digest"] });
    let saved_model: Value = client
        .post(format!("{base}/api/v1/data/models/save"))
        .bearer_auth(&bearer)
        .header("idempotency-key", "save-model-v1")
        .json(&model_request)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let model_replay: Value = client
        .post(format!("{base}/api/v1/data/models/save"))
        .bearer_auth(&bearer)
        .header("idempotency-key", "save-model-v1")
        .json(&model_request)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(saved_model, model_replay);

    let view = json!({
        "schema_version": "hyperai.data.view.v1",
        "view_id": "cost-chart",
        "model_digest": saved_model["definition_digest"],
        "presentation": "bar",
        "columns": ["gpu_model", "asking_usd_micros_per_hour"]
    });
    let view_preview: Value = client
        .post(format!("{base}/api/v1/data/views/preview"))
        .bearer_auth(&bearer)
        .json(&view)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let saved_view: Value = client
        .post(format!("{base}/api/v1/data/views/save"))
        .bearer_auth(&bearer)
        .header("idempotency-key", "save-view-v1")
        .json(&json!({ "view": view, "preview_digest": view_preview["preview_digest"] }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(saved_view["definition_id"], "cost-chart");

    assert_definition_catalogs(&client, &base, &bearer).await?;

    task.abort();
    let _ = task.await;
    Ok(())
}

#[tokio::test]
async fn gpudiff_schedule_status_requires_owner_and_defaults_stopped()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _paths, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let route = format!("http://{}/api/v1/data/gpudiff/schedule", record.address);

    let unauthorized = client.get(&route).send().await?;
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let status: Value = client
        .get(&route)
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        status["schema_version"],
        "hyperai.data.gpudiff_refresh_schedule.v1"
    );
    assert_eq!(
        status["state_slice"],
        "hyperai.bazaar.gpudiff_owner_scheduled_refresh.v1"
    );
    assert_eq!(status["state"], "stopped");
    assert_eq!(status["interval_seconds"], 6 * 60 * 60);

    let unauthorized_start = client
        .post(&route)
        .header(header::CONTENT_TYPE, "application/json")
        .body("{}")
        .send()
        .await?;
    assert_eq!(unauthorized_start.status(), StatusCode::UNAUTHORIZED);
    let unauthorized_stop = client
        .post(format!(
            "http://{}/api/v1/data/gpudiff/schedule/stop",
            record.address
        ))
        .header(header::CONTENT_TYPE, "application/json")
        .body("{}")
        .send()
        .await?;
    assert_eq!(unauthorized_stop.status(), StatusCode::UNAUTHORIZED);

    task.abort();
    let _ = task.await;
    Ok(())
}

#[tokio::test]
#[ignore = "performs one bounded live HTTPS GET by starting the owner-controlled GPUDiff schedule"]
#[allow(clippy::too_many_lines)]
async fn gpudiff_owner_schedule_imports_and_queries_the_public_snapshot()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, paths, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let base = format!("http://{}", record.address);
    let route = format!("{base}/api/v1/data/gpudiff/schedule");

    let unauthorized = client.get(&route).send().await?;
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let start_response = client
        .post(&route)
        .bearer_auth(&bearer)
        .header(header::CONTENT_TYPE, "application/json")
        .body("{}")
        .send()
        .await?;
    let refresh_status = start_response.status();
    let schedule_status: Value = start_response.error_for_status()?.json().await?;
    assert_eq!(schedule_status["state"], "active");
    assert!(
        schedule_status["last_row_count"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
    let duplicate_start: Value = client
        .post(&route)
        .bearer_auth(&bearer)
        .header(header::CONTENT_TYPE, "application/json")
        .body("{}")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(duplicate_start["state"], "active");
    assert_eq!(
        duplicate_start["last_attempt_unix_seconds"],
        schedule_status["last_attempt_unix_seconds"]
    );

    let raw_digest = schedule_status["last_raw_source_digest"]
        .as_str()
        .ok_or("missing raw source digest")?;
    let raw_path = paths
        .root
        .join("data/sources/gpudiff")
        .join(format!("{raw_digest}.json"));
    let raw_bytes = std::fs::read(raw_path)?;
    assert_eq!(
        hyperai_foundation::Sha256Digest::from_bytes(Sha256::digest(&raw_bytes).into()).to_hex(),
        raw_digest
    );

    let catalog: Value = client
        .get(format!("{base}/api/v1/data/catalog"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let dataset = catalog["datasets"]
        .as_array()
        .and_then(|datasets| {
            datasets
                .iter()
                .find(|dataset| dataset["dataset_id"] == "hyperai.gpudiff_public_gpu_offers.v1")
        })
        .ok_or("GPUDiff dataset missing from authenticated catalog")?;
    assert_eq!(dataset["row_count"], schedule_status["last_row_count"]);
    let normalized_digest = dataset["snapshot_digest"]
        .as_str()
        .ok_or("missing normalized snapshot digest")?;
    let imported_rows = schedule_status["last_row_count"]
        .as_u64()
        .ok_or("missing imported row count")?;
    let observed_at_min = schedule_status["last_observed_at_min"]
        .as_str()
        .ok_or("missing earliest source timestamp")?;
    let observed_at_max = schedule_status["last_observed_at_max"]
        .as_str()
        .ok_or("missing latest source timestamp")?;

    let query_response = client
        .post(format!("{base}/api/v1/data/query"))
        .bearer_auth(&bearer)
        .json(&json!({
            "query_id": "gpudiff-live-e2e-query",
            "request": {
                "dataset_id": "hyperai.gpudiff_public_gpu_offers.v1",
                "sql": "SELECT provider, gpu_model, price_decimal, unit, observed_at, provenance_url FROM dataset LIMIT 5",
                "format": "json",
                "max_rows": 5
            }
        }))
        .send()
        .await?;
    let query_status = query_response.status();
    let query: Value = query_response.error_for_status()?.json().await?;
    assert_eq!(query["snapshot_digest"], normalized_digest);
    assert_eq!(query["row_count"].as_u64(), Some(5));
    assert_eq!(query["rows"].as_array().map(Vec::len), Some(5));
    assert!(
        query["output_bytes"]
            .as_u64()
            .is_some_and(|bytes| bytes <= 256 * 1024)
    );
    let queried_rows = query["row_count"]
        .as_u64()
        .ok_or("missing bounded query row count")?;

    let stop: Value = client
        .post(format!("{base}/api/v1/data/gpudiff/schedule/stop"))
        .bearer_auth(&bearer)
        .header(header::CONTENT_TYPE, "application/json")
        .body("{}")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(stop["state"], "stopped");
    assert_eq!(stop["last_raw_source_digest"], raw_digest);
    eprintln!(
        "GPUDiff scheduled live E2E: start_status={refresh_status} raw_sha256={raw_digest} normalized_sha256={normalized_digest} rows={imported_rows} observed_at={observed_at_min}..{observed_at_max} query_status={query_status} query_rows={queried_rows} stop_state={}",
        stop["state"]
    );

    task.abort();
    let _ = task.await;
    Ok(())
}
