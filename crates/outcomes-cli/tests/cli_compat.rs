//! Executable-level compatibility checks for the `hyperai` and `hsai` aliases.

use std::{
    error::Error,
    fs,
    net::TcpListener,
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

use serde_json::Value;

struct ServerProcess(Child);

impl Drop for ServerProcess {
    fn drop(&mut self) {
        if self.0.try_wait().is_ok_and(|status| status.is_none()) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

struct InstalledServiceGuard<'a> {
    executable: &'a std::path::Path,
    home: &'a str,
    active: bool,
}

impl InstalledServiceGuard<'_> {
    fn stop(&mut self) -> Result<Value, Box<dyn Error>> {
        let output = Command::new(self.executable)
            .args(["service", "stop", "--home", self.home])
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "installed service stop failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        let report = serde_json::from_slice(&output.stdout)?;
        self.active = false;
        Ok(report)
    }
}

impl Drop for InstalledServiceGuard<'_> {
    fn drop(&mut self) {
        if self.active {
            let _ = Command::new(self.executable)
                .args(["service", "stop", "--home", self.home])
                .output();
        }
    }
}

fn installed_cli_json(
    executable: &std::path::Path,
    arguments: &[&str],
) -> Result<Value, Box<dyn Error>> {
    let output = Command::new(executable).args(arguments).output()?;
    if !output.status.success() {
        return Err(format!(
            "installed HyperAI command {arguments:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn exercise_installed_cli_lifecycle(
    executable: &std::path::Path,
    home: &std::path::Path,
) -> Result<(), Box<dyn Error>> {
    let version = Command::new(executable).arg("--version").output()?;
    assert!(
        version.status.success(),
        "installed HyperAI --version failed: {}",
        String::from_utf8_lossy(&version.stderr)
    );
    assert!(String::from_utf8_lossy(&version.stdout).starts_with("hyperai "));

    let home_arg = home.to_str().ok_or("installed home path was not UTF-8")?;
    let initialized = installed_cli_json(executable, &["init", "--home", home_arg])?;
    assert_eq!(
        initialized["schema_version"],
        "hyperai.local-home-report.v1"
    );
    assert_eq!(initialized["initialized"], true);

    let initial_doctor = installed_cli_json(executable, &["doctor", "--home", home_arg])?;
    assert_eq!(initial_doctor["status"], "incomplete");
    assert_eq!(initial_doctor["service"]["status"], "stopped");

    let mut service = InstalledServiceGuard {
        executable,
        home: home_arg,
        active: true,
    };
    let started = installed_cli_json(
        executable,
        &["service", "start", "--home", home_arg, "--port", "0"],
    )?;
    assert_eq!(started["status"], "started");
    assert_eq!(started["service"]["ready"], true);
    let address = started["service"]["address"]
        .as_str()
        .ok_or("installed service start omitted its address")?;
    assert!(address.starts_with("127.0.0.1:"));

    let status = installed_cli_json(executable, &["service", "status", "--home", home_arg])?;
    let connected = installed_cli_json(executable, &["connect", "--home", home_arg])?;
    assert_eq!(status["status"], "ready");
    assert_eq!(connected["status"], "ready");
    assert_eq!(status["address"], address);
    assert_eq!(connected["address"], address);
    assert_eq!(status["instance_id"], started["service"]["instance_id"]);
    assert_eq!(connected["instance_id"], started["service"]["instance_id"]);

    let connections =
        installed_cli_json(executable, &["connections", "--json", "--home", home_arg])?;
    assert_eq!(
        connections["schema_version"],
        "hyperai.interface.connection-inventory.v1"
    );
    assert_eq!(
        connections["connections"].as_array().map(Vec::len),
        Some(11)
    );
    assert_eq!(connections["connections"][2]["module_id"], "compute_bazaar");
    assert_eq!(
        connections["connections"][2]["connection_mode"],
        "reference_only"
    );
    assert_eq!(
        connections["connections"][2]["exposed_capability_ids"],
        serde_json::json!([])
    );

    let ready_doctor = installed_cli_json(executable, &["doctor", "--home", home_arg])?;
    assert_eq!(ready_doctor["status"], "ready");
    assert_eq!(
        ready_doctor["service"]["instance_id"],
        status["instance_id"]
    );
    assert_eq!(ready_doctor["checks"][2]["name"], "external_effects");
    assert_eq!(ready_doctor["checks"][2]["status"], "closed");

    exercise_installed_market_quote_lifecycle(executable, home)?;

    let stopped = service.stop()?;
    assert_eq!(stopped["status"], "shutdown_requested");
    let status = installed_cli_json(executable, &["service", "status", "--home", home_arg])?;
    assert_eq!(status["status"], "stopped");
    assert_eq!(status["ready"], false);
    Ok(())
}

fn exercise_installed_compatibility_entrypoints(
    hyperai: &std::path::Path,
    hsai: &std::path::Path,
    hyperai_ui: &std::path::Path,
    temporary_root: &std::path::Path,
) -> Result<(), Box<dyn Error>> {
    let hyperai_version = Command::new(hyperai).arg("--version").output()?;
    let hsai_version = Command::new(hsai).arg("--version").output()?;
    assert!(hyperai_version.status.success());
    assert!(hsai_version.status.success());
    assert_eq!(hyperai_version.stdout, hsai_version.stdout);

    let hyperai_demo = installed_cli_json(hyperai, &["market", "demo"])?;
    let hsai_demo = installed_cli_json(hsai, &["market", "demo"])?;
    assert_eq!(hyperai_demo, hsai_demo);
    assert_eq!(
        hyperai_demo["schema_version"],
        "hyperai.compute-market.local-demo.v1"
    );

    let home = temporary_root.join("installed-compatibility-home");
    let home_arg = home
        .to_str()
        .ok_or("compatibility home path was not UTF-8")?;
    let port_reservation = TcpListener::bind(("127.0.0.1", 0))?;
    let port = port_reservation.local_addr()?.port().to_string();
    drop(port_reservation);
    assert_ne!(
        port, "4317",
        "test launcher must not use the shared service port"
    );

    let child = Command::new(hyperai_ui)
        .args(["--port", &port, "--home", home_arg])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let mut process = ServerProcess(child);
    let record = wait_for_server(&home, &mut process.0)?;
    let address = record["address"]
        .as_str()
        .ok_or("installed hyperai-ui omitted its service address")?;
    assert!(address.starts_with("127.0.0.1:"));

    let status = installed_cli_json(hyperai, &["service", "status", "--home", home_arg])?;
    let connected = installed_cli_json(hyperai, &["connect", "--home", home_arg])?;
    assert_eq!(status["status"], "ready");
    assert_eq!(connected["status"], "ready");
    assert_eq!(status["address"], address);
    assert_eq!(connected["address"], address);
    assert_eq!(status["instance_id"], record["instance_id"]);
    assert_eq!(connected["instance_id"], record["instance_id"]);

    let stopped = installed_cli_json(hyperai, &["service", "stop", "--home", home_arg])?;
    assert_eq!(stopped["status"], "shutdown_requested");
    let final_status = installed_cli_json(hyperai, &["service", "status", "--home", home_arg])?;
    assert_eq!(final_status["status"], "stopped");
    let process_status = process.0.wait()?;
    assert!(process_status.success());
    Ok(())
}

fn exercise_installed_tui_pty(
    executable: &std::path::Path,
    market_quote_lifecycle: bool,
) -> Result<(), Box<dyn Error>> {
    let harness = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/verify_hyperai_tui_ctrl_c.py");
    let mut command = Command::new("python3");
    command.arg(harness).arg("--binary").arg(executable);
    if market_quote_lifecycle {
        command.arg("--market-quote-lifecycle");
    }
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "installed HyperAI TUI resize/restoration PTY check failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let report: Value = serde_json::from_slice(&output.stdout)?;
    println!("Installed HyperAI TUI PTY report: {report}");
    assert_eq!(report["alternate_screen_entered"], true);
    assert_eq!(report["raw_mode_entered"], true);
    assert_eq!(report["ctrl_c_exit"], 0);
    assert_eq!(report["termios_restored"], true);
    assert_eq!(report["alternate_screen_left"], true);
    assert_eq!(report["cursor_restored"], true);
    assert_eq!(report["temporary_home_removed"], true);
    let resize_checks = report["resize_checks"]
        .as_array()
        .ok_or("installed HyperAI TUI report omitted resize checks")?;
    assert_eq!(resize_checks.len(), 2);
    for (check, (columns, rows, navigation_events)) in
        resize_checks.iter().zip([(48, 10, 11), (100, 30, 1)])
    {
        assert_eq!(check["columns"], columns);
        assert_eq!(check["rows"], rows);
        assert_eq!(check["navigation_events"], navigation_events);
        assert!(
            check["redraw_bytes"]
                .as_u64()
                .is_some_and(|bytes| bytes > 0)
        );
        assert!(
            check["maximum_cursor_row"]
                .as_u64()
                .is_some_and(|row| row <= rows)
        );
    }
    assert!(
        resize_checks[1]["maximum_cursor_row"]
            .as_u64()
            .is_some_and(|row| row > 10)
    );
    if market_quote_lifecycle {
        let flow = &report["market_quote_lifecycle"];
        for field in [
            "overview_rendered",
            "market_selected",
            "preview_rendered",
            "approval_rendered",
            "receipt_rendered",
            "receipt_digest_displayed",
            "synthetic_effect_boundary_displayed",
        ] {
            assert_eq!(flow[field], true, "installed TUI did not prove {field}");
        }
    }
    assert!(
        report["transcript_bytes"]
            .as_u64()
            .is_some_and(|bytes| bytes > 0)
    );
    for field in ["transcript_sha256", "binary_sha256"] {
        let digest = report[field]
            .as_str()
            .ok_or_else(|| format!("installed HyperAI TUI report omitted {field}"))?;
        assert_eq!(digest.len(), 64);
    }
    Ok(())
}

fn wait_for_server(home: &std::path::Path, process: &mut Child) -> Result<Value, Box<dyn Error>> {
    let record_path = home.join("runtime").join("service.json");
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_millis(300))
        .build()?;
    for _ in 0..400 {
        if let Ok(bytes) = fs::read(&record_path) {
            let record: Value = serde_json::from_slice(&bytes)?;
            if let Some(address) = record["address"].as_str()
                && client
                    .get(format!("http://{address}/healthz"))
                    .send()
                    .is_ok_and(|response| response.status().is_success())
            {
                return Ok(record);
            }
        }
        if let Some(status) = process.try_wait()? {
            return Err(format!("service process exited before readiness: {status}").into());
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err("service process did not become ready".into())
}

fn assert_bound_market_trace_cli_matches_service(
    client: &reqwest::blocking::Client,
    base: &str,
    bearer: &str,
    data_root_arg: &str,
) -> Result<(), Box<dyn Error>> {
    let preview: Value = client
        .get(format!("{base}/api/v1/data/synthetic/preview"))
        .bearer_auth(bearer)
        .send()?
        .error_for_status()?
        .json()?;
    client
        .post(format!("{base}/api/v1/data/synthetic/seed"))
        .bearer_auth(bearer)
        .header("Idempotency-Key", "cli-trace-parity-seed-v1")
        .json(&serde_json::json!({
            "preview_digest": preview["preview_digest"],
        }))
        .send()?
        .error_for_status()?;

    let response = client
        .get(format!("{base}/api/v1/market/workload-trace"))
        .bearer_auth(bearer)
        .send()?;
    assert!(response.status().is_success());
    let service_trace: Value = response.json()?;
    let data_reference = service_trace["references"]
        .as_array()
        .and_then(|references| {
            references
                .iter()
                .find(|reference| reference["owner_id"] == "hyperai-data")
        })
        .ok_or("bound service trace omitted the Data context reference")?;
    assert_eq!(data_reference["status"], "bound");
    let bound_digest = data_reference["digest"]
        .as_str()
        .ok_or("bound service trace omitted the verified Data digest")?;
    assert_eq!(
        service_trace["intent"]["body"]["data_references"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        service_trace["intent"]["body"]["data_references"][0]["digest"],
        bound_digest
    );

    let output = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["market", "trace", "--data-root", data_root_arg])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "bound market trace CLI failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let cli_trace: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(cli_trace, service_trace);
    Ok(())
}

fn assert_unbound_market_trace_cli_matches_service(
    client: &reqwest::blocking::Client,
    base: &str,
    bearer: &str,
    data_root_arg: &str,
    expected_status: &str,
) -> Result<Value, Box<dyn Error>> {
    let response = client
        .get(format!("{base}/api/v1/market/workload-trace"))
        .bearer_auth(bearer)
        .send()?;
    assert!(response.status().is_success());
    let service_trace: Value = response.json()?;
    let data_reference = service_trace["references"]
        .as_array()
        .and_then(|references| {
            references
                .iter()
                .find(|reference| reference["owner_id"] == "hyperai-data")
        })
        .ok_or("unbound service trace omitted the Data context reference")?;
    assert_eq!(data_reference["status"], expected_status);
    assert!(data_reference["digest"].is_null());
    assert_eq!(
        service_trace["intent"]["body"]["data_references"],
        serde_json::json!([])
    );

    let output = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["market", "trace", "--data-root", data_root_arg])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "market trace CLI failed for {expected_status}: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let cli_trace: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(cli_trace, service_trace);
    Ok(service_trace)
}

#[cfg(unix)]
fn assert_permission_limited_market_trace_cli_matches_service(
    client: &reqwest::blocking::Client,
    base: &str,
    bearer: &str,
    data_root_arg: &str,
    sample: &std::path::Path,
) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(sample)?.permissions();
    let original_mode = permissions.mode();
    permissions.set_mode(original_mode & !0o444);
    fs::set_permissions(sample, permissions)?;

    let direct_read = fs::read(sample);
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
    let parity = if denied {
        assert_unbound_market_trace_cli_matches_service(
            client,
            base,
            bearer,
            data_root_arg,
            "denied",
        )
        .map(|_| ())
    } else {
        assert_bound_market_trace_cli_matches_service(client, base, bearer, data_root_arg)
    };

    let mut restored = fs::metadata(sample)?.permissions();
    restored.set_mode(original_mode);
    fs::set_permissions(sample, restored)?;
    parity
}

fn assert_seller_failure_preview_cli_matches_service(
    client: &reqwest::blocking::Client,
    base: &str,
    bearer: &str,
    data_root_arg: &str,
) -> Result<(), Box<dyn Error>> {
    let response = client
        .get(format!(
            "{base}/api/v1/market/workload-trace/failure-preview"
        ))
        .bearer_auth(bearer)
        .send()?;
    assert!(response.status().is_success());
    let service_failure_trace: Value = response.json()?;
    let output = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args([
            "market",
            "trace",
            "--seller-failure-preview",
            "--data-root",
            data_root_arg,
        ])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "seller-failure preview CLI failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let cli_failure_trace: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(cli_failure_trace, service_failure_trace);
    Ok(())
}

fn assert_inconclusive_preview_cli_matches_service(
    client: &reqwest::blocking::Client,
    base: &str,
    bearer: &str,
    data_root_arg: &str,
) -> Result<(), Box<dyn Error>> {
    let response = client
        .get(format!(
            "{base}/api/v1/market/workload-trace/inconclusive-preview"
        ))
        .bearer_auth(bearer)
        .send()?;
    assert!(response.status().is_success());
    let service_trace: Value = response.json()?;
    let output = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args([
            "market",
            "trace",
            "--inconclusive-preview",
            "--data-root",
            data_root_arg,
        ])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "inconclusive preview CLI failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let cli_trace: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(cli_trace, service_trace);

    let conflicting_flags = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args([
            "market",
            "trace",
            "--seller-failure-preview",
            "--inconclusive-preview",
        ])
        .output()?;
    assert!(!conflicting_flags.status.success());
    Ok(())
}

#[test]
fn hyperai_and_hsai_keep_market_demo_json_compatibility() -> Result<(), Box<dyn Error>> {
    let hyperai = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["market", "demo"])
        .output()?;
    let hsai = Command::new(env!("CARGO_BIN_EXE_hsai"))
        .args(["market", "demo"])
        .output()?;
    assert!(hyperai.status.success());
    assert!(hsai.status.success());
    let hyperai_json: Value = serde_json::from_slice(&hyperai.stdout)?;
    let hsai_json: Value = serde_json::from_slice(&hsai.stdout)?;
    assert_eq!(hyperai_json, hsai_json);
    assert_eq!(
        hyperai_json["schema_version"],
        "hyperai.compute-market.local-demo.v1"
    );
    Ok(())
}

#[test]
fn market_trace_command_returns_the_bound_synthetic_projection() -> Result<(), Box<dyn Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["market", "trace"])
        .output()?;
    assert!(output.status.success());
    let trace: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        trace["schema_version"],
        "hyperai.interface.workload-trace.v1"
    );
    assert_eq!(
        trace["state_slice"],
        "hyperai.bazaar.workload_trace_synthetic.v1"
    );
    assert_eq!(
        trace["intent"]["body"]["workload_id"],
        "workload-hyperai-local-synthetic-v1"
    );
    assert_eq!(trace["digest"].as_str().map(str::len), Some(64));
    assert!(trace["references"].as_array().is_some_and(|references| {
        references.iter().any(|reference| {
            reference["owner_id"] == "hyperai-market"
                && reference["status"] == "mock_verdict_verified"
        }) && references.iter().any(|reference| {
            reference["owner_id"] == "hyperai-market"
                && reference["status"] == "mock_disposition_release_to_seller"
        }) && references.iter().any(|reference| {
            reference["owner_id"] == "hyperai-trade"
                && reference["status"] == "advisory_market_data_only"
        })
    }));
    Ok(())
}

#[test]
fn market_trace_cli_matches_the_authenticated_service_projection() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("hyperai-home");
    let home_arg = home.to_str().ok_or("temporary home was not UTF-8")?;
    let child = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["__serve", "--home", home_arg, "--port", "0"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let mut process = ServerProcess(child);
    let record = wait_for_server(&home, &mut process.0)?;
    let address = record["address"]
        .as_str()
        .ok_or("service record has no address")?;
    let base = format!("http://{address}");
    let bearer = fs::read_to_string(home.join("runtime").join("bearer.token"))?;
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()?;

    let response = client
        .get(format!("{base}/api/v1/market/workload-trace"))
        .bearer_auth(&bearer)
        .send()?;
    assert!(response.status().is_success());
    let service_trace: Value = response.json()?;
    let data_reference = service_trace["references"]
        .as_array()
        .and_then(|references| {
            references
                .iter()
                .find(|reference| reference["owner_id"] == "hyperai-data")
        })
        .ok_or("service trace omitted the Data context reference")?;
    assert_eq!(
        service_trace["intent"]["body"]["data_references"],
        serde_json::json!([])
    );
    assert_eq!(data_reference["status"], "not_seeded");
    assert!(data_reference["digest"].is_null());

    let data_root = home.join("data");
    let data_root_arg = data_root.to_str().ok_or("data root was not UTF-8")?;
    let output = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["market", "trace", "--data-root", data_root_arg])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "market trace CLI failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let cli_trace: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(cli_trace, service_trace);

    assert_seller_failure_preview_cli_matches_service(&client, &base, &bearer, data_root_arg)?;
    assert_inconclusive_preview_cli_matches_service(&client, &base, &bearer, data_root_arg)?;

    assert_bound_market_trace_cli_matches_service(&client, &base, &bearer, data_root_arg)?;

    let sample = data_root.join("synthetic-market-observations.json");
    #[cfg(unix)]
    assert_permission_limited_market_trace_cli_matches_service(
        &client,
        &base,
        &bearer,
        data_root_arg,
        &sample,
    )?;

    let mut sample_bytes = fs::read(&sample)?;
    sample_bytes.push(b' ');
    fs::write(&sample, &sample_bytes)?;
    assert_unbound_market_trace_cli_matches_service(
        &client,
        &base,
        &bearer,
        data_root_arg,
        "conflicting",
    )?;
    fs::remove_file(sample)?;
    assert_unbound_market_trace_cli_matches_service(
        &client,
        &base,
        &bearer,
        data_root_arg,
        "missing",
    )?;

    let oversized_manifest = data_root.join("manifests/unavailable-test-manifest.json");
    fs::write(&oversized_manifest, vec![b'x'; 16 * 1024 + 1])?;
    let unavailable_trace = assert_unbound_market_trace_cli_matches_service(
        &client,
        &base,
        &bearer,
        data_root_arg,
        "unavailable",
    )?;
    assert!(
        !unavailable_trace
            .to_string()
            .contains("unavailable-test-manifest.json")
    );

    stop_test_service(&client, &base, &bearer, &mut process)?;
    Ok(())
}

#[test]
fn market_compare_cli_matches_the_authenticated_service_projection() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("hyperai-home");
    let home_arg = home.to_str().ok_or("temporary home was not UTF-8")?;
    let child = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["__serve", "--home", home_arg, "--port", "0"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let mut process = ServerProcess(child);
    let record = wait_for_server(&home, &mut process.0)?;
    let address = record["address"]
        .as_str()
        .ok_or("service record has no address")?;
    let base = format!("http://{address}");
    let bearer = fs::read_to_string(home.join("runtime").join("bearer.token"))?;
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()?;

    let response = client
        .get(format!("{base}/api/v1/market/supply-comparison"))
        .bearer_auth(&bearer)
        .send()?;
    assert!(response.status().is_success());
    let service_comparison: Value = response.json()?;
    assert_eq!(
        service_comparison["body"]["schema_version"],
        "hyperai.compute-market.supply-comparison.v1"
    );

    let output = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["market", "compare", "--home", home_arg])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "market compare CLI failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let cli_comparison: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(cli_comparison, service_comparison);

    let comparison_digest = service_comparison["digest"]
        .as_str()
        .ok_or("service comparison has no digest")?;
    let executable = std::path::Path::new(env!("CARGO_BIN_EXE_hyperai"));
    let cli_quote_preview = run_market_quote_preview(executable, home_arg, comparison_digest)?;
    let preview_digest = cli_quote_preview["body"]["quote_preview"]["digest"]
        .as_str()
        .ok_or("quote preview had no nested preview digest")?;
    let stale_digest = "0".repeat(64);
    let deal_digest =
        run_market_quote_approval(executable, home_arg, preview_digest, &stale_digest)?;
    run_market_quote_completion(
        executable,
        home_arg,
        preview_digest,
        &deal_digest,
        &stale_digest,
    )?;

    stop_test_service(&client, &base, &bearer, &mut process)?;
    Ok(())
}

#[test]
#[allow(clippy::too_many_lines)] // One disposable-service lifecycle proves the complete CLI history contract.
fn market_history_cli_returns_the_verified_authenticated_service_page() -> Result<(), Box<dyn Error>>
{
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("hyperai-home");
    let home_arg = home.to_str().ok_or("temporary home was not UTF-8")?;
    let child = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["__serve", "--home", home_arg, "--port", "0"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let mut process = ServerProcess(child);
    let record = wait_for_server(&home, &mut process.0)?;
    let address = record["address"]
        .as_str()
        .ok_or("service record has no address")?;
    let base = format!("http://{address}");
    let bearer = fs::read_to_string(home.join("runtime").join("bearer.token"))?;
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()?;

    let comparison = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["market", "compare", "--home", home_arg])
        .output()?;
    assert!(comparison.status.success());
    let comparison: Value = serde_json::from_slice(&comparison.stdout)?;
    let comparison_digest = comparison["digest"]
        .as_str()
        .ok_or("comparison omitted its digest")?;
    let preview = run_market_quote_preview(
        std::path::Path::new(env!("CARGO_BIN_EXE_hyperai")),
        home_arg,
        comparison_digest,
    )?;
    let preview_digest = preview["body"]["quote_preview"]["digest"]
        .as_str()
        .ok_or("quote preview omitted its digest")?;
    let deal_digest = run_market_quote_approval(
        std::path::Path::new(env!("CARGO_BIN_EXE_hyperai")),
        home_arg,
        preview_digest,
        &"0".repeat(64),
    )?;
    run_market_quote_completion(
        std::path::Path::new(env!("CARGO_BIN_EXE_hyperai")),
        home_arg,
        preview_digest,
        &deal_digest,
        &"0".repeat(64),
    )?;

    let alternate_preview: Value = client
        .post(format!(
            "{base}/api/v1/market/quote-preview/scenario/alternate"
        ))
        .bearer_auth(&bearer)
        .json(&serde_json::json!({}))
        .send()?
        .error_for_status()?
        .json()?;
    let alternate_preview_digest = alternate_preview["digest"]
        .as_str()
        .ok_or("alternate preview omitted its digest")?;
    assert_ne!(alternate_preview_digest, preview_digest);
    let alternate_deal_digest = run_market_quote_approval(
        std::path::Path::new(env!("CARGO_BIN_EXE_hyperai")),
        home_arg,
        alternate_preview_digest,
        &"0".repeat(64),
    )?;
    run_market_quote_completion(
        std::path::Path::new(env!("CARGO_BIN_EXE_hyperai")),
        home_arg,
        alternate_preview_digest,
        &alternate_deal_digest,
        &"0".repeat(64),
    )?;

    let response = client
        .get(format!("{base}/api/v1/market/deal-history?limit=1"))
        .bearer_auth(&bearer)
        .send()?;
    assert!(response.status().is_success());
    let service_first_page: Value = response.json()?;
    assert_eq!(
        service_first_page["body"]["entries"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        service_first_page["body"]["entries"][0]["approval"]["deal_digest"],
        alternate_deal_digest
    );
    assert_eq!(
        service_first_page["body"]["entries"][0]["completion_receipt"]["digest"]
            .as_str()
            .map(str::len),
        Some(64)
    );
    let service_cursor = service_first_page["body"]["next_cursor"]
        .as_str()
        .filter(|cursor| !cursor.is_empty())
        .ok_or("first history page omitted its continuation cursor")?;
    let mut second_page_url = reqwest::Url::parse(&format!("{base}/api/v1/market/deal-history"))?;
    second_page_url
        .query_pairs_mut()
        .append_pair("limit", "1")
        .append_pair("cursor", service_cursor);
    let service_second_page: Value = client
        .get(second_page_url)
        .bearer_auth(&bearer)
        .send()?
        .error_for_status()?
        .json()?;
    assert_eq!(
        service_second_page["body"]["entries"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        service_second_page["body"]["entries"][0]["approval"]["deal_digest"],
        deal_digest
    );
    assert!(service_second_page["body"]["next_cursor"].is_null());
    assert_ne!(service_first_page["digest"], service_second_page["digest"]);
    assert!(
        service_first_page["body"]["entries"][0]["sequence"].as_u64()
            > service_second_page["body"]["entries"][0]["sequence"].as_u64()
    );

    let output = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["market", "history", "--limit", "1", "--home", home_arg])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "market history CLI failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let cli_first_page: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(cli_first_page, service_first_page);
    assert_eq!(
        cli_first_page["body"]["schema_version"],
        "hyperai.compute-market.local-history-page.v1"
    );
    assert_eq!(
        cli_first_page["body"]["entries"][0]["approval"]["deal_digest"],
        alternate_deal_digest
    );
    let cli_cursor = cli_first_page["body"]["next_cursor"]
        .as_str()
        .ok_or("CLI first page omitted its opaque cursor")?;
    assert_eq!(cli_cursor, service_cursor);
    let output = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args([
            "market", "history", "--limit", "1", "--cursor", cli_cursor, "--home", home_arg,
        ])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "market history second page failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let cli_second_page: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(cli_second_page, service_second_page);
    assert_eq!(
        cli_second_page["body"]["entries"][0]["approval"]["deal_digest"],
        deal_digest
    );
    assert!(cli_second_page["body"]["next_cursor"].is_null());

    let bounded = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["market", "history", "--limit", "1", "--home", home_arg])
        .output()?;
    assert!(bounded.status.success());
    let bounded_page: Value = serde_json::from_slice(&bounded.stdout)?;
    assert_eq!(bounded_page, service_first_page);

    let rejected_limit = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["market", "history", "--limit", "51", "--home", home_arg])
        .output()?;
    assert!(!rejected_limit.status.success());
    let rejection: Value = serde_json::from_slice(&rejected_limit.stderr)?;
    assert_eq!(rejection["error_code"], "MARKET_HISTORY_INVALID_QUERY");

    let rejected = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args([
            "market",
            "history",
            "--cursor",
            "invalid-cursor",
            "--home",
            home_arg,
        ])
        .output()?;
    assert!(!rejected.status.success());
    let rejection: Value = serde_json::from_slice(&rejected.stderr)?;
    assert_eq!(rejection["error_code"], "MARKET_HISTORY_INVALID_QUERY");

    stop_test_service(&client, &base, &bearer, &mut process)?;
    Ok(())
}

fn run_market_quote_preview(
    executable: &std::path::Path,
    home: &str,
    comparison_digest: &str,
) -> Result<Value, Box<dyn Error>> {
    let output = Command::new(executable)
        .args([
            "market",
            "quote-preview",
            "--comparison-digest",
            comparison_digest,
            "--home",
            home,
        ])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "market quote-preview CLI failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let preview: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        preview["body"]["schema_version"],
        "hyperai.compute-market.supply-quote-preview.v1"
    );
    assert_eq!(preview["body"]["selected_offer_id"], "offer-gpu-b");
    assert_eq!(preview["body"]["comparison_digest"], comparison_digest);
    assert_eq!(
        preview["body"]["quote_preview"]["body"]["selected_offer_id"],
        "offer-gpu-b"
    );
    Ok(preview)
}

fn run_market_quote_approval(
    executable: &std::path::Path,
    home: &str,
    preview_digest: &str,
    stale_digest: &str,
) -> Result<String, Box<dyn Error>> {
    let rejected = Command::new(executable)
        .args([
            "market",
            "quote-approve",
            "--preview-digest",
            stale_digest,
            "--home",
            home,
        ])
        .output()?;
    assert!(!rejected.status.success());
    let rejected: Value = serde_json::from_slice(&rejected.stderr)?;
    assert_eq!(rejected["error_code"], "QUOTE_PREVIEW_CHANGED");

    let output = Command::new(executable)
        .args([
            "market",
            "quote-approve",
            "--preview-digest",
            preview_digest,
            "--home",
            home,
        ])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "market quote-approve CLI failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let approval: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        approval["schema_version"],
        "hyperai.compute-market.local-quote-approval.v1"
    );
    assert_eq!(approval["preview_digest"], preview_digest);
    assert!(approval["effects"].as_str().is_some_and(|effects| {
        effects.contains("synthetic in-memory approval only")
            && effects.contains("no fulfillment")
            && effects.contains("funds effect")
    }));
    approval["deal_digest"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "quote approval had no deal digest".into())
}

fn run_market_quote_completion(
    executable: &std::path::Path,
    home: &str,
    preview_digest: &str,
    deal_digest: &str,
    stale_digest: &str,
) -> Result<(), Box<dyn Error>> {
    let rejected = Command::new(executable)
        .args([
            "market",
            "quote-complete",
            "--preview-digest",
            preview_digest,
            "--deal-digest",
            stale_digest,
            "--home",
            home,
        ])
        .output()?;
    assert!(!rejected.status.success());
    let rejected: Value = serde_json::from_slice(&rejected.stderr)?;
    assert_eq!(rejected["error_code"], "MARKET_DEAL_NOT_APPROVED");

    let output = Command::new(executable)
        .args([
            "market",
            "quote-complete",
            "--preview-digest",
            preview_digest,
            "--deal-digest",
            deal_digest,
            "--home",
            home,
        ])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "market quote-complete CLI failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let completion: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        completion["schema_version"],
        "hyperai.interface.market-quote-completion.v1"
    );
    assert_eq!(completion["preview_digest"], preview_digest);
    assert_eq!(completion["deal_digest"], deal_digest);
    assert_eq!(completion["demo"]["report"]["deal_digest"], deal_digest);
    assert_eq!(
        completion["effect_boundary"],
        "local_synthetic_mock_fulfillment_verifier_and_settlement_only"
    );
    assert_eq!(
        completion["demo"]["receipt"]["digest"]
            .as_str()
            .map(str::len),
        Some(64)
    );
    assert!(
        completion["demo"]["receipt"]["body"]["effects"]
            .as_str()
            .is_some_and(|effects| {
                effects.contains("no network") && effects.contains("or funds moved")
            })
    );
    Ok(())
}

fn exercise_installed_market_quote_lifecycle(
    executable: &std::path::Path,
    home: &std::path::Path,
) -> Result<(), Box<dyn Error>> {
    let home = home.to_str().ok_or("installed home path was not UTF-8")?;
    let comparison = installed_cli_json(executable, &["market", "compare", "--home", home])?;
    assert_eq!(
        comparison["body"]["schema_version"],
        "hyperai.compute-market.supply-comparison.v1"
    );
    let comparison_digest = comparison["digest"]
        .as_str()
        .ok_or("installed comparison had no digest")?;
    let preview = run_market_quote_preview(executable, home, comparison_digest)?;
    let preview_digest = preview["body"]["quote_preview"]["digest"]
        .as_str()
        .ok_or("installed quote preview had no nested digest")?;
    let stale_digest = "0".repeat(64);
    let deal_digest = run_market_quote_approval(executable, home, preview_digest, &stale_digest)?;
    run_market_quote_completion(
        executable,
        home,
        preview_digest,
        &deal_digest,
        &stale_digest,
    )
}

#[test]
fn hyperai_and_hsai_keep_storage_backup_restore_json_contract() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("hyperai-home");
    let home_arg = home.to_str().ok_or("temporary home was not UTF-8")?;
    let initialized = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["init", "--home", home_arg])
        .output()?;
    assert!(initialized.status.success());

    let backup = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["storage", "backup", "--home", home_arg])
        .output()?;
    assert!(backup.status.success());
    let backup_report: Value = serde_json::from_slice(&backup.stdout)?;
    assert_eq!(
        backup_report["schema_version"],
        "hyperai.local-storage-backup.v1"
    );
    assert_eq!(backup_report["status"], "created");
    assert_eq!(backup_report["database_schema_version"], 1);
    assert_eq!(
        backup_report["backup_sha256"].as_str().map(str::len),
        Some(64)
    );
    assert!(backup_report["home"].is_null());

    let backup_name = backup_report["backup_name"]
        .as_str()
        .ok_or("backup report omitted its basename")?;
    let restored = Command::new(env!("CARGO_BIN_EXE_hsai"))
        .args([
            "storage",
            "restore",
            "--home",
            home_arg,
            "--backup-name",
            backup_name,
        ])
        .output()?;
    assert!(restored.status.success());
    let restore_report: Value = serde_json::from_slice(&restored.stdout)?;
    assert_eq!(
        restore_report["schema_version"],
        "hyperai.local-storage-restore.v1"
    );
    assert_eq!(restore_report["status"], "restored");
    assert_eq!(restore_report["selected_backup_name"], backup_name);
    assert_eq!(
        restore_report["selected_backup_sha256"],
        backup_report["backup_sha256"]
    );
    assert_eq!(restore_report["metadata_row_count"], 0);
    assert!(restore_report["safety_backup_name"].is_string());
    assert_eq!(
        restore_report["safety_backup_sha256"]
            .as_str()
            .map(str::len),
        Some(64)
    );
    assert!(!String::from_utf8_lossy(&restored.stdout).contains(home_arg));
    Ok(())
}

#[test]
fn storage_restore_rejects_path_traversal_with_stable_error_contract() -> Result<(), Box<dyn Error>>
{
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("hyperai-home");
    let home_arg = home.to_str().ok_or("temporary home was not UTF-8")?;
    let initialized = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["init", "--home", home_arg])
        .output()?;
    assert!(initialized.status.success());

    let rejected = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args([
            "storage",
            "restore",
            "--home",
            home_arg,
            "--backup-name",
            "../outside.sqlite3",
        ])
        .output()?;
    assert_eq!(rejected.status.code(), Some(2));
    let error: Value = serde_json::from_slice(&rejected.stderr)?;
    assert_eq!(error["error_code"], "INVALID_BACKUP_NAME");
    assert!(
        !error["message"]
            .as_str()
            .is_some_and(|message| message.contains(home_arg))
    );
    Ok(())
}

#[test]
fn agent_profile_command_validates_without_starting_a_process() -> Result<(), Box<dyn Error>> {
    let profile = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args([
            "agent",
            "profile",
            "--name",
            "Synthetic fixture",
            "--executable",
            "/usr/bin/true",
            "--working-directory",
            "/tmp",
            "--arg=--offline",
        ])
        .output()?;
    assert!(profile.status.success());
    let report: Value = serde_json::from_slice(&profile.stdout)?;
    assert_eq!(report["schema_version"], "hyperai.acp.profile-check.v1");
    assert_eq!(report["status"], "valid_not_started");
    assert_eq!(report["argument_count"], 1);
    assert_eq!(report["process_sandbox"], false);
    Ok(())
}

#[cfg(unix)]
#[test]
#[ignore = "requires service sidecars staged by cargo build --workspace --bins"]
fn service_issues_scoped_capability_to_synthetic_acp_profile() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let (hyperai, fixture) = packaged_fixture_paths()?;
    exercise_synthetic_acp_install(&hyperai, &fixture, temporary.path())
}

#[cfg(unix)]
#[test]
#[ignore = "installs locked workspace binaries into a disposable root"]
fn source_install_bundles_mcp_sidecar_and_connects_synthetic_peer() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let install_root = temporary.path().join("installed HyperAI");
    let installer = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/install-hyperai-local.mjs");
    let install = Command::new("node")
        .arg(installer)
        .args([
            "--root",
            install_root.to_str().ok_or("install path was not UTF-8")?,
            "--offline",
        ])
        .output()?;
    if !install.status.success() {
        return Err(format!(
            "local source install failed: {}",
            String::from_utf8_lossy(&install.stderr)
        )
        .into());
    }

    let binary_directory = install_root.join("bin");
    let hyperai = binary_directory.join("hyperai");
    let hsai = binary_directory.join("hsai");
    let hyperai_ui = binary_directory.join("hyperai-ui");
    let mcp_server = binary_directory.join("hyperai-mcp");
    assert!(hyperai.is_file(), "installed hyperai binary is missing");
    assert!(hsai.is_file(), "installed hsai binary is missing");
    assert!(
        hyperai_ui.is_file(),
        "installed hyperai-ui binary is missing"
    );
    assert!(
        mcp_server.is_file(),
        "installed sibling MCP server is missing"
    );
    assert!(
        !binary_directory.join("acp-fixture-peer").exists(),
        "synthetic test peer must not be part of the product install"
    );

    exercise_installed_compatibility_entrypoints(&hyperai, &hsai, &hyperai_ui, temporary.path())?;

    let lifecycle_home = temporary.path().join("installed-cli-home");
    exercise_installed_cli_lifecycle(&hyperai, &lifecycle_home)?;
    exercise_installed_tui_pty(&hyperai, true)?;

    let test_binary = std::env::current_exe()?;
    let target_directory = test_binary
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or("CLI integration test binary has no Cargo target directory")?;
    let fixture = target_directory.join("acp-fixture-peer");
    if !fixture.is_file() {
        return Err("build workspace binaries before the local install integration".into());
    }
    exercise_synthetic_acp_install(&hyperai, &fixture, temporary.path())
}

#[cfg(unix)]
fn exercise_synthetic_acp_install(
    hyperai: &std::path::Path,
    fixture: &std::path::Path,
    temporary_root: &std::path::Path,
) -> Result<(), Box<dyn Error>> {
    let home = temporary_root.join("home");
    let home_arg = home.to_str().ok_or("temporary home was not UTF-8")?;
    let child = Command::new(hyperai)
        .args(["__serve", "--home", home_arg, "--port", "0"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let mut process = ServerProcess(child);
    let record = wait_for_server(&home, &mut process.0)?;
    let address = record["address"]
        .as_str()
        .ok_or("service record has no address")?;
    let base = format!("http://{address}");
    let bearer = fs::read_to_string(home.join("runtime").join("bearer.token"))?;
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()?;

    seed_synthetic_data(&client, &base, &bearer)?;
    let (simulation_before, owner_preview) =
        verify_owner_and_cli_preview(hyperai, &client, &base, &bearer, home_arg)?;
    let fixture_session = SyntheticAcpSession {
        client: &client,
        base: &base,
        bearer: &bearer,
        fixture,
        working_directory: temporary_root,
        simulation_before: &simulation_before,
        owner_preview: &owner_preview,
    };
    let connected =
        exercise_synthetic_acp_session(&fixture_session, &WorkloadTraceExpectation::Success)?;
    assert_eq!(connected["status"], "ready");

    let synthetic_data = home.join("data").join("synthetic-market-observations.json");
    let denied = exercise_permission_limited_acp_trace(&fixture_session, &synthetic_data)?;
    if !denied {
        eprintln!("Data permission probe: readable; denied ACP state not exercised");
    }

    let mut bytes = fs::read(&synthetic_data)?;
    bytes.push(b' ');
    fs::write(&synthetic_data, bytes)?;
    verify_owner_workload_trace_conflict(&client, &base, &bearer)?;

    let failed_trace =
        exercise_synthetic_acp_session(&fixture_session, &WorkloadTraceExpectation::Conflict)?;
    assert_eq!(failed_trace["status"], "ready");
    stop_test_service(&client, &base, &bearer, &mut process)?;
    Ok(())
}

#[cfg(unix)]
struct SyntheticAcpSession<'a> {
    client: &'a reqwest::blocking::Client,
    base: &'a str,
    bearer: &'a str,
    fixture: &'a std::path::Path,
    working_directory: &'a std::path::Path,
    simulation_before: &'a Value,
    owner_preview: &'a Value,
}

#[cfg(unix)]
#[derive(Clone, Debug, Eq, PartialEq)]
enum WorkloadTraceExpectation {
    Success,
    Conflict,
    Denied { trace_digest: String },
}

#[cfg(unix)]
fn exercise_synthetic_acp_session(
    session: &SyntheticAcpSession<'_>,
    expectation: &WorkloadTraceExpectation,
) -> Result<Value, Box<dyn Error>> {
    let connected = connect_synthetic_fixture(
        session.client,
        session.base,
        session.bearer,
        session.fixture,
        session.working_directory,
        expectation,
    )?;
    prompt_synthetic_fixture(
        session.client,
        session.base,
        session.bearer,
        expectation.prompt_event(),
    )?;
    verify_simulation_unchanged(
        session.client,
        session.base,
        session.bearer,
        session.simulation_before,
    )?;
    disconnect_and_verify_fixture(
        session.client,
        session.base,
        session.bearer,
        session.working_directory,
        session.owner_preview,
        expectation,
    )?;
    Ok(connected)
}

#[cfg(unix)]
impl WorkloadTraceExpectation {
    fn prompt_event(&self) -> &'static str {
        match self {
            Self::Success => "Synthetic ACP response.",
            Self::Conflict => {
                "Synthetic ACP fixture observed Workload Trace Data status conflicting."
            }
            Self::Denied { .. } => {
                "Synthetic ACP fixture observed Workload Trace Data status denied."
            }
        }
    }
}

#[cfg(unix)]
fn exercise_permission_limited_acp_trace(
    session: &SyntheticAcpSession<'_>,
    sample: &std::path::Path,
) -> Result<bool, Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    let original_mode = fs::metadata(sample)?.permissions().mode();
    let mut restricted = fs::metadata(sample)?.permissions();
    restricted.set_mode(original_mode & !0o444);
    fs::set_permissions(sample, restricted)?;

    let result = (|| -> Result<bool, Box<dyn Error>> {
        let denied = match fs::read(sample) {
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => true,
            Ok(_) => false,
            Err(error) => return Err(error.into()),
        };
        eprintln!(
            "Data permission probe: {}",
            if denied {
                "PermissionDenied"
            } else {
                "readable; denied not exercised"
            }
        );
        let trace = verify_owner_workload_trace_data_status(
            session.client,
            session.base,
            session.bearer,
            if denied { "denied" } else { "bound" },
        )?;
        if denied {
            let trace_digest = trace["digest"]
                .as_str()
                .ok_or("denied owner trace omitted its trace digest")?
                .to_owned();
            exercise_synthetic_acp_session(
                session,
                &WorkloadTraceExpectation::Denied { trace_digest },
            )?;
        }
        Ok(denied)
    })();

    let mut restored = fs::metadata(sample)?.permissions();
    restored.set_mode(original_mode);
    fs::set_permissions(sample, restored)?;
    let denied = result?;
    if fs::read(sample).is_err() {
        return Err(
            std::io::Error::other("synthetic Data file permission was not restored").into(),
        );
    }
    Ok(denied)
}

#[cfg(unix)]
fn packaged_fixture_paths() -> Result<(std::path::PathBuf, std::path::PathBuf), Box<dyn Error>> {
    let hyperai = std::path::PathBuf::from(env!("CARGO_BIN_EXE_hyperai"));
    let binary_dir = hyperai.parent().ok_or("HyperAI binary has no parent")?;
    let fixture = binary_dir.join("acp-fixture-peer");
    let mcp_server = binary_dir.join("hyperai-mcp");
    if !fixture.is_file() || !mcp_server.is_file() {
        return Err(
            "build the workspace binaries before running the ACP service integration test".into(),
        );
    }
    Ok((hyperai, fixture))
}

#[cfg(unix)]
fn seed_synthetic_data(
    client: &reqwest::blocking::Client,
    base: &str,
    bearer: &str,
) -> Result<(), Box<dyn Error>> {
    let preview: Value = client
        .get(format!("{base}/api/v1/data/synthetic/preview"))
        .bearer_auth(bearer)
        .send()?
        .error_for_status()?
        .json()?;
    let preview_digest = preview["preview_digest"]
        .as_str()
        .ok_or("synthetic Data preview omitted its digest")?;
    client
        .post(format!("{base}/api/v1/data/synthetic/seed"))
        .bearer_auth(bearer)
        .header("idempotency-key", "acp-synthetic-data-seed-v1")
        .json(&serde_json::json!({ "preview_digest": preview_digest }))
        .send()?
        .error_for_status()?;
    Ok(())
}

#[cfg(unix)]
fn verify_owner_and_cli_preview(
    hyperai: &std::path::Path,
    client: &reqwest::blocking::Client,
    base: &str,
    bearer: &str,
    home: &str,
) -> Result<(Value, Value), Box<dyn Error>> {
    let simulation_before = get_fleet_simulation(client, base, bearer)?;
    let owner_preview: Value = client
        .post(format!("{base}/api/v1/fleet/operation-preview"))
        .bearer_auth(bearer)
        .json(&serde_json::json!({
            "node_id": "fixture-node-atlas",
            "operation": "launch"
        }))
        .send()?
        .error_for_status()?
        .json()?;
    assert_eq!(owner_preview["preview_status"], "preview_ready");
    assert_eq!(owner_preview["authority"], "not_authorized");
    assert_eq!(
        owner_preview["effect_boundary"],
        "advisory_only_does_not_invoke_or_reserve"
    );

    let cli_preview = Command::new(hyperai)
        .args([
            "fleet",
            "--home",
            home,
            "preview",
            "--node-id",
            "fixture-node-atlas",
            "--operation",
            "launch",
        ])
        .output()?;
    assert!(
        cli_preview.status.success(),
        "{}",
        String::from_utf8_lossy(&cli_preview.stderr)
    );
    let cli_preview: Value = serde_json::from_slice(&cli_preview.stdout)?;
    for field in [
        "schema_version",
        "node_id",
        "operation",
        "preview_status",
        "authority",
        "effect_boundary",
        "preview_digest",
    ] {
        assert_eq!(cli_preview[field], owner_preview[field], "field {field}");
    }
    Ok((simulation_before, owner_preview))
}

#[cfg(unix)]
fn get_fleet_simulation(
    client: &reqwest::blocking::Client,
    base: &str,
    bearer: &str,
) -> Result<Value, Box<dyn Error>> {
    Ok(client
        .get(format!("{base}/api/v1/fleet/simulation"))
        .bearer_auth(bearer)
        .send()?
        .error_for_status()?
        .json()?)
}

#[cfg(unix)]
fn connect_synthetic_fixture(
    client: &reqwest::blocking::Client,
    base: &str,
    bearer: &str,
    fixture: &std::path::Path,
    working_directory: &std::path::Path,
    expectation: &WorkloadTraceExpectation,
) -> Result<Value, Box<dyn Error>> {
    let mut args = vec![
        "--probe-mcp-capability",
        "--close-support",
        "--invoke-mcp-stdio",
    ];
    match expectation {
        WorkloadTraceExpectation::Success => {}
        WorkloadTraceExpectation::Conflict => {
            args.push("--expect-workload-trace-conflict");
        }
        WorkloadTraceExpectation::Denied { trace_digest } => {
            args.push("--expect-workload-trace-denied");
            args.push("--expected-workload-trace-digest");
            args.push(trace_digest);
        }
    }
    let response = client
        .post(format!("{base}/api/v1/agent/connect"))
        .bearer_auth(bearer)
        .json(&serde_json::json!({
            "profile": {
                "name": "Synthetic scoped capability probe",
                "executable": fixture,
                "args": args,
                "working_directory": working_directory,
                "environment": {}
            }
        }))
        .send()?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text()?;
        let peer_failure =
            fs::read_to_string(working_directory.join("mcp-capability-probe.failure"))
                .unwrap_or_else(|_| "no synthetic peer diagnostic was written".to_owned());
        return Err(format!(
            "synthetic ACP connection failed with status {status}: {body}; peer detail: {peer_failure}"
        )
        .into());
    }
    let connected: Value = response.json()?;
    assert_eq!(connected["protocol_version"], 1);
    assert_eq!(
        connected["tools"],
        serde_json::json!(hyperai_acp::MCP_TOOL_NAMES_V6)
    );
    assert!(!connected.to_string().contains("HYPERAI_MCP_CAPABILITY"));
    Ok(connected)
}

#[cfg(unix)]
fn prompt_synthetic_fixture(
    client: &reqwest::blocking::Client,
    base: &str,
    bearer: &str,
    expected_event: &str,
) -> Result<(), Box<dyn Error>> {
    let response: Value = client
        .post(format!("{base}/api/v1/agent/prompt"))
        .bearer_auth(bearer)
        .json(&serde_json::json!({ "prompt": "Summarize the fixed synthetic trace status." }))
        .send()?
        .error_for_status()?
        .json()?;
    assert_eq!(response["status"], "ready");
    assert_eq!(response["events"].as_array().map(Vec::len), Some(1));
    assert_eq!(response["events"][0]["text"], expected_event);
    Ok(())
}

#[cfg(unix)]
fn verify_owner_workload_trace_data_status(
    client: &reqwest::blocking::Client,
    base: &str,
    bearer: &str,
    expected_status: &str,
) -> Result<Value, Box<dyn Error>> {
    let response = client
        .get(format!("{base}/api/v1/market/workload-trace"))
        .bearer_auth(bearer)
        .send()?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let trace: Value = response.json()?;
    let data_reference = trace["references"]
        .as_array()
        .and_then(|references| {
            references
                .iter()
                .find(|reference| reference["owner_id"] == "hyperai-data")
        })
        .ok_or("owner Workload Trace omitted the Data reference")?;
    assert_eq!(data_reference["status"], expected_status);
    if expected_status == "bound" {
        let digest = data_reference["digest"]
            .as_str()
            .ok_or("bound owner trace omitted its Data digest")?;
        assert_eq!(
            trace["intent"]["body"]["data_references"][0]["digest"],
            digest
        );
    } else {
        assert!(data_reference["digest"].is_null());
        assert_eq!(
            trace["intent"]["body"]["data_references"],
            serde_json::json!([])
        );
    }
    Ok(trace)
}

#[cfg(unix)]
fn verify_owner_workload_trace_conflict(
    client: &reqwest::blocking::Client,
    base: &str,
    bearer: &str,
) -> Result<(), Box<dyn Error>> {
    let trace = verify_owner_workload_trace_data_status(client, base, bearer, "conflicting")?;
    assert_eq!(
        trace["schema_version"],
        "hyperai.interface.workload-trace.v1"
    );
    assert_eq!(
        trace["state_slice"],
        "hyperai.bazaar.workload_trace_synthetic.v1"
    );
    assert!(trace["digest"].as_str().is_some());
    assert!(
        !trace
            .to_string()
            .contains("synthetic-market-observations.json")
    );
    Ok(())
}

#[cfg(unix)]
fn verify_simulation_unchanged(
    client: &reqwest::blocking::Client,
    base: &str,
    bearer: &str,
    simulation_before: &Value,
) -> Result<(), Box<dyn Error>> {
    let simulation_after = get_fleet_simulation(client, base, bearer)?;
    assert_eq!(simulation_after, *simulation_before);
    Ok(())
}

#[cfg(unix)]
fn disconnect_and_verify_fixture(
    client: &reqwest::blocking::Client,
    base: &str,
    bearer: &str,
    working_directory: &std::path::Path,
    owner_preview: &Value,
    expectation: &WorkloadTraceExpectation,
) -> Result<(), Box<dyn Error>> {
    let response = client
        .post(format!("{base}/api/v1/agent/disconnect"))
        .bearer_auth(bearer)
        .json(&serde_json::json!({}))
        .send()?
        .error_for_status()?;
    let disconnected: Value = response.json()?;
    assert_eq!(disconnected["status"], "disconnected");
    let owner_comparison: Value = client
        .get(format!("{base}/api/v1/market/supply-comparison"))
        .bearer_auth(bearer)
        .send()?
        .error_for_status()?
        .json()?;
    assert_eq!(
        owner_comparison["body"]["schema_version"],
        "hyperai.compute-market.supply-comparison.v1"
    );
    assert_eq!(owner_comparison["body"]["selected_offer_id"], "offer-gpu-b");
    let owner_comparison_digest = owner_comparison["digest"]
        .as_str()
        .ok_or("owner supply comparison omitted its digest")?;
    let (query_status, trace_status) = match expectation {
        WorkloadTraceExpectation::Success => ("422:DATA_QUERY_REJECTED", "ok"),
        WorkloadTraceExpectation::Conflict => ("not_run", "conflicting"),
        WorkloadTraceExpectation::Denied { .. } => ("not_run", "denied"),
    };
    assert_eq!(
        fs::read_to_string(working_directory.join("mcp-capability-probe.report"))?,
        format!(
            "supply_comparison_digest={owner_comparison_digest}\nfleet_preview=preview_ready:not_authorized:advisory_only_does_not_invoke_or_reserve\nfleet_preview_digest={}\nquery_rejected={query_status}\nworkload_trace={trace_status}\nrevoked=401\nmcp_after_revoke=denied\n",
            owner_preview["preview_digest"]
                .as_str()
                .ok_or("owner preview omitted its preview digest")?
        )
    );
    Ok(())
}

#[cfg(unix)]
fn stop_test_service(
    client: &reqwest::blocking::Client,
    base: &str,
    bearer: &str,
    process: &mut ServerProcess,
) -> Result<(), Box<dyn Error>> {
    client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(bearer)
        .send()?
        .error_for_status()?;
    let _ = process.0.wait()?;
    Ok(())
}

#[test]
fn init_and_service_commands_use_a_disposable_home() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("home");
    let initialized = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args([
            "init",
            "--home",
            home.to_str().ok_or("temporary path was not UTF-8")?,
        ])
        .output()?;
    assert!(initialized.status.success());
    assert!(home.join("config.json").is_file());
    let init_report: Value = serde_json::from_slice(&initialized.stdout)?;
    assert_eq!(
        init_report["schema_version"],
        "hyperai.local-home-report.v1"
    );

    let started = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args([
            "service",
            "start",
            "--home",
            home.to_str().ok_or("temporary path was not UTF-8")?,
            "--port",
            "0",
        ])
        .output()?;
    if !started.status.success() {
        return Err(format!(
            "service start failed: {}",
            String::from_utf8_lossy(&started.stderr)
        )
        .into());
    }
    let start_report: Value = serde_json::from_slice(&started.stdout)?;
    let service_address = start_report["service"]["address"]
        .as_str()
        .ok_or("service start did not report an address")?;

    let duplicate_start = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args([
            "service",
            "start",
            "--home",
            home.to_str().ok_or("temporary path was not UTF-8")?,
            "--port",
            "0",
        ])
        .output()?;
    assert!(duplicate_start.status.success());
    let duplicate_report: Value = serde_json::from_slice(&duplicate_start.stdout)?;
    assert_eq!(duplicate_report["status"], "already_running");

    let status = Command::new(env!("CARGO_BIN_EXE_hsai"))
        .args([
            "service",
            "status",
            "--home",
            home.to_str().ok_or("temporary path was not UTF-8")?,
        ])
        .output()?;
    assert!(status.status.success());
    let status_report: Value = serde_json::from_slice(&status.stdout)?;
    assert_eq!(status_report["status"], "ready");
    assert_eq!(status_report["address"], service_address);

    let stopped = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args([
            "service",
            "stop",
            "--home",
            home.to_str().ok_or("temporary path was not UTF-8")?,
        ])
        .output()?;
    assert!(stopped.status.success());
    let stopped_report: Value = serde_json::from_slice(&stopped.stdout)?;
    assert_eq!(stopped_report["status"], "shutdown_requested");
    Ok(())
}

#[test]
fn explicit_home_overrides_environment_and_environment_is_used_without_flag()
-> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let environment_home = temporary.path().join("environment-home");
    let explicit_home = temporary.path().join("explicit-home");
    let environment_home_arg = environment_home.to_str().ok_or("path was not UTF-8")?;
    let explicit_home_arg = explicit_home.to_str().ok_or("path was not UTF-8")?;

    let explicit = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["init", "--home", explicit_home_arg])
        .env("HYPERAI_HOME", environment_home_arg)
        .output()?;
    assert!(explicit.status.success());
    assert!(explicit_home.join("config.json").is_file());
    assert!(!environment_home.exists());

    let environment = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .arg("init")
        .env("HYPERAI_HOME", environment_home_arg)
        .output()?;
    assert!(environment.status.success());
    assert!(environment_home.join("config.json").is_file());
    Ok(())
}

#[test]
fn service_start_refuses_occupied_port_and_leaves_no_stale_capability() -> Result<(), Box<dyn Error>>
{
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("home");
    let home_arg = home.to_str().ok_or("temporary path was not UTF-8")?;
    let occupied = TcpListener::bind(("127.0.0.1", 0))?;
    let port = occupied.local_addr()?.port().to_string();

    let start = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["service", "start", "--home", home_arg, "--port", &port])
        .output()?;
    assert!(!start.status.success());
    assert!(String::from_utf8_lossy(&start.stderr).contains("SERVICE_START_FAILED"));
    assert!(!home.join("runtime").join("service.json").exists());
    assert!(!home.join("runtime").join("bearer.token").exists());
    Ok(())
}

#[test]
fn forced_process_exit_recovers_stale_record_and_rotates_bearer() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("home");
    let home_arg = home.to_str().ok_or("temporary path was not UTF-8")?;
    let child = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["__serve", "--home", home_arg, "--port", "0"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let mut process = ServerProcess(child);
    let first_record = wait_for_server(&home, &mut process.0)?;
    let first_token = fs::read_to_string(home.join("runtime").join("bearer.token"))?;

    process.0.kill()?;
    let _ = process.0.wait()?;
    let record_path = home.join("runtime").join("service.json");
    assert!(
        record_path.is_file(),
        "forced exit should leave a stale record"
    );

    let restarted = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["service", "start", "--home", home_arg, "--port", "0"])
        .output()?;
    assert!(
        restarted.status.success(),
        "recovery start failed: {}",
        String::from_utf8_lossy(&restarted.stderr)
    );
    let restart_report: Value = serde_json::from_slice(&restarted.stdout)?;
    let second_record = &restart_report["service"];
    assert_ne!(first_record["instance_id"], second_record["instance_id"]);
    let second_token = fs::read_to_string(home.join("runtime").join("bearer.token"))?;
    assert_ne!(first_token, second_token);

    let stopped = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["service", "stop", "--home", home_arg])
        .output()?;
    assert!(stopped.status.success());
    Ok(())
}

#[test]
fn fleet_readonly_cli_exposes_inventory_doctor_and_non_executable_preview()
-> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("home");
    let home_arg = home.to_str().ok_or("temporary path was not UTF-8")?;
    let child = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["__serve", "--home", home_arg, "--port", "0"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let mut process = ServerProcess(child);
    let _ = wait_for_server(&home, &mut process.0)?;

    let before = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["fleet", "--home", home_arg, "simulation", "show"])
        .output()?;
    assert!(before.status.success());
    let before: Value = serde_json::from_slice(&before.stdout)?;

    let inventory = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["fleet", "--home", home_arg, "inventory"])
        .output()?;
    assert!(inventory.status.success());
    let inventory: Value = serde_json::from_slice(&inventory.stdout)?;
    assert_eq!(inventory["schema_version"], "hyperai.fleet.inventory.v1");
    assert_eq!(
        inventory["provenance"]["observation_mode"],
        "no_live_probe_or_import"
    );
    assert_eq!(inventory["nodes"].as_array().map(Vec::len), Some(4));
    assert_eq!(
        inventory["effect_boundary"],
        "local_synthetic_read_only_no_probe_no_external_effects"
    );

    let doctor = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["fleet", "--home", home_arg, "doctor"])
        .output()?;
    assert!(doctor.status.success());
    let doctor: Value = serde_json::from_slice(&doctor.stdout)?;
    assert_eq!(doctor, inventory["doctor"]);
    assert_eq!(doctor["mode"], "synthetic_fixture_only");

    let preview = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args([
            "fleet",
            "--home",
            home_arg,
            "preview",
            "--node-id",
            "fixture-node-atlas",
            "--operation",
            "launch",
        ])
        .output()?;
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let preview: Value = serde_json::from_slice(&preview.stdout)?;
    assert_eq!(
        preview["schema_version"],
        "hyperai.fleet.operation-preview.v1"
    );
    assert_eq!(preview["preview_status"], "preview_ready");
    assert_eq!(preview["authority"], "not_authorized");
    assert_eq!(
        preview["effect_boundary"],
        "advisory_only_does_not_invoke_or_reserve"
    );
    let preview_digest = preview["preview_digest"]
        .as_str()
        .ok_or("preview omitted its digest")?;

    let changed = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args([
            "fleet",
            "--home",
            home_arg,
            "preview",
            "--node-id",
            "fixture-node-atlas",
            "--operation",
            "terminate",
            "--expected-preview-digest",
            preview_digest,
        ])
        .output()?;
    assert!(changed.status.success());
    let changed: Value = serde_json::from_slice(&changed.stdout)?;
    assert_eq!(changed["preview_status"], "preview_changed");
    assert_ne!(changed["preview_digest"], preview["preview_digest"]);

    let after = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["fleet", "--home", home_arg, "simulation", "show"])
        .output()?;
    assert!(after.status.success());
    let after: Value = serde_json::from_slice(&after.stdout)?;
    assert_eq!(after, before, "a preview must not alter simulator state");
    Ok(())
}

#[test]
fn fleet_simulation_cli_uses_digest_bound_service_transitions() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("home");
    let home_arg = home.to_str().ok_or("temporary path was not UTF-8")?;
    let child = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["__serve", "--home", home_arg, "--port", "0"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let mut process = ServerProcess(child);
    let _ = wait_for_server(&home, &mut process.0)?;

    let show = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["fleet", "--home", home_arg, "simulation", "show"])
        .output()?;
    assert!(show.status.success());
    let initial: Value = serde_json::from_slice(&show.stdout)?;
    assert_eq!(initial["phase"], "detached");
    let expected_digest = initial["state_digest"]
        .as_str()
        .ok_or("simulator snapshot did not include a state digest")?;

    let step_args = [
        "fleet",
        "--home",
        home_arg,
        "simulation",
        "step",
        "--operation",
        "attach",
        "--expected-state-digest",
        expected_digest,
        "--idempotency-key",
        "cli-attach-once",
    ];
    let first = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(step_args)
        .output()?;
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first_json: Value = serde_json::from_slice(&first.stdout)?;
    assert_eq!(first_json["snapshot"]["phase"], "attached");

    let replay = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(step_args)
        .output()?;
    assert!(replay.status.success());
    let replay_json: Value = serde_json::from_slice(&replay.stdout)?;
    assert_eq!(first_json, replay_json);

    let rejected = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args([
            "fleet",
            "--home",
            home_arg,
            "simulation",
            "step",
            "--operation",
            "launch",
            "--gpu-units",
            "2",
            "--expected-state-digest",
            expected_digest,
            "--idempotency-key",
            "cli-attach-once",
        ])
        .output()?;
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("FLEET_SIMULATION_CONFLICT"));
    Ok(())
}

#[test]
fn data_cli_preserves_the_service_error_code_for_a_rejected_query() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("home");
    let home_arg = home.to_str().ok_or("temporary path was not UTF-8")?;
    let child = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["__serve", "--home", home_arg, "--port", "0"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let mut process = ServerProcess(child);
    let _ = wait_for_server(&home, &mut process.0)?;

    let rejected = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args([
            "data",
            "--home",
            home_arg,
            "sql",
            "hyperai.synthetic_market_observations_json.v1",
            "--query",
            "SELECT read_file('/tmp/hyperai-parity-nonexistent-canary') FROM dataset",
        ])
        .output()?;
    assert_eq!(rejected.status.code(), Some(6));
    let error: Value = serde_json::from_slice(&rejected.stderr)?;
    assert_eq!(error["error_code"], "DATA_QUERY_REJECTED");
    assert_eq!(
        error["message"],
        "The local HyperAI contract operation could not complete."
    );
    assert!(!error.to_string().contains("read_file"));
    Ok(())
}
