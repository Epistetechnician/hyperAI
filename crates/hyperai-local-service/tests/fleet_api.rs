//! Authenticated local Fleet inventory and advisory-plan API checks.

use std::{error::Error, path::PathBuf, time::Duration};

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

async fn assert_inventory(
    client: &Client,
    base: &str,
    bearer: &str,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let inventory: Value = client
        .get(format!("{base}/api/v1/fleet/inventory"))
        .bearer_auth(bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let repeated: Value = client
        .get(format!("{base}/api/v1/fleet/inventory"))
        .bearer_auth(bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(inventory, repeated);
    assert_eq!(inventory["schema_version"], "hyperai.fleet.inventory.v1");
    assert_eq!(
        inventory["provenance"]["observation_mode"],
        "no_live_probe_or_import"
    );
    assert_eq!(inventory["nodes"].as_array().map(Vec::len), Some(4));
    assert_eq!(
        inventory["health_timeline"].as_array().map(Vec::len),
        Some(4)
    );
    for state in ["healthy", "stale", "unreachable", "unknown"] {
        assert!(
            inventory["nodes"]
                .as_array()
                .is_some_and(|nodes| nodes.iter().any(|node| node["health_state"] == state))
        );
    }
    assert_eq!(inventory["doctor"]["mode"], "synthetic_fixture_only");
    let doctor: Value = client
        .get(format!("{base}/api/v1/fleet/doctor"))
        .bearer_auth(bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(doctor, inventory["doctor"]);
    Ok(inventory)
}

async fn assert_plan_paths(
    client: &Client,
    base: &str,
    bearer: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let first_plan: Value = client
        .post(format!("{base}/api/v1/fleet/operation-preview"))
        .bearer_auth(bearer)
        .json(&json!({ "node_id": "fixture-node-atlas", "operation": "launch" }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(first_plan["preview_status"], "preview_ready");
    assert_eq!(first_plan["authority"], "not_authorized");
    assert_eq!(
        first_plan["effect_boundary"],
        "advisory_only_does_not_invoke_or_reserve"
    );
    assert!(first_plan.get("command").is_none());
    assert!(first_plan.get("reservation_id").is_none());

    let changed_plan: Value = client
        .post(format!("{base}/api/v1/fleet/operation-preview"))
        .bearer_auth(bearer)
        .json(&json!({
            "node_id": "fixture-node-atlas",
            "operation": "terminate",
            "expected_preview_digest": first_plan["preview_digest"]
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(changed_plan["preview_status"], "preview_changed");
    assert_ne!(changed_plan["preview_digest"], first_plan["preview_digest"]);

    let malformed = client
        .post(format!("{base}/api/v1/fleet/operation-preview"))
        .bearer_auth(bearer)
        .json(&json!({
            "node_id": "fixture-node-atlas",
            "operation": "launch",
            "ssh_host": "localhost"
        }))
        .send()
        .await?;
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);

    let unknown_node = client
        .post(format!("{base}/api/v1/fleet/operation-preview"))
        .bearer_auth(bearer)
        .json(&json!({ "node_id": "workstation", "operation": "attach" }))
        .send()
        .await?;
    assert_eq!(unknown_node.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let rejected: Value = unknown_node.json().await?;
    assert_eq!(rejected["code"], "FLEET_REQUEST_REJECTED");
    Ok(())
}

async fn simulation_snapshot(
    client: &Client,
    base: &str,
    bearer: &str,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    Ok(client
        .get(format!("{base}/api/v1/fleet/simulation"))
        .bearer_auth(bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

async fn simulation_transition(
    client: &Client,
    base: &str,
    bearer: &str,
    request: Value,
) -> Result<reqwest::Response, Box<dyn Error + Send + Sync>> {
    Ok(client
        .post(format!("{base}/api/v1/fleet/simulation/transition"))
        .bearer_auth(bearer)
        .json(&request)
        .send()
        .await?)
}

fn simulation_request(
    snapshot: &Value,
    key: &str,
    operation: &str,
    fault: &str,
    gpu_units: Option<u8>,
) -> Value {
    json!({
        "expected_state_digest": snapshot["state_digest"],
        "idempotency_key": key,
        "operation": operation,
        "gpu_units": gpu_units,
        "fault": fault,
    })
}

#[tokio::test]
async fn fleet_simulation_is_authenticated_idempotent_and_reconciles_ambiguous_outcomes()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);

    let unauthenticated = client
        .get(format!("{base}/api/v1/fleet/simulation"))
        .send()
        .await?;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let initial = simulation_snapshot(&client, &base, &bearer).await?;
    assert_eq!(
        initial["schema_version"],
        "hyperai.fleet.simulation.snapshot.v1"
    );
    assert_eq!(initial["state_slice"], "hyperai.bazaar.fleet_simulation.v1");
    assert_eq!(initial["phase"], "detached");
    assert_eq!(initial["virtual_capacity_units"], 4);
    assert_eq!(initial["persistence"], "volatile_one_local_service_process");

    let attach_request = simulation_request(&initial, "attach-once", "attach", "none", None);
    let attach = simulation_transition(&client, &base, &bearer, attach_request.clone()).await?;
    assert_eq!(attach.status(), StatusCode::OK);
    let attach_value: Value = attach.json().await?;
    let replay = simulation_transition(&client, &base, &bearer, attach_request).await?;
    let replay_value: Value = replay.json().await?;
    assert_eq!(attach_value, replay_value);
    assert_eq!(attach_value["snapshot"]["phase"], "attached");

    let changed_same_key = simulation_request(&initial, "attach-once", "launch", "none", Some(1));
    let conflict = simulation_transition(&client, &base, &bearer, changed_same_key).await?;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    assert_eq!(
        conflict.json::<Value>().await?["code"],
        "FLEET_SIMULATION_CONFLICT"
    );

    let attached = simulation_snapshot(&client, &base, &bearer).await?;
    let reconciled = reconcile_ambiguous_launch(&client, &base, &bearer, &attached).await?;

    let snapshot = simulation_snapshot(&client, &base, &bearer).await?;
    let malformed = simulation_transition(
        &client,
        &base,
        &bearer,
        json!({
            "expected_state_digest": snapshot["state_digest"],
            "idempotency_key": "unknown-field",
            "operation": "workload_start",
            "shell_command": "echo unsafe"
        }),
    )
    .await?;
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);

    let json = serde_json::to_string(&reconciled)?;
    assert!(!json.contains("shell_command"));
    assert!(!json.contains("process_id"));
    assert!(!json.contains("pid"));
    assert_eq!(
        reconciled["effect_boundary"],
        "simulation_only_no_process_no_network"
    );

    task.abort();
    let _ = task.await;
    Ok(())
}

async fn reconcile_ambiguous_launch(
    client: &Client,
    base: &str,
    bearer: &str,
    attached: &Value,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let launch = simulation_transition(
        client,
        base,
        bearer,
        simulation_request(
            attached,
            "launch-timeout-after",
            "launch",
            "timeout_after_effect",
            Some(2),
        ),
    )
    .await?;
    let launch_value: Value = launch.json().await?;
    assert_eq!(
        launch_value["transition_status"],
        "outcome_unknown_reconciliation_required"
    );
    assert_eq!(launch_value["snapshot"]["phase"], "reconciliation_required");
    assert_eq!(launch_value["snapshot"]["quarantined_units"], 2);

    let unknown = simulation_snapshot(client, base, bearer).await?;
    let blocked = simulation_transition(
        client,
        base,
        bearer,
        simulation_request(&unknown, "blocked-start", "workload_start", "none", None),
    )
    .await?;
    assert_eq!(blocked.status(), StatusCode::CONFLICT);
    assert_eq!(
        blocked.json::<Value>().await?["code"],
        "FLEET_SIMULATION_TRANSITION_REJECTED"
    );

    let reconcile = simulation_transition(
        client,
        base,
        bearer,
        simulation_request(&unknown, "reconcile-launch", "reconcile", "none", None),
    )
    .await?;
    let reconciled: Value = reconcile.json().await?;
    assert_eq!(reconciled["transition_status"], "reconciled_effect_applied");
    assert_eq!(reconciled["snapshot"]["phase"], "launched");
    assert_eq!(reconciled["snapshot"]["allocated_units"], 2);
    Ok(reconciled)
}

#[tokio::test]
async fn fleet_simulation_partial_termination_requires_cleanup_to_release_lease()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);

    let initial = simulation_snapshot(&client, &base, &bearer).await?;
    let attach = simulation_transition(
        &client,
        &base,
        &bearer,
        simulation_request(&initial, "attach", "attach", "none", None),
    )
    .await?
    .json::<Value>()
    .await?;
    let launch = simulation_transition(
        &client,
        &base,
        &bearer,
        simulation_request(&attach["snapshot"], "launch", "launch", "none", Some(4)),
    )
    .await?
    .json::<Value>()
    .await?;
    let partial = simulation_transition(
        &client,
        &base,
        &bearer,
        simulation_request(
            &launch["snapshot"],
            "partial-terminate",
            "terminate",
            "partial_effect",
            None,
        ),
    )
    .await?
    .json::<Value>()
    .await?;
    assert_eq!(partial["snapshot"]["phase"], "cleanup_required");
    assert_eq!(partial["snapshot"]["allocated_units"], 2);

    let cleanup = simulation_transition(
        &client,
        &base,
        &bearer,
        simulation_request(&partial["snapshot"], "cleanup", "cleanup", "none", None),
    )
    .await?
    .json::<Value>()
    .await?;
    assert_eq!(cleanup["snapshot"]["phase"], "terminated");
    assert_eq!(cleanup["snapshot"]["allocated_units"], 0);
    assert_eq!(cleanup["snapshot"]["virtual_lease_id"], Value::Null);

    task.abort();
    let _ = task.await;
    Ok(())
}

#[tokio::test]
async fn fleet_contract_is_authenticated_deterministic_and_preview_only()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);

    let unauthenticated = client
        .get(format!("{base}/api/v1/fleet/inventory"))
        .send()
        .await?;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
    let inventory = assert_inventory(&client, &base, &bearer).await?;
    assert_eq!(inventory["schema_version"], "hyperai.fleet.inventory.v1");
    assert_plan_paths(&client, &base, &bearer).await?;

    task.abort();
    let _ = task.await;
    Ok(())
}
