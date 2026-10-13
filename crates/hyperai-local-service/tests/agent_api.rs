//! Local ACP session routes remain owner-authenticated and validate profiles before launch.

use std::{error::Error, fs, path::PathBuf, time::Duration};

use hyperai_local_service::{
    AuthState,
    lifecycle::{self, ServiceRecordV1},
    server,
    storage::{self, HomePaths},
};
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
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
    let client = Client::builder().timeout(Duration::from_secs(2)).build()?;
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

fn assert_workload_trace_toolset(status: &Value) {
    assert_eq!(
        status["tool_set_version"],
        "hyperai.agent-tools.readonly.v6"
    );
    assert_eq!(status["tools"], json!(hyperai_acp::MCP_TOOL_NAMES_V6));
}

fn assert_inconclusive_trace_contract(trace: &Value) -> Result<(), std::io::Error> {
    assert_eq!(
        trace["schema_version"],
        "hyperai.interface.workload-trace.v1"
    );
    assert_eq!(
        trace["state_slice"],
        "hyperai.bazaar.workload_trace_inconclusive_served.v1"
    );
    assert_eq!(
        trace["claim_ceiling"],
        "digest-bound local synthetic inconclusive mock-outcome preview"
    );
    let references = trace["references"]
        .as_array()
        .ok_or_else(|| std::io::Error::other("inconclusive trace has no reference list"))?;
    assert!(references.iter().any(|reference| {
        reference["record_id"] == "synthetic-mock-verifier-decision-v1"
            && reference["status"] == "mock_verdict_inconclusive"
            && reference["claim_ceiling"]
                == "local_fixture_verdict_only_not_independent_correctness_evidence"
    }));
    assert!(references.iter().any(|reference| {
        reference["record_id"] == "synthetic-mock-disposition-v1"
            && reference["status"] == "mock_disposition_hold_for_dispute"
            && reference["digest"].is_null()
            && reference["claim_ceiling"] == "no_payment_or_entitlement_claim"
    }));
    assert!(references.iter().any(|reference| {
        reference["record_id"] == "synthetic-local-market-receipt-v1"
            && reference["digest"]
                .as_str()
                .is_some_and(|digest| digest.len() == 64)
    }));
    Ok(())
}

async fn get_workload_trace(
    client: &Client,
    route: &str,
    bearer: &str,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    Ok(client
        .get(route)
        .bearer_auth(bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

fn workload_trace_data_reference(trace: &Value) -> Option<&Value> {
    trace
        .get("references")?
        .as_array()?
        .iter()
        .find(|reference| reference["owner_id"] == "hyperai-data")
}

#[tokio::test]
async fn serves_the_workload_trace_renderer_at_the_interface_asset_path()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);

    let response = client
        .get(format!("{base}/workload-trace.js"))
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[reqwest::header::CONTENT_TYPE],
        "text/javascript; charset=utf-8"
    );
    assert_eq!(
        response.text().await?,
        include_str!("../../../apps/hyperai-interface/workload-trace.js")
    );

    client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;
    Ok(())
}

#[tokio::test]
async fn workload_trace_keeps_unseeded_data_unbound() -> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, paths, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);
    let route = format!("{base}/api/v1/market/workload-trace");

    assert_eq!(
        client.get(&route).send().await?.status(),
        StatusCode::UNAUTHORIZED
    );

    let trace = get_workload_trace(&client, &route, &bearer).await?;

    assert_eq!(
        trace["schema_version"],
        "hyperai.interface.workload-trace.v1"
    );
    assert_eq!(trace["intent"]["body"]["data_references"], json!([]));
    let data_reference =
        workload_trace_data_reference(&trace).ok_or("trace omitted the Data context reference")?;
    assert_eq!(data_reference["status"], "not_seeded");
    assert!(data_reference["digest"].is_null());
    assert_eq!(
        data_reference["link_basis"],
        "local_catalog_does_not_contain_requested_dataset"
    );

    let data_root = paths.root.join("data");
    hyperai_data::seed_synthetic(&data_root)?;
    let bound = get_workload_trace(&client, &route, &bearer).await?;
    let bound_reference =
        workload_trace_data_reference(&bound).ok_or("bound trace omitted the Data reference")?;
    let bound_digest = bound["intent"]["body"]["data_references"][0]["digest"]
        .as_str()
        .ok_or("bound intent omitted the verified Data digest")?;
    assert_eq!(bound_reference["status"], "bound");
    assert_eq!(bound_reference["digest"], bound_digest);
    assert_eq!(
        bound_reference["link_basis"],
        "verified_local_manifest_and_snapshot_content_digest"
    );

    client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;
    Ok(())
}

#[tokio::test]
async fn workload_trace_surfaces_local_source_failures_without_digests()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, paths, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);
    let route = format!("{base}/api/v1/market/workload-trace");
    let data_root = paths.root.join("data");
    let sample = data_root.join("synthetic-market-observations.json");
    hyperai_data::seed_synthetic(&data_root)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&sample)?.permissions();
        permissions.set_mode(0o0);
        fs::set_permissions(&sample, permissions)?;
        let denied_or_bound = get_workload_trace(&client, &route, &bearer).await?;
        let denied_reference = workload_trace_data_reference(&denied_or_bound)
            .ok_or("permission-limited trace omitted the Data reference")?;
        let read_result = fs::read(&sample);
        let mut permissions = fs::metadata(&sample)?.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(&sample, permissions)?;
        if read_result
            .as_ref()
            .is_err_and(|error| error.kind() == std::io::ErrorKind::PermissionDenied)
        {
            assert_eq!(denied_reference["status"], "denied");
            assert!(denied_reference["digest"].is_null());
        } else {
            assert_eq!(denied_reference["status"], "bound");
        }
    }

    fs::remove_file(&sample)?;
    let missing = get_workload_trace(&client, &route, &bearer).await?;
    let missing_reference = workload_trace_data_reference(&missing)
        .ok_or("missing trace omitted the Data reference")?;
    assert_eq!(missing_reference["status"], "missing");
    assert!(missing_reference["digest"].is_null());

    hyperai_data::seed_synthetic(&data_root)?;
    fs::write(&sample, b"changed local synthetic bytes\n")?;
    let conflicting = get_workload_trace(&client, &route, &bearer).await?;
    let conflict_reference = workload_trace_data_reference(&conflicting)
        .ok_or("conflicting trace omitted the Data reference")?;
    assert_eq!(conflict_reference["status"], "conflicting");
    assert!(conflict_reference["digest"].is_null());

    client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;
    Ok(())
}

#[tokio::test]
async fn seller_failure_preview_is_owner_only_and_separate_from_success_trace()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);

    let unauthenticated = client
        .get(format!(
            "{base}/api/v1/market/workload-trace/failure-preview"
        ))
        .send()
        .await?;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let success: Value = client
        .get(format!("{base}/api/v1/market/workload-trace"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        success["state_slice"],
        "hyperai.bazaar.workload_trace_synthetic.v1"
    );

    let failure: Value = client
        .get(format!(
            "{base}/api/v1/market/workload-trace/failure-preview"
        ))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        failure["schema_version"],
        "hyperai.interface.workload-trace.v1"
    );
    assert_eq!(
        failure["state_slice"],
        "hyperai.bazaar.workload_trace_seller_failure.v1"
    );
    assert_eq!(
        failure["claim_ceiling"],
        "digest-bound local synthetic seller-attributed failure projection"
    );
    let references = failure["references"]
        .as_array()
        .ok_or("failure trace has no reference list")?;
    assert!(references.iter().any(|reference| {
        reference["schema_version"] == "hyperai.compute-market.fulfillment-failure.v1"
            && reference["status"] == "fulfillment_failed"
            && reference["digest"].as_str().is_some()
    }));
    assert!(
        references
            .iter()
            .any(|reference| { reference["record_id"] == "synthetic-approved-deal-v1" })
    );
    for completion_only_record in [
        "synthetic-fulfillment-v1",
        "synthetic-mock-verifier-decision-v1",
        "synthetic-mock-disposition-v1",
        "synthetic-local-market-receipt-v1",
    ] {
        assert!(
            !references
                .iter()
                .any(|reference| reference["record_id"] == completion_only_record),
            "failure trace includes completion-only record {completion_only_record}"
        );
    }

    client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;
    Ok(())
}

#[tokio::test]
async fn inconclusive_preview_is_owner_only_and_preserves_other_trace_projections()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);

    let unauthenticated = client
        .get(format!(
            "{base}/api/v1/market/workload-trace/inconclusive-preview"
        ))
        .send()
        .await?;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let success_before: Value = client
        .get(format!("{base}/api/v1/market/workload-trace"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        success_before["state_slice"],
        "hyperai.bazaar.workload_trace_synthetic.v1"
    );

    let seller_failure: Value = client
        .get(format!(
            "{base}/api/v1/market/workload-trace/failure-preview"
        ))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        seller_failure["state_slice"],
        "hyperai.bazaar.workload_trace_seller_failure.v1"
    );

    let inconclusive: Value = client
        .get(format!(
            "{base}/api/v1/market/workload-trace/inconclusive-preview"
        ))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_inconclusive_trace_contract(&inconclusive)?;

    let scoped_preview: Value = client
        .get(format!(
            "{base}/api/v1/agent/tools/market/workload-trace/inconclusive-preview"
        ))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(scoped_preview, inconclusive);

    let success_after: Value = client
        .get(format!("{base}/api/v1/market/workload-trace"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(success_after, success_before);

    client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;
    Ok(())
}

#[tokio::test]
async fn data_status_previews_are_owner_only_bounded_and_source_free()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);
    let route = format!("{base}/api/v1/market/workload-trace/data-status-preview");

    let unauthenticated = client.get(format!("{route}/missing")).send().await?;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    for status in ["missing", "conflicting", "denied"] {
        let trace: Value = client
            .get(format!("{route}/{status}"))
            .bearer_auth(&bearer)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        assert_eq!(
            trace["state_slice"],
            "hyperai.bazaar.workload_trace_data_status_served.v1"
        );
        assert_eq!(
            trace["claim_ceiling"],
            "digest-bound local synthetic unresolved Data status preview"
        );
        assert!(
            trace["nonclaims"]
                .as_str()
                .is_some_and(|value| value.contains("no Data source was read"))
        );
        let data_reference = trace["references"]
            .as_array()
            .and_then(|references| {
                references
                    .iter()
                    .find(|reference| reference["owner_id"] == "hyperai-data")
            })
            .ok_or("Data status preview omitted its source reference")?;
        assert_eq!(data_reference["status"], status);
        assert_eq!(data_reference["digest"], Value::Null);
        assert_eq!(
            data_reference["link_basis"],
            "fixed_synthetic_status_fixture_no_data_source_read"
        );
        assert_eq!(
            data_reference["claim_ceiling"],
            "synthetic_data_status_fixture_only_no_source_read_conflict_or_access_decision"
        );
        let expected = trace["intent"]["body"]["data_references"][0]["digest"]
            .as_str()
            .ok_or("synthetic intent omitted its expected Data digest")?;
        assert_eq!(expected.len(), 64);
        assert_eq!(
            trace["request_binding"]["body"]["intent_digest"],
            trace["intent"]["digest"]
        );
    }

    let invalid = client
        .get(format!("{route}/available"))
        .bearer_auth(&bearer)
        .send()
        .await?;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    let invalid_body: Value = invalid.json().await?;
    assert_eq!(invalid_body["code"], "WORKLOAD_TRACE_DATA_STATUS_INVALID");

    client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;
    Ok(())
}

#[tokio::test]
async fn duplicate_disposition_preview_is_owner_only_and_keeps_one_receipt()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);
    let route = format!("{base}/api/v1/market/workload-trace/duplicate-disposition-preview");

    let unauthenticated = client.get(&route).send().await?;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let preview: Value = client
        .get(&route)
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        preview["state_slice"],
        "hyperai.bazaar.workload_trace_duplicate_disposition_served.v1"
    );
    assert_eq!(
        preview["claim_ceiling"],
        "digest-bound local synthetic duplicate disposition replay refusal preview"
    );
    assert!(
        preview["nonclaims"]
            .as_str()
            .is_some_and(|value| value.contains("no second mock settlement receipt")
                && value.contains("funds movement"))
    );

    let references = preview["references"]
        .as_array()
        .ok_or("duplicate disposition preview omitted references")?;
    let first = references
        .iter()
        .find(|reference| reference["record_id"] == "synthetic-mock-disposition-v1")
        .ok_or("duplicate disposition preview omitted its first settlement")?;
    assert_eq!(first["status"], "mock_disposition_release_to_seller");
    let replay = references
        .iter()
        .find(|reference| reference["record_id"] == "synthetic-mock-disposition-replay-attempt-v1")
        .ok_or("duplicate disposition preview omitted the replay refusal")?;
    assert_eq!(replay["status"], "mock_settlement_replay_refused");
    assert_eq!(replay["digest"], Value::Null);
    assert_eq!(
        references
            .iter()
            .filter(|reference| reference["record_id"] == "synthetic-local-market-receipt-v1")
            .count(),
        1
    );

    let ordinary: Value = client
        .get(format!("{base}/api/v1/market/workload-trace"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_ne!(
        ordinary["state_slice"],
        "hyperai.bazaar.workload_trace_duplicate_disposition_served.v1"
    );
    assert!(
        !ordinary["references"].as_array().is_some_and(|references| {
            references.iter().any(|reference| {
                reference["record_id"] == "synthetic-mock-disposition-replay-attempt-v1"
            })
        })
    );

    client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;
    Ok(())
}

#[tokio::test]
async fn agent_control_is_owner_authenticated_and_rejects_invalid_profile_before_process_start()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, paths, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);

    let unauthenticated = client
        .get(format!("{base}/api/v1/agent/status"))
        .send()
        .await?;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let status: Value = client
        .get(format!("{base}/api/v1/agent/status"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(status["schema_version"], "hyperai.acp.session.v1");
    assert_eq!(status["status"], "disconnected");
    assert_workload_trace_toolset(&status);
    assert!(!status.to_string().contains(&bearer));

    let disconnect_without_json = client
        .post(format!("{base}/api/v1/agent/disconnect"))
        .bearer_auth(&bearer)
        .send()
        .await?;
    assert_eq!(disconnect_without_json.status(), StatusCode::BAD_REQUEST);
    let disconnect_error: Value = disconnect_without_json.json().await?;
    assert_eq!(disconnect_error["code"], "INVALID_JSON");

    let empty_json_disconnect = client
        .post(format!("{base}/api/v1/agent/disconnect"))
        .bearer_auth(&bearer)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .send()
        .await?;
    assert_eq!(empty_json_disconnect.status(), StatusCode::OK);
    let disconnected: Value = empty_json_disconnect.json().await?;
    assert_eq!(disconnected["status"], "disconnected");

    let invalid_profile = client
        .post(format!("{base}/api/v1/agent/connect"))
        .bearer_auth(&bearer)
        .json(&json!({
            "profile": {
                "name": "invalid local profile",
                "executable": "relative-agent",
                "working_directory": "/tmp",
                "args": [],
                "environment": {}
            }
        }))
        .send()
        .await?;
    assert_eq!(invalid_profile.status(), StatusCode::BAD_REQUEST);
    let invalid_body: Value = invalid_profile.json().await?;
    assert_eq!(invalid_body["code"], "AGENT_PROFILE_INVALID");

    let scoped_route_without_capability = client
        .get(format!("{base}/api/v1/agent/tools/data/catalog"))
        .send()
        .await?;
    assert_eq!(
        scoped_route_without_capability.status(),
        StatusCode::UNAUTHORIZED
    );

    let prompt_canary = "ACP_API_PROMPT_CANARY_DO_NOT_RETAIN";
    let prompt_without_session = client
        .post(format!("{base}/api/v1/agent/prompt"))
        .bearer_auth(&bearer)
        .json(&json!({"prompt": prompt_canary}))
        .send()
        .await?;
    assert_eq!(
        prompt_without_session.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let prompt_error: Value = prompt_without_session.json().await?;
    assert_eq!(prompt_error["code"], "AGENT_SESSION_FAILED");
    assert!(!prompt_error.to_string().contains(prompt_canary));

    let metadata = storage::connect_metadata(&paths).await?;
    let stored_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM desk_meta")
        .fetch_one(&metadata)
        .await?;
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(&metadata)
    .await?;
    assert_eq!(stored_rows, 0);
    assert_eq!(tables, ["desk_meta"]);
    metadata.close().await;
    assert_eq!(fs::read_dir(&paths.logs)?.count(), 0);

    client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;
    Ok(())
}

#[tokio::test]
async fn fleet_agent_preview_alias_matches_operator_policy_and_exposes_no_transition_route()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);
    let request = json!({ "node_id": "fixture-node-atlas", "operation": "launch" });

    let operator_preview: Value = client
        .post(format!("{base}/api/v1/fleet/operation-preview"))
        .bearer_auth(&bearer)
        .json(&request)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let agent_preview: Value = client
        .post(format!("{base}/api/v1/agent/tools/fleet/operation-preview"))
        .bearer_auth(&bearer)
        .json(&request)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    assert_eq!(agent_preview, operator_preview);
    assert_eq!(agent_preview["preview_status"], "preview_ready");
    assert_eq!(agent_preview["authority"], "not_authorized");
    assert_eq!(
        agent_preview["effect_boundary"],
        "advisory_only_does_not_invoke_or_reserve"
    );
    assert!(agent_preview.get("command").is_none());
    assert!(agent_preview.get("reservation_id").is_none());

    let transition = client
        .post(format!(
            "{base}/api/v1/agent/tools/fleet/simulation/transition"
        ))
        .bearer_auth(&bearer)
        .json(&json!({ "operation": "launch" }))
        .send()
        .await?;
    assert_eq!(transition.status(), StatusCode::NOT_FOUND);

    client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;
    Ok(())
}

#[tokio::test]
async fn agent_data_query_alias_preserves_owner_error_status_and_envelope()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);
    let request = json!({
        "query_id": "agent-error-parity-query",
        "request": {
            "dataset_id": "hyperai.synthetic_market_observations_json.v1",
            "sql": "SELECT read_file('/etc/passwd') FROM dataset",
            "format": "json",
            "max_rows": null
        }
    });

    let owner_response = client
        .post(format!("{base}/api/v1/data/query"))
        .bearer_auth(&bearer)
        .json(&request)
        .send()
        .await?;
    let owner_status = owner_response.status();
    let owner_error: Value = owner_response.json().await?;

    let agent_response = client
        .post(format!("{base}/api/v1/agent/tools/data/query"))
        .bearer_auth(&bearer)
        .json(&request)
        .send()
        .await?;
    let agent_status = agent_response.status();
    let agent_error: Value = agent_response.json().await?;

    assert_eq!(agent_status, owner_status);
    assert_eq!(agent_error, owner_error);
    assert_eq!(agent_error["code"], "DATA_QUERY_REJECTED");
    assert!(!agent_error.to_string().contains("read_file"));

    client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;
    Ok(())
}
