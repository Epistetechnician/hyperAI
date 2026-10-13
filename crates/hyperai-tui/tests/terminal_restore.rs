//! Terminal cleanup guard and narrow-layout behavior checks.

use std::{
    error::Error,
    panic::AssertUnwindSafe,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use hyperai_local_service::{AuthState, lifecycle, server, storage};
use hyperai_tui::{
    TerminalRestore, render_workspace, render_workspace_with_eval, render_workspace_with_fleet,
};
use ratatui::{Terminal, backend::TestBackend, widgets::ListState};
use serde_json::{Value, json};

#[test]
fn restoration_callback_runs_when_terminal_setup_exits_early() {
    let restored = Arc::new(AtomicBool::new(false));
    let callback_state = Arc::clone(&restored);
    let result: Result<(), &'static str> = {
        let _restore = TerminalRestore::new(move || {
            callback_state.store(true, Ordering::SeqCst);
        });
        Err("simulated terminal startup failure")
    };
    assert!(result.is_err());
    assert!(restored.load(Ordering::SeqCst));
}

#[test]
fn restoration_callback_runs_during_unwind() {
    let restored = Arc::new(AtomicBool::new(false));
    let callback_state = Arc::clone(&restored);
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _restore = TerminalRestore::new(move || {
            callback_state.store(true, Ordering::SeqCst);
        });
        std::panic::resume_unwind(Box::new("simulated terminal panic"));
    }));
    assert!(result.is_err());
    assert!(restored.load(Ordering::SeqCst));
}

#[test]
fn workspace_renders_module_state_and_survives_a_narrow_terminal() -> Result<(), Box<dyn Error>> {
    let status = json!({
        "connections": [
            { "name": "Données", "state": "en attente" }
        ]
    });
    let mut terminal = Terminal::new(TestBackend::new(80, 20))?;
    let mut list_state = ListState::default();
    terminal.draw(|frame| {
        render_workspace(frame, &status, "http://127.0.0.1:4317", 0, &mut list_state);
    })?;
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    assert!(rendered.contains("HyperAI"));
    assert!(rendered.contains("Données: en attente"));

    terminal.backend_mut().resize(32, 8);
    terminal.draw(|frame| {
        render_workspace(frame, &status, "http://127.0.0.1:4317", 0, &mut list_state);
    })?;
    Ok(())
}

#[test]
fn workload_trace_workspace_humanizes_mock_outcomes_and_keeps_claim_limits()
-> Result<(), Box<dyn Error>> {
    let status = json!({
        "connections": [],
        "workload_trace": {
            "intent": {"body": {"workload_id": "workload-local-synthetic"}},
            "digest": "trace-digest",
            "claim_ceiling": "digest-bound local synthetic projection only",
            "references": [
                {
                    "owner_id": "hyperai-market",
                    "record_id": "synthetic-mock-verifier-decision-v1",
                    "status": "mock_verdict_inconclusive",
                    "digest": "decision-digest"
                },
                {
                    "owner_id": "hyperai-market",
                    "record_id": "synthetic-mock-disposition-v1",
                    "status": "mock_disposition_hold_for_dispute",
                    "digest": null
                }
            ]
        }
    });
    let mut terminal = Terminal::new(TestBackend::new(100, 32))?;
    let mut list_state = ListState::default();
    terminal.draw(|frame| {
        render_workspace(frame, &status, "http://127.0.0.1:4317", 11, &mut list_state);
    })?;
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    assert!(rendered.contains("mock verdict inconclusive"));
    assert!(rendered.contains("mock disposition hold for dispute"));
    assert!(rendered.contains("no source digest"));
    assert!(rendered.contains("No workload ran or funds moved."));
    assert!(rendered.contains("No correctness or payment claim."));
    assert!(!rendered.contains("mock_verdict_inconclusive"));
    assert!(!rendered.contains("mock_disposition_hold_for_dispute"));

    terminal.backend_mut().resize(80, 24);
    terminal.draw(|frame| {
        render_workspace(frame, &status, "http://127.0.0.1:4317", 11, &mut list_state);
    })?;
    let narrow_rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    assert!(narrow_rendered.contains("mock verdict inconclusive"));
    assert!(narrow_rendered.contains("mock disposition hold for dispute"));
    assert!(narrow_rendered.contains("No workload ran or funds moved."));
    Ok(())
}

#[test]
fn workload_trace_tui_humanizes_rejected_verdict_and_refund_without_payment_claim()
-> Result<(), Box<dyn Error>> {
    let status = json!({
        "connections": [],
        "workload_trace": {
            "intent": {"body": {"workload_id": "workload-local-synthetic"}},
            "digest": "trace-digest",
            "claim_ceiling": "digest-bound local synthetic projection only",
            "references": [
                {
                    "owner_id": "hyperai-market",
                    "record_id": "synthetic-mock-verifier-decision-v1",
                    "status": "mock_verdict_rejected",
                    "digest": "decision-digest"
                },
                {
                    "owner_id": "hyperai-market",
                    "record_id": "synthetic-mock-disposition-v1",
                    "status": "mock_disposition_refund_to_buyer",
                    "digest": null
                }
            ]
        }
    });
    let mut terminal = Terminal::new(TestBackend::new(100, 32))?;
    let mut list_state = ListState::default();
    terminal.draw(|frame| {
        render_workspace(frame, &status, "http://127.0.0.1:4317", 11, &mut list_state);
    })?;
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    assert!(rendered.contains("mock verdict rejected"));
    assert!(rendered.contains("mock disposition refund to buyer"));
    assert!(rendered.contains("No workload ran or funds moved."));
    assert!(rendered.contains("No correctness or payment claim."));
    assert!(!rendered.contains("mock_verdict_rejected"));
    assert!(!rendered.contains("mock_disposition_refund_to_buyer"));

    terminal.backend_mut().resize(80, 24);
    terminal.draw(|frame| {
        render_workspace(frame, &status, "http://127.0.0.1:4317", 11, &mut list_state);
    })?;
    let narrow_rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    assert!(narrow_rendered.contains("mock verdict rejected"));
    assert!(narrow_rendered.contains("mock disposition refund to buyer"));
    assert!(narrow_rendered.contains("No workload ran or funds moved."));
    Ok(())
}

#[test]
fn workload_trace_tui_shows_seller_failure_claim_ceiling_at_wide_and_narrow_sizes()
-> Result<(), Box<dyn Error>> {
    let status = json!({
        "connections": [],
        "workload_trace": {
            "intent": {"body": {"workload_id": "workload-local-synthetic"}},
            "digest": "trace-digest",
            "claim_ceiling": "digest-bound local synthetic seller-attributed failure projection",
            "references": [{
                "owner_id": "hyperai-market",
                "record_id": "synthetic-attempt-1",
                "status": "fulfillment_failed",
                "digest": "failure-digest",
                "claim_ceiling": "seller_attributed_failure_report_only_not_independent_execution_or_payment_evidence"
            }]
        }
    });
    let mut terminal = Terminal::new(TestBackend::new(120, 32))?;
    let mut list_state = ListState::default();
    terminal.draw(|frame| {
        render_workspace(frame, &status, "http://127.0.0.1:4317", 11, &mut list_state);
    })?;
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    assert!(rendered.contains("fulfillment failed"));
    assert!(rendered.contains("Claim ceiling: seller report only"));
    assert!(rendered.contains("Not independent execution or payment evidence"));
    assert!(rendered.contains("Execution, correctness, and payment remain unverified."));
    assert!(rendered.contains("Preview: fixed synthetic failure"));
    assert!(rendered.contains("Keys: i=hold f=seller r=default"));
    assert!(!rendered.contains("No workload ran or funds moved."));
    assert!(!rendered.contains("seller_attributed_failure_report_only"));

    terminal.backend_mut().resize(80, 24);
    terminal.draw(|frame| {
        render_workspace(frame, &status, "http://127.0.0.1:4317", 11, &mut list_state);
    })?;
    let narrow_rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    assert!(narrow_rendered.contains("fulfillment failed"));
    assert!(narrow_rendered.contains("Claim ceiling: seller report only"));
    assert!(narrow_rendered.contains("Execution, correctness, and payment remain unverified."));
    assert!(narrow_rendered.contains("Preview: fixed synthetic failure"));
    assert!(narrow_rendered.contains("Keys: i=hold f=seller r=default"));
    assert!(!narrow_rendered.contains("No workload ran or funds moved."));
    Ok(())
}

#[tokio::test]
async fn workload_trace_tui_renders_the_authenticated_service_projection()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("hyperai-home");
    let server_home = home.clone();
    let service = tokio::spawn(async move { server::serve(Some(&server_home), 0).await });
    let paths = storage::paths_for_home(Some(&home))?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;

    let mut ready_record = None;
    for _ in 0..100 {
        if let Ok(record) = lifecycle::read_record(&paths)
            && client
                .get(format!("http://{}/healthz", record.address))
                .send()
                .await
                .is_ok_and(|response| response.status().is_success())
        {
            ready_record = Some(record);
            break;
        }
        if service.is_finished() {
            return Err("local service exited before TUI trace readiness".into());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let record = ready_record.ok_or("local service did not become ready")?;
    let bearer = AuthState::read_bearer_file(&paths.runtime.join("bearer.token"))?;
    let mut terminal = Terminal::new(TestBackend::new(140, 90))?;
    let mut list_state = ListState::default();
    assert_initial_and_bound_trace_tui_projection(
        &client,
        &mut terminal,
        &mut list_state,
        &record.address,
        &bearer,
    )
    .await?;

    let sample = home.join("data/synthetic-market-observations.json");
    #[cfg(unix)]
    assert_permission_limited_tui_projection(
        &client,
        &mut terminal,
        &mut list_state,
        &record.address,
        &bearer,
        &sample,
    )
    .await?;

    assert_conflicting_and_missing_trace_tui_projection(
        &client,
        &mut terminal,
        &mut list_state,
        &record.address,
        &bearer,
        &sample,
    )
    .await?;

    assert_seller_failure_trace_tui_projection(
        &client,
        &mut terminal,
        &mut list_state,
        &record.address,
        &bearer,
    )
    .await?;

    assert_inconclusive_trace_tui_projection(
        &client,
        &mut terminal,
        &mut list_state,
        &record.address,
        &bearer,
    )
    .await?;

    client
        .post(format!("http://{}/api/v1/service/stop", record.address))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?;
    service.await??;
    Ok(())
}

async fn assert_seller_failure_trace_tui_projection(
    client: &reqwest::Client,
    terminal: &mut Terminal<TestBackend>,
    list_state: &mut ListState,
    address: &str,
    bearer: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let failure_trace: Value = client
        .get(format!(
            "http://{address}/api/v1/market/workload-trace/failure-preview"
        ))
        .bearer_auth(bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        failure_trace["state_slice"],
        "hyperai.bazaar.workload_trace_seller_failure.v1"
    );
    let failure_status = json!({
        "connections": [],
        "workload_trace": failure_trace,
        "workload_trace_error": null
    });
    terminal.draw(|frame| {
        render_workspace(
            frame,
            &failure_status,
            &format!("http://{address}"),
            11,
            list_state,
        );
    })?;
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    assert!(rendered.contains("fixed synthetic failure"));
    assert!(rendered.contains("Execution, correctness, and payment remain unverified."));
    assert!(rendered.contains("Keys: i=hold f=seller r=default"));
    Ok(())
}

async fn assert_inconclusive_trace_tui_projection(
    client: &reqwest::Client,
    terminal: &mut Terminal<TestBackend>,
    list_state: &mut ListState,
    address: &str,
    bearer: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let trace: Value = client
        .get(format!(
            "http://{address}/api/v1/market/workload-trace/inconclusive-preview"
        ))
        .bearer_auth(bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        trace["state_slice"],
        "hyperai.bazaar.workload_trace_inconclusive_served.v1"
    );
    let status = json!({
        "connections": [],
        "workload_trace": trace,
        "workload_trace_error": null
    });
    terminal.draw(|frame| {
        render_workspace(frame, &status, &format!("http://{address}"), 11, list_state);
    })?;
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    for expected in [
        "fixed inconclusive mock outcome",
        "Keys: i=hold f=seller r=default",
        "mock verdict inconclusive",
        "mock disposition hold for dispute",
        "local fixture verdict only not independent correctness evidence",
        "no payment or entitlement claim",
        "No actual verifier decision, dispute, funds hold, or payment authorization.",
    ] {
        assert!(rendered.contains(expected), "missing {expected}");
    }

    terminal.backend_mut().resize(80, 24);
    terminal.draw(|frame| {
        render_workspace(frame, &status, &format!("http://{address}"), 11, list_state);
    })?;
    let narrow = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    for expected in [
        "fixed inconclusive mock outcome",
        "Keys: i=hold f=seller r=default",
        "Mock verdict: inconclusive.",
        "Mock disposition: hold for dispute.",
        "No actual verifier decision",
        "funds hold",
        "payment authorization",
    ] {
        assert!(
            narrow.contains(expected),
            "narrow render missing {expected}: {narrow:?}"
        );
    }
    assert!(!narrow.contains("No workload ran or funds moved."));
    assert!(!narrow.contains("payment authorized"));
    Ok(())
}

async fn assert_initial_and_bound_trace_tui_projection(
    client: &reqwest::Client,
    terminal: &mut Terminal<TestBackend>,
    list_state: &mut ListState,
    address: &str,
    bearer: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let trace = get_workload_trace(client, address, bearer).await?;
    assert_workload_trace_data_projection(
        terminal,
        list_state,
        &trace,
        address,
        "not_seeded",
        "not seeded",
        None,
    )?;

    seed_synthetic_trace_data(client, address, bearer).await?;
    let bound_trace = get_workload_trace(client, address, bearer).await?;
    let bound_digest = trace_data_digest(&bound_trace)
        .ok_or("bound service trace omitted its verified Data digest")?;
    assert_workload_trace_data_projection(
        terminal,
        list_state,
        &bound_trace,
        address,
        "bound",
        "bound",
        Some(bound_digest),
    )
}

async fn assert_conflicting_and_missing_trace_tui_projection(
    client: &reqwest::Client,
    terminal: &mut Terminal<TestBackend>,
    list_state: &mut ListState,
    address: &str,
    bearer: &str,
    sample: &std::path::Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut sample_bytes = std::fs::read(sample)?;
    sample_bytes.push(b' ');
    std::fs::write(sample, &sample_bytes)?;
    let conflicting_trace = get_workload_trace(client, address, bearer).await?;
    assert_workload_trace_data_projection(
        terminal,
        list_state,
        &conflicting_trace,
        address,
        "conflicting",
        "conflicting",
        None,
    )?;

    std::fs::remove_file(sample)?;
    let missing_trace = get_workload_trace(client, address, bearer).await?;
    assert_workload_trace_data_projection(
        terminal,
        list_state,
        &missing_trace,
        address,
        "missing",
        "missing",
        None,
    )?;

    let data_root = sample
        .parent()
        .ok_or("synthetic data file has no parent directory")?;
    let oversized_manifest = data_root
        .join("manifests")
        .join("unavailable-test-manifest.json");
    std::fs::write(&oversized_manifest, vec![b'x'; 16 * 1024 + 1])?;
    let unavailable_trace = get_workload_trace(client, address, bearer).await?;
    assert!(
        !unavailable_trace
            .to_string()
            .contains("unavailable-test-manifest.json")
    );
    assert_workload_trace_data_projection(
        terminal,
        list_state,
        &unavailable_trace,
        address,
        "unavailable",
        "unavailable",
        None,
    )
}

fn trace_data_digest(trace: &Value) -> Option<&str> {
    trace["references"]
        .as_array()?
        .iter()
        .find(|reference| reference["owner_id"] == "hyperai-data")?["digest"]
        .as_str()
}

async fn get_workload_trace(
    client: &reqwest::Client,
    address: &str,
    bearer: &str,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    Ok(client
        .get(format!("http://{address}/api/v1/market/workload-trace"))
        .bearer_auth(bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

#[cfg(unix)]
async fn assert_permission_limited_tui_projection(
    client: &reqwest::Client,
    terminal: &mut Terminal<TestBackend>,
    list_state: &mut ListState,
    address: &str,
    bearer: &str,
    sample: &std::path::Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = std::fs::metadata(sample)?.permissions();
    let original_mode = permissions.mode();
    permissions.set_mode(original_mode & !0o444);
    std::fs::set_permissions(sample, permissions)?;

    let direct_read = std::fs::read(sample);
    let denied = direct_read
        .as_ref()
        .is_err_and(|error| error.kind() == std::io::ErrorKind::PermissionDenied);
    eprintln!(
        "Data permission probe: {}",
        if denied {
            "PermissionDenied"
        } else {
            "readable; denied not exercised"
        }
    );
    let trace_result = get_workload_trace(client, address, bearer).await;

    let mut restored = std::fs::metadata(sample)?.permissions();
    restored.set_mode(original_mode);
    std::fs::set_permissions(sample, restored)?;
    let trace = trace_result?;
    if denied {
        assert_workload_trace_data_projection(
            terminal, list_state, &trace, address, "denied", "denied", None,
        )
    } else {
        let digest = trace_data_digest(&trace)
            .ok_or("readable permission probe did not return a bound Data digest")?;
        assert_workload_trace_data_projection(
            terminal,
            list_state,
            &trace,
            address,
            "bound",
            "bound",
            Some(digest),
        )
    }
}

async fn seed_synthetic_trace_data(
    client: &reqwest::Client,
    address: &str,
    bearer: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let preview: Value = client
        .get(format!("http://{address}/api/v1/data/synthetic/preview"))
        .bearer_auth(bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    client
        .post(format!("http://{address}/api/v1/data/synthetic/seed"))
        .bearer_auth(bearer)
        .header("Idempotency-Key", "tui-trace-parity-seed-v1")
        .json(&json!({"preview_digest": preview["preview_digest"]}))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

fn assert_workload_trace_data_projection(
    terminal: &mut Terminal<TestBackend>,
    list_state: &mut ListState,
    trace: &Value,
    address: &str,
    expected_status: &str,
    expected_label: &str,
    expected_digest: Option<&str>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let trace_digest = trace["digest"]
        .as_str()
        .ok_or("service trace omitted its digest")?;
    let data_reference = trace["references"]
        .as_array()
        .and_then(|references| {
            references
                .iter()
                .find(|reference| reference["owner_id"] == "hyperai-data")
        })
        .ok_or("service trace omitted the Data context reference")?;
    assert_eq!(data_reference["status"], expected_status);
    if let Some(digest) = expected_digest {
        assert_eq!(data_reference["digest"], digest);
        assert_eq!(
            trace["intent"]["body"]["data_references"][0]["digest"],
            digest
        );
    } else {
        assert!(data_reference["digest"].is_null());
        assert_eq!(trace["intent"]["body"]["data_references"], json!([]));
    }
    let status = json!({"connections": [], "workload_trace": trace});
    terminal.draw(|frame| {
        render_workspace(frame, &status, &format!("http://{address}"), 11, list_state);
    })?;
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    assert!(rendered.contains("workload-hyperai-local-synthetic-v1"));
    assert!(rendered.contains(&trace_digest[..12]));
    assert!(rendered.contains(expected_label));
    if let Some(digest) = expected_digest {
        assert!(rendered.contains(&digest[..12]));
    } else {
        assert!(rendered.contains("no source digest"));
    }
    assert!(rendered.contains("mock verdict verified"));
    assert!(rendered.contains("mock disposition release to seller"));
    assert!(rendered.contains("No workload ran or funds moved."));
    assert!(rendered.contains("No correctness or payment claim."));
    if expected_status == "unavailable" {
        terminal.backend_mut().resize(80, 24);
        terminal.draw(|frame| {
            render_workspace(frame, &status, &format!("http://{address}"), 11, list_state);
        })?;
        let narrow_rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        for expected in [
            "unavailable",
            "no source digest",
            "No workload ran or funds moved.",
            "No correctness or payment claim.",
        ] {
            assert!(
                narrow_rendered.contains(expected),
                "narrow render missing {expected}: {narrow_rendered:?}"
            );
        }
        terminal.backend_mut().resize(140, 90);
    }
    Ok(())
}

#[test]
fn agent_workspace_shows_profile_scope_and_local_controls() -> Result<(), Box<dyn Error>> {
    let status = json!({
        "connections": [],
        "agent": {
            "status": "ready",
            "local_session_id": "0123456789abcdef0123456789abcdef",
            "session_id": "synthetic-session-v1",
            "profile_digest": "a".repeat(64),
            "tool_set_version": "hyperai.agent-tools.readonly.v5",
            "data_scope": "local_synthetic_data_and_readonly_fleet_eval_trade",
            "protocol_version": 1,
            "profile": {
                "name": "Synthetic agent",
                "executable": "/tmp/acp-fixture-peer",
                "working_directory": "/tmp"
            },
            "capabilities": [],
            "tools": ["hyperai_data_catalog"],
            "events": [],
            "error_code": null
        }
    });
    let mut terminal = Terminal::new(TestBackend::new(120, 36))?;
    let mut list_state = ListState::default();
    terminal.draw(|frame| {
        render_workspace(frame, &status, "http://127.0.0.1:4317", 10, &mut list_state);
    })?;
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    assert!(rendered.contains("ACP session: ready"));
    assert!(rendered.contains("Synthetic agent"));
    assert!(rendered.contains("hyperai.agent-tools.readonly.v5"));
    assert!(rendered.contains("Enter prompt"));
    Ok(())
}

#[test]
fn fleet_workspace_labels_virtual_state_and_recovery_boundary() -> Result<(), Box<dyn Error>> {
    let status = json!({
        "connections": [],
        "fleet_inventory": {
            "nodes": [{
                "node_id": "fixture-node-atlas",
                "display_name": "Atlas illustrative node",
                "health_state": "healthy"
            }]
        },
        "fleet_doctor": {
            "mode": "synthetic_fixture_only",
            "checks": [
                {"check_id": "live_health_probe", "status": "unknown"},
                {"check_id": "operation_authority", "status": "blocked"}
            ]
        },
        "fleet_preview": {
            "preview_status": "preview_ready",
            "preview_digest": "a".repeat(64),
            "node_id": "fixture-node-atlas",
            "operation": "launch",
            "authority": "not_authorized",
            "effect_boundary": "advisory_only_does_not_invoke_or_reserve",
            "health_gate": "fixture_marks_healthy_but_does_not_establish_live_health"
        },
        "fleet_selected_node_index": 0,
        "fleet_selected_operation": "launch",
        "fleet_message": "Advisory preview ready. No resource was contacted or reserved."
    });
    let simulation = json!({
        "simulation_id": "fleet-scenario-atlas-v1",
        "owner": "local_service_simulation",
        "phase": "reconciliation_required",
        "allocated_units": 0,
        "quarantined_units": 2,
        "virtual_capacity_units": 4,
        "lease_state": "unknown",
        "virtual_task_id": null,
        "last_operation": "launch",
        "persistence": "volatile_one_local_service_process",
    });
    let mut terminal = Terminal::new(TestBackend::new(120, 36))?;
    let mut list_state = ListState::default();
    terminal.draw(|frame| {
        render_workspace_with_fleet(
            frame,
            &status,
            &simulation,
            "http://127.0.0.1:4317",
            5,
            &mut list_state,
        );
    })?;
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    assert!(rendered.contains("Workspace: Fleet / inventory, preview, simulator"));
    assert!(rendered.contains("Inventory: 1 synthetic fixtures"));
    assert!(rendered.contains("Doctor · live probe unknown · operation authority blocked"));
    assert!(rendered.contains("Preview: preview_ready"));
    assert!(rendered.contains("Authority: not_authorized"));
    assert!(rendered.contains("advisory_only_does_not_invoke_or_reserve"));
    assert!(rendered.contains("Keys: n target · o intent · v preview · Enter simulate"));
    assert!(rendered.contains("No resource was contacted or reserved"));
    assert!(rendered.contains("reconciliation_required"));
    assert!(rendered.contains("Synthetic fixtures only · no live health/capacity or host calls."));

    terminal.backend_mut().resize(32, 8);
    terminal.draw(|frame| {
        render_workspace_with_fleet(
            frame,
            &status,
            &simulation,
            "http://127.0.0.1:4317",
            5,
            &mut list_state,
        );
    })?;
    Ok(())
}

#[test]
fn eval_workspace_labels_fixed_fixture_comparison_and_survives_narrow_terminal()
-> Result<(), Box<dyn Error>> {
    let status = json!({"connections": []});
    let catalog = json!({
        "suite_id": "hyperai.synthetic_eval.compute_market.v1",
        "suite_digest": "sha256:synthetic-suite",
        "repeat_count": 3,
        "cases": [
            {"case_id": "deal_constraints", "category": "deal_constraints"},
            {"case_id": "source_attribution", "category": "source_attribution"},
            {"case_id": "market_data_limits", "category": "market_data_limits"},
            {"case_id": "risk_reasoning", "category": "risk_reasoning"},
            {"case_id": "financing_boundaries", "category": "financing_boundaries"},
            {"case_id": "operational_safety", "category": "operational_safety"}
        ]
    });
    let report = json!({
        "control": {"matching_cases": 6, "case_count": 6},
        "alternate": {"matching_cases": 3, "case_count": 6},
        "repeated_result_digest": "sha256:repeat",
        "child_wall_time_ms": [1, 1, 1],
        "scorer_input_bytes": 142,
        "scorer_output_bytes": 512,
        "cases": []
    });
    let mut terminal = Terminal::new(TestBackend::new(100, 30))?;
    let mut list_state = ListState::default();
    terminal.draw(|frame| {
        render_workspace_with_eval(
            frame,
            &status,
            &catalog,
            Some(&report),
            "http://127.0.0.1:4317",
            6,
            &mut list_state,
        );
    })?;
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    assert!(rendered.contains("Workspace: Eval"));
    assert!(rendered.contains("no agent or model runs"));
    assert!(rendered.contains("6/6 rubric matches"));
    assert!(rendered.contains("3/6 rubric matches"));

    terminal.backend_mut().resize(32, 8);
    terminal.draw(|frame| {
        render_workspace_with_eval(
            frame,
            &status,
            &catalog,
            Some(&report),
            "http://127.0.0.1:4317",
            6,
            &mut list_state,
        );
    })?;
    Ok(())
}
