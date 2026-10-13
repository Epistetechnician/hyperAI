//! End-to-end stable-v1 negotiation against the bundled local synthetic peer.

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
use std::{collections::BTreeMap, error::Error, path::PathBuf};

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
use tokio::io::AsyncReadExt;

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
use hyperai_acp::{AgentMcpConfigV1, AgentProfileV1, AgentSessionManager};

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
static SYNTHETIC_PEER_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
async fn read_bounded_http_request(stream: &mut tokio::net::TcpStream) -> std::io::Result<Vec<u8>> {
    let mut request = Vec::with_capacity(1024);
    let mut chunk = [0_u8; 512];
    loop {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            return Err(std::io::Error::other(
                "scoped-capability request ended before its headers",
            ));
        }
        request.extend_from_slice(&chunk[..read]);
        if request.len() > 8 * 1024 {
            return Err(std::io::Error::other(
                "scoped-capability request exceeded the fixture limit",
            ));
        }
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    let headers_end = request
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| std::io::Error::other("request headers were incomplete"))?;
    let headers = String::from_utf8_lossy(&request[..headers_end]);
    let content_length = headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find_map(|(name, value)| {
            name.eq_ignore_ascii_case("content-length")
                .then_some(value.trim())
        })
        .map(str::parse::<usize>)
        .transpose()
        .map_err(|_| std::io::Error::other("request Content-Length was invalid"))?
        .unwrap_or(0);
    let body_start = headers_end + 4;
    let expected_length = body_start
        .checked_add(content_length)
        .ok_or_else(|| std::io::Error::other("request length overflowed"))?;
    if expected_length > 8 * 1024 || request.len() > expected_length {
        return Err(std::io::Error::other(
            "scoped-capability request exceeded its declared fixture limit",
        ));
    }
    while request.len() < expected_length {
        let remaining = expected_length - request.len();
        let read_length = remaining.min(chunk.len());
        let read = stream.read(&mut chunk[..read_length]).await?;
        if read == 0 {
            return Err(std::io::Error::other(
                "scoped-capability request ended before its body",
            ));
        }
        request.extend_from_slice(&chunk[..read]);
    }
    Ok(request)
}

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
fn read_sanitized_probe_failure(directory: &std::path::Path, capability: &str) -> String {
    use std::io::Read as _;

    std::fs::File::open(directory.join("mcp-capability-probe.failure"))
        .ok()
        .and_then(|file| {
            let mut bytes = Vec::new();
            file.take(512).read_to_end(&mut bytes).ok()?;
            Some(
                String::from_utf8_lossy(&bytes)
                    .chars()
                    .map(|character| {
                        if character.is_control() {
                            ' '
                        } else {
                            character
                        }
                    })
                    .collect::<String>()
                    .replace(capability, "[redacted]"),
            )
        })
        .unwrap_or_else(|| "<unavailable>".to_owned())
}

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
#[tokio::test]
async fn synthetic_peer_negotiates_v1_uses_fixed_mcp_and_completes_prompt()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let _test_guard = SYNTHETIC_PEER_TEST_LOCK.lock().await;
    let temporary = tempfile::tempdir()?;
    let manager = AgentSessionManager::default();
    let profile = AgentProfileV1 {
        name: "HyperAI synthetic ACP peer".to_owned(),
        executable: PathBuf::from(env!("CARGO_BIN_EXE_acp-fixture-peer")),
        args: Vec::new(),
        working_directory: temporary.path().to_path_buf(),
        environment: BTreeMap::default(),
    };
    let mcp = AgentMcpConfigV1 {
        endpoint: "http://127.0.0.1:4317".to_owned(),
        capability: "synthetic-read-only-capability-0123456789".to_owned(),
        executable: temporary.path().join("hyperai-mcp-fixture"),
    };

    let connected = manager.connect(profile, mcp).await?;
    assert_eq!(connected.status, "ready");
    assert_eq!(connected.protocol_version, Some(1));
    assert_eq!(
        connected.local_session_id.as_ref().map(String::len),
        Some(32)
    );
    assert_eq!(connected.profile_digest.as_ref().map(String::len), Some(64));
    assert_eq!(
        connected.tool_set_version,
        "hyperai.agent-tools.readonly.v6"
    );
    assert_eq!(
        connected.data_scope,
        "local_synthetic_buyer_supply_quote_workload_trace_data_readonly_fleet_eval_trade"
    );
    assert_eq!(
        connected
            .profile
            .as_ref()
            .map(|profile| profile.name.as_str()),
        Some("HyperAI synthetic ACP peer")
    );
    assert_eq!(
        connected.session_id.as_deref(),
        Some("synthetic-session-v1")
    );

    let prompt_canary = "ACP_PROMPT_CANARY_DO_NOT_RETAIN";
    let prompted = manager.prompt(prompt_canary.to_owned()).await?;
    assert_eq!(prompted.status, "ready");
    assert_eq!(prompted.events.len(), 1);
    assert_eq!(prompted.events[0].text, "Synthetic ACP response.");
    assert!(!serde_json::to_string(&prompted)?.contains(prompt_canary));

    let disconnected = manager.disconnect().await?;
    assert_eq!(disconnected.status, "disconnected");
    let after_disconnect = manager.snapshot().await;
    assert_eq!(after_disconnect.status, "disconnected");
    assert!(after_disconnect.events.is_empty());
    Ok(())
}

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
const SCOPED_CAPABILITY_ROUTE_CHECKS: [(&str, u16, &str); 12] = [
    (
        "GET /api/v1/agent/tools/data/catalog HTTP/1.1\r\n",
        200,
        "{}",
    ),
    (
        "GET /api/v1/agent/tools/market/workload-trace/inconclusive-preview HTTP/1.1\r\n",
        200,
        "{}",
    ),
    (
        "GET /api/v1/agent/tools/market/workload-trace/failure-preview HTTP/1.1\r\n",
        200,
        r#"{"state_slice":"hyperai.bazaar.workload_trace_seller_failure.v1"}"#,
    ),
    ("GET /api/v1/market/workload-trace HTTP/1.1\r\n", 401, "{}"),
    (
        "GET /api/v1/market/workload-trace/failure-preview HTTP/1.1\r\n",
        401,
        "{}",
    ),
    (
        "GET /api/v1/market/workload-trace/inconclusive-preview HTTP/1.1\r\n",
        401,
        "{}",
    ),
    ("POST /api/v1/market/quote-approve HTTP/1.1\r\n", 401, "{}"),
    ("POST /api/v1/market/quote-complete HTTP/1.1\r\n", 401, "{}"),
    (
        "POST /api/v1/agent/tools/market/workload-trace/failure-preview HTTP/1.1\r\n",
        401,
        "{}",
    ),
    (
        "POST /api/v1/agent/tools/market/workload-trace/inconclusive-preview HTTP/1.1\r\n",
        401,
        "{}",
    ),
    ("GET /api/v1/agent/tools/unknown HTTP/1.1\r\n", 401, "{}"),
    (
        "POST /api/v1/fleet/simulation/transition HTTP/1.1\r\n",
        401,
        "{}",
    ),
];

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
#[tokio::test]
async fn synthetic_peer_checks_scoped_mcp_capability_allow_and_deny_routes()
-> Result<(), Box<dyn Error + Send + Sync>> {
    use tokio::{io::AsyncWriteExt, net::TcpListener};

    let _test_guard = SYNTHETIC_PEER_TEST_LOCK.lock().await;
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let address = listener.local_addr()?;
    let capability = "synthetic-scoped-capability-0123456789";
    let route_checks = tokio::spawn(async move {
        for (request_prefix, status, body) in SCOPED_CAPABILITY_ROUTE_CHECKS {
            let (mut stream, _) = listener.accept().await?;
            let request = read_bounded_http_request(&mut stream).await?;
            let request = String::from_utf8_lossy(&request);
            if !request.starts_with(request_prefix) {
                return Err(std::io::Error::other(
                    "unexpected scoped-capability method or path",
                ));
            }
            if !request.contains(&format!("Authorization: Bearer {capability}\r\n")) {
                return Err(std::io::Error::other(
                    "scoped-capability request omitted its expected bearer header",
                ));
            }
            let reason = if status == 200 { "OK" } else { "Unauthorized" };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).await?;
        }
        Ok::<(), std::io::Error>(())
    });

    let temporary = tempfile::tempdir()?;
    let manager = AgentSessionManager::default();
    let profile = AgentProfileV1 {
        name: "HyperAI scoped-capability synthetic ACP peer".to_owned(),
        executable: PathBuf::from(env!("CARGO_BIN_EXE_acp-fixture-peer")),
        args: vec!["--probe-mcp-capability".to_owned()],
        working_directory: temporary.path().to_path_buf(),
        environment: BTreeMap::default(),
    };
    let mcp = AgentMcpConfigV1 {
        endpoint: format!("http://{address}"),
        capability: capability.to_owned(),
        executable: temporary.path().join("hyperai-mcp-fixture"),
    };

    let connected = match manager.connect(profile, mcp).await {
        Ok(connected) => connected,
        Err(error) => {
            let failure_marker = read_sanitized_probe_failure(temporary.path(), capability);
            let route_result = tokio::time::timeout(Duration::from_millis(100), route_checks).await;
            return Err(format!(
                "synthetic MCP scope probe failed: {error}; fixture failure marker: {failure_marker}; local route result: {route_result:?}"
            )
            .into());
        }
    };
    assert_eq!(connected.status, "ready");
    assert_eq!(manager.disconnect().await?.status, "disconnected");
    route_checks.await??;
    Ok(())
}

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
#[tokio::test]
async fn cancellation_completes_a_pending_prompt_and_keeps_the_session_connected()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let _test_guard = SYNTHETIC_PEER_TEST_LOCK.lock().await;
    let temporary = tempfile::tempdir()?;
    let manager = Arc::new(AgentSessionManager::default());
    let profile = AgentProfileV1 {
        name: "HyperAI cancellable synthetic ACP peer".to_owned(),
        executable: PathBuf::from(env!("CARGO_BIN_EXE_acp-fixture-peer")),
        args: vec!["--wait-for-cancel".to_owned()],
        working_directory: temporary.path().to_path_buf(),
        environment: BTreeMap::default(),
    };
    let mcp = AgentMcpConfigV1 {
        endpoint: "http://127.0.0.1:4317".to_owned(),
        capability: "synthetic-read-only-capability-0123456789".to_owned(),
        executable: temporary.path().join("hyperai-mcp-fixture"),
    };

    manager.connect(profile, mcp).await?;
    let prompt_manager = Arc::clone(&manager);
    let prompt = tokio::spawn(async move {
        prompt_manager
            .prompt("Wait for a local cancellation.".to_owned())
            .await
    });

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let snapshot = manager.snapshot().await;
            if snapshot
                .events
                .iter()
                .any(|event| event.text == "Synthetic prompt is pending.")
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;

    let cancellation = manager.cancel().await?;
    assert!(matches!(
        cancellation.status.as_str(),
        "cancellation_requested" | "cancelled"
    ));
    let completed = tokio::time::timeout(Duration::from_secs(5), prompt).await??;
    assert_eq!(completed?.status, "cancelled");

    let disconnected = manager.disconnect().await?;
    assert_eq!(disconnected.status, "disconnected");
    Ok(())
}

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
#[tokio::test]
async fn disconnect_is_bounded_when_the_peer_stalls_session_close()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let _test_guard = SYNTHETIC_PEER_TEST_LOCK.lock().await;
    let temporary = tempfile::tempdir()?;
    let manager = AgentSessionManager::default();
    let profile = AgentProfileV1 {
        name: "HyperAI stalled-close synthetic ACP peer".to_owned(),
        executable: PathBuf::from(env!("CARGO_BIN_EXE_acp-fixture-peer")),
        args: vec!["--close-support".to_owned(), "--stall-close".to_owned()],
        working_directory: temporary.path().to_path_buf(),
        environment: BTreeMap::default(),
    };
    let mcp = AgentMcpConfigV1 {
        endpoint: "http://127.0.0.1:4317".to_owned(),
        capability: "synthetic-read-only-capability-0123456789".to_owned(),
        executable: temporary.path().join("hyperai-mcp-fixture"),
    };

    manager.connect(profile, mcp).await?;
    let started = Instant::now();
    let disconnected = tokio::time::timeout(Duration::from_secs(5), manager.disconnect()).await??;
    assert_eq!(disconnected.status, "disconnected");
    assert!(started.elapsed() < Duration::from_secs(4));
    Ok(())
}

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
#[tokio::test]
async fn disconnect_stops_the_synthetic_peers_owned_process_tree()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let _test_guard = SYNTHETIC_PEER_TEST_LOCK.lock().await;
    let temporary = tempfile::tempdir()?;
    let pid_file = temporary.path().join("synthetic-child.pid");
    let manager = AgentSessionManager::default();
    let profile = AgentProfileV1 {
        name: "HyperAI child-process synthetic ACP peer".to_owned(),
        executable: PathBuf::from(env!("CARGO_BIN_EXE_acp-fixture-peer")),
        args: vec![
            "--close-support".to_owned(),
            "--spawn-child-pid-file".to_owned(),
            pid_file.to_string_lossy().into_owned(),
        ],
        working_directory: temporary.path().to_path_buf(),
        environment: BTreeMap::default(),
    };
    let mcp = AgentMcpConfigV1 {
        endpoint: "http://127.0.0.1:4317".to_owned(),
        capability: "synthetic-read-only-capability-0123456789".to_owned(),
        executable: temporary.path().join("hyperai-mcp-fixture"),
    };

    manager.connect(profile, mcp).await?;
    let pid_text = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Ok(pid_text) = std::fs::read_to_string(&pid_file) {
                break pid_text;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    let child_pid = pid_text.trim().parse::<u32>()?;
    assert!(synthetic_process_is_running(child_pid, pid_text.trim()));

    assert_eq!(manager.disconnect().await?.status, "disconnected");
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if !synthetic_process_is_running(child_pid, pid_text.trim()) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?;
    assert!(!synthetic_process_is_running(child_pid, pid_text.trim()));
    Ok(())
}

#[cfg(target_os = "linux")]
fn synthetic_process_is_running(raw_pid_number: u32, raw_pid: &str) -> bool {
    let Ok(raw_pid) = i32::try_from(raw_pid_number) else {
        return false;
    };
    let Some(pid) = rustix::process::Pid::from_raw(raw_pid) else {
        return false;
    };
    if rustix::process::test_kill_process(pid).is_err() {
        return false;
    }
    let Ok(stat) = std::fs::read_to_string(format!("/proc/{raw_pid}/stat")) else {
        return true;
    };
    let Some(state) = stat
        .rsplit_once(')')
        .and_then(|(_, fields)| fields.split_whitespace().next())
    else {
        return true;
    };
    !matches!(state, "Z" | "X")
}

#[cfg(all(unix, not(target_os = "linux")))]
fn synthetic_process_is_running(raw_pid_number: u32, _raw_pid: &str) -> bool {
    let Ok(raw_pid) = i32::try_from(raw_pid_number) else {
        return false;
    };
    rustix::process::Pid::from_raw(raw_pid)
        .is_some_and(|pid| rustix::process::test_kill_process(pid).is_ok())
}

#[cfg(all(windows, feature = "windows-acp"))]
fn synthetic_process_is_running(pid: u32, _raw_pid: &str) -> bool {
    subc_jobobject::process_exists(pid)
}

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
#[tokio::test]
async fn unsupported_protocol_version_fails_closed() -> Result<(), Box<dyn Error + Send + Sync>> {
    let _test_guard = SYNTHETIC_PEER_TEST_LOCK.lock().await;
    let temporary = tempfile::tempdir()?;
    let manager = AgentSessionManager::default();
    let profile = AgentProfileV1 {
        name: "HyperAI wrong-version synthetic ACP peer".to_owned(),
        executable: PathBuf::from(env!("CARGO_BIN_EXE_acp-fixture-peer")),
        args: vec!["--wrong-version".to_owned()],
        working_directory: temporary.path().to_path_buf(),
        environment: BTreeMap::default(),
    };
    let mcp = AgentMcpConfigV1 {
        endpoint: "http://127.0.0.1:4317".to_owned(),
        capability: "synthetic-read-only-capability-0123456789".to_owned(),
        executable: temporary.path().join("hyperai-mcp-fixture"),
    };

    assert_eq!(
        manager.connect(profile, mcp).await,
        Err(hyperai_acp::SessionError::VersionMismatch)
    );
    Ok(())
}

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
#[tokio::test]
async fn malformed_and_batched_agent_frames_fail_the_handshake()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let _test_guard = SYNTHETIC_PEER_TEST_LOCK.lock().await;
    for mode in ["--malformed-on-initialize", "--batch-on-initialize"] {
        let temporary = tempfile::tempdir()?;
        let manager = AgentSessionManager::default();
        let profile = AgentProfileV1 {
            name: "HyperAI malformed-frame synthetic ACP peer".to_owned(),
            executable: PathBuf::from(env!("CARGO_BIN_EXE_acp-fixture-peer")),
            args: vec![mode.to_owned()],
            working_directory: temporary.path().to_path_buf(),
            environment: BTreeMap::default(),
        };
        let mcp = AgentMcpConfigV1 {
            endpoint: "http://127.0.0.1:4317".to_owned(),
            capability: "synthetic-read-only-capability-0123456789".to_owned(),
            executable: temporary.path().join("hyperai-mcp-fixture"),
        };

        let result =
            tokio::time::timeout(Duration::from_secs(5), manager.connect(profile, mcp)).await?;
        if result.is_ok() {
            let _ = manager.disconnect().await;
            return Err(format!("ACP accepted invalid fixture frame for mode {mode}").into());
        }
    }
    Ok(())
}

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
#[tokio::test]
async fn stdout_overflow_fails_the_handshake_within_the_session_budget()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let _test_guard = SYNTHETIC_PEER_TEST_LOCK.lock().await;
    let temporary = tempfile::tempdir()?;
    let manager = AgentSessionManager::default();
    let profile = AgentProfileV1 {
        name: "HyperAI stdout-overflow synthetic ACP peer".to_owned(),
        executable: PathBuf::from(env!("CARGO_BIN_EXE_acp-fixture-peer")),
        args: vec!["--stdout-overflow-on-initialize".to_owned()],
        working_directory: temporary.path().to_path_buf(),
        environment: BTreeMap::default(),
    };
    let mcp = AgentMcpConfigV1 {
        endpoint: "http://127.0.0.1:4317".to_owned(),
        capability: "synthetic-read-only-capability-0123456789".to_owned(),
        executable: temporary.path().join("hyperai-mcp-fixture"),
    };

    let result =
        tokio::time::timeout(Duration::from_secs(5), manager.connect(profile, mcp)).await?;
    if result.is_ok() {
        let _ = manager.disconnect().await;
        return Err("ACP accepted a peer exceeding the stdout byte ceiling".into());
    }
    Ok(())
}

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
#[tokio::test]
async fn stderr_overflow_fails_the_session_with_a_sanitized_status()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let _test_guard = SYNTHETIC_PEER_TEST_LOCK.lock().await;
    let temporary = tempfile::tempdir()?;
    let manager = AgentSessionManager::default();
    let profile = AgentProfileV1 {
        name: "HyperAI stderr-overflow synthetic ACP peer".to_owned(),
        executable: PathBuf::from(env!("CARGO_BIN_EXE_acp-fixture-peer")),
        args: vec!["--stderr-overflow-on-prompt".to_owned()],
        working_directory: temporary.path().to_path_buf(),
        environment: BTreeMap::default(),
    };
    let mcp = AgentMcpConfigV1 {
        endpoint: "http://127.0.0.1:4317".to_owned(),
        capability: "synthetic-read-only-capability-0123456789".to_owned(),
        executable: temporary.path().join("hyperai-mcp-fixture"),
    };

    manager.connect(profile, mcp).await?;
    let _ = tokio::time::timeout(
        Duration::from_secs(5),
        manager.prompt("Trigger the bounded synthetic stderr fixture.".to_owned()),
    )
    .await?;
    let snapshot = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let snapshot = manager.snapshot().await;
            if snapshot.status != "running" {
                break snapshot;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?;
    assert_eq!(snapshot.status, "failed");
    assert_eq!(snapshot.error_code.as_deref(), Some("ACP_STDERR_LIMIT"));
    assert!(!serde_json::to_string(&snapshot)?.contains("ACP_STDERR_CANARY_DO_NOT_RETAIN"));
    let disconnected = manager.disconnect().await?;
    assert_eq!(disconnected.status, "failed");
    Ok(())
}
