//! Executable-level contracts for the bounded local synthetic Eval API.

use std::{
    error::Error,
    fs,
    io::Write,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

use reqwest::StatusCode;
use serde_json::{Value, json};

struct ServerProcess(Child);

impl Drop for ServerProcess {
    fn drop(&mut self) {
        if self.0.try_wait().is_ok_and(|status| status.is_none()) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn wait_for_server(home: &Path, process: &mut Child) -> Result<String, Box<dyn Error>> {
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
                return Ok(format!("http://{address}"));
            }
        }
        if let Some(status) = process.try_wait()? {
            return Err(format!("service exited before readiness: {status}").into());
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err("service did not become ready".into())
}

fn request_eval(
    client: &reqwest::blocking::Client,
    base_url: &str,
    token: &str,
    body: &Value,
) -> Result<(StatusCode, Value), Box<dyn Error>> {
    let response = client
        .post(format!("{base_url}/api/v1/eval/run"))
        .bearer_auth(token)
        .json(body)
        .send()?;
    let status = response.status();
    Ok((status, response.json()?))
}

fn eval_request(catalog: &Value, key: &str) -> Value {
    json!({
        "suite_id": catalog["suite_id"],
        "expected_suite_digest": catalog["suite_digest"],
        "pair_id": catalog["pair_id"],
        "idempotency_key": key
    })
}

fn verify_catalog_and_cli(
    client: &reqwest::blocking::Client,
    base_url: &str,
    token: &str,
    home_arg: &str,
) -> Result<Value, Box<dyn Error>> {
    let unauthorized = client
        .get(format!("{base_url}/api/v1/eval/catalog"))
        .send()?;
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let catalog_response = client
        .get(format!("{base_url}/api/v1/eval/catalog"))
        .bearer_auth(token)
        .send()?;
    assert_eq!(catalog_response.status(), StatusCode::OK);
    let catalog: Value = catalog_response.json()?;
    assert_eq!(catalog["cases"].as_array().map(Vec::len), Some(6));
    let catalog_text = catalog.to_string();
    assert!(!catalog_text.contains("expected_code"));
    assert!(!catalog_text.contains("constraints_preserved"));

    let cli_catalog = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args(["eval", "--home", home_arg, "catalog"])
        .output()?;
    assert!(cli_catalog.status.success());
    let cli_catalog: Value = serde_json::from_slice(&cli_catalog.stdout)?;
    assert_eq!(cli_catalog["cases"].as_array().map(Vec::len), Some(6));
    Ok(catalog)
}

fn verify_cli_run(home_arg: &str, catalog: &Value) -> Result<(), Box<dyn Error>> {
    let cli_run = Command::new(env!("CARGO_BIN_EXE_hyperai"))
        .args([
            "eval",
            "--home",
            home_arg,
            "run",
            "--expected-suite-digest",
            catalog["suite_digest"]
                .as_str()
                .ok_or("catalog digest missing")?,
            "--idempotency-key",
            "eval-cli-run",
        ])
        .output()?;
    assert!(cli_run.status.success());
    let cli_report: Value = serde_json::from_slice(&cli_run.stdout)?;
    assert_eq!(cli_report["alternate"]["matching_cases"], 3);
    Ok(())
}

fn verify_report(
    client: &reqwest::blocking::Client,
    base_url: &str,
    token: &str,
    catalog: &Value,
) -> Result<(Value, Value), Box<dyn Error>> {
    let request = eval_request(catalog, "eval-run-00");
    let (status, report) = request_eval(client, base_url, token, &request)?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["control"]["matching_cases"], 6);
    assert_eq!(report["alternate"]["matching_cases"], 3);
    assert_eq!(
        report["child_wall_time_ms"].as_array().map(Vec::len),
        Some(3)
    );
    assert_eq!(report["cases"].as_array().map(Vec::len), Some(6));
    let report_text = report.to_string();
    assert!(!report_text.contains("expected_code"));
    assert!(!report_text.contains("constraints_preserved"));
    Ok((request, report))
}

fn verify_idempotency_and_rejections(
    client: &reqwest::blocking::Client,
    base_url: &str,
    token: &str,
    catalog: &Value,
    request: &Value,
    report: &Value,
) -> Result<(), Box<dyn Error>> {
    let (replay_status, replay) = request_eval(client, base_url, token, request)?;
    assert_eq!(replay_status, StatusCode::OK);
    assert_eq!(replay, *report);

    let mut changed = request.clone();
    changed["pair_id"] = json!("changed-pair");
    let (conflict_status, conflict) = request_eval(client, base_url, token, &changed)?;
    assert_eq!(conflict_status, StatusCode::CONFLICT);
    assert_eq!(conflict["code"], "EVAL_IDEMPOTENCY_CONFLICT");

    let mut stale = eval_request(catalog, "stale-suite");
    stale["expected_suite_digest"] =
        json!("0000000000000000000000000000000000000000000000000000000000000000");
    let (stale_status, stale_report) = request_eval(client, base_url, token, &stale)?;
    assert_eq!(stale_status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(stale_report["code"], "EVAL_REQUEST_REJECTED");

    let mut unknown_field = stale;
    unknown_field["unexpected"] = json!(true);
    let (unknown_status, unknown) = request_eval(client, base_url, token, &unknown_field)?;
    assert_eq!(unknown_status, StatusCode::BAD_REQUEST);
    assert_eq!(unknown["code"], "INVALID_JSON");

    let oversized = format!("{{\"padding\":\"{}\"}}", "x".repeat(5_000));
    let oversized_response = client
        .post(format!("{base_url}/api/v1/eval/run"))
        .bearer_auth(token)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(oversized)
        .send()?;
    assert_eq!(oversized_response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    Ok(())
}

fn verify_retry_and_capacity(
    client: &reqwest::blocking::Client,
    base_url: &str,
    token: &str,
    catalog: &Value,
) -> Result<(), Box<dyn Error>> {
    let interrupted_request = eval_request(catalog, "eval-run-01");
    let impatient_client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_millis(1))
        .build()?;
    let _interrupted = request_eval(&impatient_client, base_url, token, &interrupted_request);
    let (recovered_status, _) = request_eval(client, base_url, token, &interrupted_request)?;
    assert_eq!(recovered_status, StatusCode::OK);

    for index in 2..31 {
        let unique = eval_request(catalog, &format!("eval-run-{index:02}"));
        let (unique_status, _) = request_eval(client, base_url, token, &unique)?;
        assert_eq!(unique_status, StatusCode::OK);
    }
    let full_request = eval_request(catalog, "eval-run-31");
    let (full_status, full) = request_eval(client, base_url, token, &full_request)?;
    assert_eq!(full_status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(full["code"], "EVAL_CACHE_FULL");
    Ok(())
}

#[test]
fn compatibility_interface_binary_supports_the_fixed_scorer_child() -> Result<(), Box<dyn Error>> {
    let catalog = hyperai_eval::eval_catalog_v1()?;
    let request = hyperai_eval::EvalScorerRequestV1 {
        expected_suite_digest: catalog.suite_digest,
        pair_id: hyperai_eval::EVAL_PAIR_ID_V1.to_owned(),
        seed_label: hyperai_eval::EVAL_SEED_LABEL_V1.to_owned(),
    };
    let input = serde_json::to_vec(&request)?;
    let mut child = Command::new(env!("CARGO_BIN_EXE_hyperai-ui"))
        .arg("__internal_eval_scorer")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("compatibility scorer stdin was not piped")?
        .write_all(&input)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["control"]["matching_cases"], 6);
    assert_eq!(report["alternate"]["matching_cases"], 3);
    Ok(())
}

#[test]
fn eval_api_is_authenticated_bounded_idempotent_and_volatile() -> Result<(), Box<dyn Error>> {
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
    let base_url = wait_for_server(&home, &mut process.0)?;
    let token = fs::read_to_string(home.join("runtime").join("bearer.token"))?;
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()?;

    let catalog = verify_catalog_and_cli(&client, &base_url, &token, home_arg)?;
    verify_cli_run(home_arg, &catalog)?;
    let (request, report) = verify_report(&client, &base_url, &token, &catalog)?;
    verify_idempotency_and_rejections(&client, &base_url, &token, &catalog, &request, &report)?;
    verify_retry_and_capacity(&client, &base_url, &token, &catalog)?;
    Ok(())
}
