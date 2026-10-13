//! Read-only SQL policy, formats, and resource-limit checks.

use std::error::Error;

use hyperai_data::{DataError, QueryFormat, QueryRequestV1, catalog, query, seed_synthetic};
use hyperai_foundation::Sha256Digest;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlparser::{
    ast::{SetExpr, Statement},
    dialect::GenericDialect,
    parser::Parser,
};

fn request(sql: &str, format: QueryFormat) -> QueryRequestV1 {
    QueryRequestV1 {
        dataset_id: "hyperai.synthetic_market_observations_json.v1".to_owned(),
        sql: sql.to_owned(),
        format,
        max_rows: None,
    }
}

fn seeded_root() -> Result<(tempfile::TempDir, std::path::PathBuf), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("data");
    seed_synthetic(&root)?;
    Ok((temporary, root))
}

fn write_control_character_snapshot(
    root: &std::path::Path,
    payload: &str,
) -> Result<(), Box<dyn Error>> {
    let manifest_path = root
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
    std::fs::write(root.join(relative_path), &contents)?;
    manifest["byte_length"] = json!(contents.len());
    manifest["content_digest"] =
        serde_json::to_value(Sha256Digest::from_bytes(Sha256::digest(&contents).into()))?;
    manifest["row_count"] = json!(3);
    std::fs::write(manifest_path, serde_json::to_vec(&manifest)?)?;
    Ok(())
}

#[tokio::test]
async fn json_and_csv_queries_are_stable_and_keep_snapshot_identity() -> Result<(), Box<dyn Error>>
{
    let (_temporary, root) = seeded_root()?;
    let sql = "SELECT gpu_model, available_count FROM dataset ORDER BY gpu_model";
    let json_result = query(&root, &request(sql, QueryFormat::Json)).await?;
    let repeat = query(&root, &request(sql, QueryFormat::Json)).await?;
    let csv_result = query(&root, &request(sql, QueryFormat::Csv)).await?;
    let catalog_result = catalog(&root)?;

    assert_eq!(json_result, repeat);
    assert_eq!(json_result.row_count, 3);
    assert_eq!(
        json_result.snapshot_digest,
        catalog_result.datasets[0].snapshot_digest
    );
    assert!(json_result.rows.iter().flatten().any(Option::is_none));
    assert!(
        csv_result
            .csv
            .as_deref()
            .is_some_and(|csv| csv.starts_with("gpu_model,available_count\n"))
    );
    assert!(
        csv_result
            .csv
            .as_deref()
            .is_some_and(|csv| csv.lines().count() == 4)
    );
    assert!(csv_result.output_bytes <= 256 * 1024);
    Ok(())
}

#[tokio::test]
async fn csv_query_rejects_when_the_complete_json_response_exceeds_its_limit()
-> Result<(), Box<dyn Error>> {
    let (_temporary, root) = seeded_root()?;
    let payload = "\u{1}".repeat(50_000);
    write_control_character_snapshot(&root, &payload)?;

    let selected_csv_bytes = "gpu_model\n".len() + payload.len() + 1 + "A100\nH100\n".len();
    assert!(selected_csv_bytes < 256 * 1024);
    assert!(matches!(
        query(
            &root,
            &request("SELECT gpu_model FROM dataset", QueryFormat::Csv)
        )
        .await,
        Err(DataError::LimitExceeded)
    ));
    Ok(())
}

#[tokio::test]
async fn distinct_on_is_rejected_before_dataset_lookup_and_distinct_still_works()
-> Result<(), Box<dyn Error>> {
    let (_temporary, root) = seeded_root()?;
    let ordinary = query(
        &root,
        &request(
            "SELECT DISTINCT gpu_model FROM dataset ORDER BY gpu_model",
            QueryFormat::Json,
        ),
    )
    .await?;
    assert_eq!(ordinary.row_count, 3);
    assert_eq!(ordinary.columns, ["gpu_model"]);

    let empty_home = tempfile::tempdir()?;
    let missing_data_root = empty_home.path().join("unseeded-data");
    for sql in [
        "SELECT DISTINCT ON () gpu_model FROM dataset",
        "SELECT DISTINCT ON (gpu_model) gpu_model FROM dataset ORDER BY gpu_model",
    ] {
        assert!(matches!(
            query(&missing_data_root, &request(sql, QueryFormat::Json)).await,
            Err(DataError::QueryRejected)
        ));
    }
    assert!(!missing_data_root.exists());
    Ok(())
}

#[tokio::test]
async fn query_row_and_sql_byte_limits_are_enforced() -> Result<(), Box<dyn Error>> {
    let (_temporary, root) = seeded_root()?;
    let mut limited = request("SELECT gpu_model FROM dataset", QueryFormat::Json);
    limited.max_rows = Some(2);
    assert!(matches!(
        query(&root, &limited).await,
        Err(DataError::LimitExceeded)
    ));

    let oversized = request(&" ".repeat(4_097), QueryFormat::Json);
    assert!(matches!(
        query(&root, &oversized).await,
        Err(DataError::LimitExceeded)
    ));
    Ok(())
}

#[tokio::test]
async fn source_file_byte_ceiling_is_enforced_before_opening_the_scan() -> Result<(), Box<dyn Error>>
{
    let (_temporary, root) = seeded_root()?;
    let manifest_path = root
        .join("manifests")
        .join("hyperai.synthetic_market_observations_json.v1.json");
    let mut manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
    manifest["byte_length"] = serde_json::json!(16 * 1024 * 1024 + 1);
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest)?)?;

    assert!(matches!(
        query(
            &root,
            &request("SELECT gpu_model FROM dataset", QueryFormat::Json)
        )
        .await,
        Err(DataError::LimitExceeded)
    ));
    Ok(())
}

#[tokio::test]
async fn non_read_only_or_unregistered_sql_is_rejected() -> Result<(), Box<dyn Error>> {
    let (_temporary, root) = seeded_root()?;
    let select_into = "SELECT gpu_model INTO TABLE exported FROM dataset";
    let statements = Parser::parse_sql(&GenericDialect {}, select_into)?;
    assert!(matches!(
        statements.as_slice(),
        [Statement::Query(query)]
            if matches!(query.body.as_ref(), SetExpr::Select(select) if select.into.is_some())
    ));
    for sql in [
        "DELETE FROM dataset",
        select_into,
        "CREATE EXTERNAL TABLE x STORED AS PARQUET LOCATION 'https://example.invalid/x'",
        "SELECT * FROM read_parquet('/tmp/untrusted.parquet')",
        "SELECT gpu_model FROM dataset TABLESAMPLE BERNOULLI (50)",
        "SELECT gpu_model FROM dataset FOR JSON AUTO",
        "SELECT gpu_model FROM dataset FOR XML RAW",
        "SELECT gpu_model FROM dataset FORMAT JSON",
        "SELECT gpu_model FROM dataset FETCH FIRST 1 ROWS ONLY",
        "SELECT gpu_model FROM dataset PREWHERE gpu_model = 'GPU A'",
        "SELECT TOP (1) gpu_model FROM dataset",
        "SELECT gpu_model FROM dataset LATERAL VIEW explode(array(1)) lv AS x",
        "SELECT gpu_model FROM dataset CLUSTER BY gpu_model",
        "SELECT gpu_model FROM dataset SORT BY gpu_model",
        "SELECT * FROM missing_table",
        "SELECT pg_sleep(30) FROM dataset",
        "SELECT * FROM dataset; DROP TABLE dataset",
        "SELECT COUNT(*) FROM dataset a CROSS JOIN dataset b",
        "WITH RECURSIVE t(n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM t) SELECT n FROM t",
        "SELECT * FROM UNNEST([1,2,3])",
    ] {
        let result = query(&root, &request(sql, QueryFormat::Json)).await;
        assert!(
            matches!(result, Err(DataError::QueryRejected)),
            "accepted query: {sql}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn select_into_is_rejected_before_dataset_access() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let missing_data_root = temporary.path().join("unseeded-data");

    assert!(matches!(
        query(
            &missing_data_root,
            &request(
                "SELECT gpu_model INTO TABLE exported FROM dataset",
                QueryFormat::Json
            )
        )
        .await,
        Err(DataError::QueryRejected)
    ));
    assert!(!missing_data_root.exists());
    Ok(())
}

#[tokio::test]
async fn unallowlisted_special_expressions_are_rejected_before_dataset_access()
-> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let missing_data_root = temporary.path().join("unseeded-data");

    for sql in [
        "SELECT CONVERT(gpu_model USING UTF8) FROM dataset",
        "SELECT CAST(gpu_model AS VARCHAR FORMAT 'x') FROM dataset",
        "SELECT gpu_model COLLATE utf8_bin FROM dataset",
        "SELECT gpu_model IS NFC NORMALIZED FROM dataset",
        "SELECT gpu_model MEMBER OF (ARRAY['A100']) FROM dataset",
        "SELECT EXTRACT(DAY FROM observed_at) FROM dataset",
        "SELECT POSITION('A' IN gpu_model) FROM dataset",
        "SELECT SUBSTRING(gpu_model FROM 1 FOR 2) FROM dataset",
        "SELECT TRIM(gpu_model) FROM dataset",
        "SELECT OVERLAY(gpu_model PLACING 'X' FROM 1) FROM dataset",
        "SELECT FLOOR(asking_usd_micros_per_hour TO DAY) FROM dataset",
        "SELECT CEIL(asking_usd_micros_per_hour, 2) FROM dataset",
    ] {
        assert!(
            matches!(
                query(&missing_data_root, &request(sql, QueryFormat::Json)).await,
                Err(DataError::QueryRejected)
            ),
            "expected pre-planning rejection for {sql}"
        );
        assert!(
            !missing_data_root.exists(),
            "query created the data root for {sql}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn planner_unsupported_in_unnest_is_rejected_before_dataset_access()
-> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let missing_data_root = temporary.path().join("unseeded-data");
    let sql = "SELECT gpu_model IN UNNEST(ARRAY['A100']) FROM dataset";

    let result = query(&missing_data_root, &request(sql, QueryFormat::Json)).await;

    assert!(matches!(result, Err(DataError::QueryRejected)));
    assert!(!missing_data_root.exists());
    Ok(())
}

#[tokio::test]
async fn unsupported_function_argument_shapes_are_rejected_and_count_star_works()
-> Result<(), Box<dyn Error>> {
    let (_temporary, root) = seeded_root()?;
    for sql in [
        "SELECT COUNT(*) FROM dataset",
        "SELECT COUNT(DISTINCT gpu_model) FROM dataset",
    ] {
        let result = query(&root, &request(sql, QueryFormat::Json)).await?;
        assert_eq!(result.row_count, 1, "{sql}");
        assert_eq!(result.rows, vec![vec![Some("3".to_owned())]], "{sql}");
    }

    for (sql, expected_rows) in [
        ("SELECT ABS(-2) FROM dataset", 3),
        ("SELECT COALESCE(NULL, 'fallback') FROM dataset", 3),
        ("SELECT ROUND(1.5) FROM dataset", 3),
        ("SELECT AVG(asking_usd_micros_per_hour) FROM dataset", 1),
        ("SELECT COUNT(gpu_model) FROM dataset", 1),
        ("SELECT MAX(asking_usd_micros_per_hour) FROM dataset", 1),
        ("SELECT MIN(asking_usd_micros_per_hour) FROM dataset", 1),
        ("SELECT SUM(asking_usd_micros_per_hour) FROM dataset", 1),
    ] {
        let result = query(&root, &request(sql, QueryFormat::Json)).await?;
        assert_eq!(result.row_count, expected_rows, "{sql}");
        assert_eq!(result.rows.len(), expected_rows, "{sql}");
        assert_eq!(result.rows[0].len(), 1, "{sql}");
    }

    let empty_home = tempfile::tempdir()?;
    let missing_data_root = empty_home.path().join("unseeded-data");
    for sql in [
        "SELECT ABS() FROM dataset",
        "SELECT COUNT() FROM dataset",
        "SELECT ABS(*) FROM dataset",
        "SELECT SUM(*) FROM dataset",
        "SELECT ABS(dataset.*) FROM dataset",
        "SELECT COUNT(dataset.*) FROM dataset",
        "SELECT ABS(value := gpu_model) FROM dataset",
        "SELECT COUNT(value := gpu_model) FROM dataset",
        "SELECT COUNT(DISTINCT *) FROM dataset",
        "SELECT COUNT(ALL *) FROM dataset",
    ] {
        assert!(
            matches!(
                query(&missing_data_root, &request(sql, QueryFormat::Json)).await,
                Err(DataError::QueryRejected)
            ),
            "expected pre-planning rejection for {sql}"
        );
        assert!(
            !missing_data_root.exists(),
            "query created the data root for {sql}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn odbc_function_syntax_keeps_allowlisted_execution_and_rejects_other_names()
-> Result<(), Box<dyn Error>> {
    let (_temporary, root) = seeded_root()?;
    let ordinary = query(
        &root,
        &request("SELECT ABS(-2) FROM dataset", QueryFormat::Json),
    )
    .await?;
    let escaped = query(
        &root,
        &request("SELECT { fn ABS(-2) } FROM dataset", QueryFormat::Json),
    )
    .await?;
    assert_eq!(escaped.row_count, ordinary.row_count);
    assert_eq!(escaped.rows, ordinary.rows);

    let empty_home = tempfile::tempdir()?;
    let absent_root = empty_home.path().join("unseeded-data");
    let unallowlisted = "SELECT { fn RANDOM() } FROM dataset";
    assert!(matches!(
        query(&absent_root, &request(unallowlisted, QueryFormat::Json)).await,
        Err(DataError::QueryRejected)
    ));
    assert!(!absent_root.exists());
    Ok(())
}

#[tokio::test]
async fn unsupported_aggregate_argument_clauses_are_rejected_before_data_access()
-> Result<(), Box<dyn Error>> {
    let (_temporary, root) = seeded_root()?;
    for sql in [
        "SELECT SUM(available_count) FILTER (WHERE available_count > 0) FROM dataset",
        "SELECT SUM(available_count ORDER BY gpu_model) FROM dataset",
    ] {
        let result = query(&root, &request(sql, QueryFormat::Json)).await?;
        assert_eq!(result.row_count, 1, "{sql}");
        assert_eq!(result.rows, vec![vec![Some("10".to_owned())]], "{sql}");
    }

    let temporary = tempfile::tempdir()?;
    let missing_data_root = temporary.path().join("unseeded-data");
    for sql in [
        "SELECT SUM(available_count LIMIT 1) FROM dataset",
        "SELECT SUM(available_count IGNORE NULLS) FROM dataset",
        "SELECT SUM(available_count) IGNORE NULLS FROM dataset",
        "SELECT SUM(available_count HAVING MAX available_count) FROM dataset",
        "SELECT SUM(available_count SEPARATOR ',') FROM dataset",
        "SELECT SUM(available_count ON OVERFLOW ERROR) FROM dataset",
        "SELECT SUM(available_count ABSENT ON NULL) FROM dataset",
        "SELECT SUM(available_count RETURNING BIGINT) FROM dataset",
    ] {
        assert!(
            matches!(
                query(&missing_data_root, &request(sql, QueryFormat::Json)).await,
                Err(DataError::QueryRejected)
            ),
            "expected pre-planning rejection for {sql}"
        );
        assert!(
            !missing_data_root.exists(),
            "query created the data root for {sql}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn stale_snapshot_is_rejected_before_query_results_escape() -> Result<(), Box<dyn Error>> {
    let (_temporary, root) = seeded_root()?;
    let result = query(
        &root,
        &request("SELECT gpu_model FROM dataset", QueryFormat::Json),
    )
    .await?;
    let source = root.join("synthetic-market-observations.json");
    std::fs::write(&source, b"{}\n")?;

    assert!(matches!(
        query(
            &root,
            &request("SELECT gpu_model FROM dataset", QueryFormat::Json)
        )
        .await,
        Err(DataError::IntegrityMismatch)
    ));
    assert_eq!(result.row_count, 3);
    Ok(())
}

#[tokio::test]
async fn parquet_snapshot_uses_the_same_bounded_read_path() -> Result<(), Box<dyn Error>> {
    let (_temporary, root) = seeded_root()?;
    let parquet_request = QueryRequestV1 {
        dataset_id: "hyperai.synthetic_market_observations_parquet.v1".to_owned(),
        sql: "SELECT gpu_model, asking_usd_micros_per_hour FROM dataset ORDER BY gpu_model"
            .to_owned(),
        format: QueryFormat::Json,
        max_rows: None,
    };
    let result = query(&root, &parquet_request).await?;

    assert_eq!(result.row_count, 3);
    assert_eq!(result.columns, ["gpu_model", "asking_usd_micros_per_hour"]);
    Ok(())
}

#[tokio::test]
async fn already_canceled_query_returns_no_result() -> Result<(), Box<dyn Error>> {
    let (_temporary, root) = seeded_root()?;
    let request = request("SELECT gpu_model FROM dataset", QueryFormat::Json);
    let (sender, receiver) = tokio::sync::watch::channel(true);
    let result = hyperai_data::query_with_cancellation(&root, &request, receiver).await;
    drop(sender);

    assert!(matches!(result, Err(DataError::Cancelled)));
    Ok(())
}
