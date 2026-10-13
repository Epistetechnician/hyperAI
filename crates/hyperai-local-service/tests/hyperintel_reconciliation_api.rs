//! Authenticated `HyperIntel` reconciliation route integration coverage.

#[path = "../../hyperintel-adapter/tests/support/reconciliation.rs"]
mod support;

use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    time::Duration,
};

use hyperai_hyperintel_adapter::MAX_INPUT_BYTES;
use hyperai_local_service::{
    AuthState,
    lifecycle::{self, ServiceRecordV1},
    server,
    storage::{self, HomePaths},
};
use hyperintel_reconciliation::{ReceiptReconciliationReportV1, ReceiptReconciliationStatusV1};
use reqwest::{Client, header};
use serde_json::Value;
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
    let client = Client::builder().timeout(Duration::from_secs(1)).build()?;
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

fn directory_contains_bytes(root: &Path, needle: &[u8]) -> io::Result<bool> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        if file_type.is_dir() && directory_contains_bytes(&path, needle)? {
            return Ok(true);
        }
        if file_type.is_file()
            && fs::read(path)?
                .windows(needle.len())
                .any(|window| window == needle)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn authenticated_service_reconciles_synthetic_lineage_without_persisting_it()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let service_home = temporary.path().join("home");
    let (task, paths, record, bearer) = launch(service_home.clone()).await?;
    let client = Client::builder().timeout(Duration::from_secs(2)).build()?;
    let base = format!("http://{}", record.address);

    let input = support::build_completed_input()
        .await
        .map_err(|error| io::Error::other(error.to_string()))?;
    assert!(
        input
            .execution_input
            .len()
            .saturating_add(input.trusted_keys_input.len())
            <= MAX_INPUT_BYTES
    );
    let body = serde_json::to_vec(&serde_json::json!({
        "execution_input": input.execution_input,
        "trusted_keys_input": input.trusted_keys_input,
    }))?;

    let unauthenticated = client
        .post(format!("{base}/api/v1/inference/reconcile-execution"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(body.clone())
        .send()
        .await?;
    assert_eq!(unauthenticated.status(), reqwest::StatusCode::UNAUTHORIZED);
    let unauthenticated_body: Value = unauthenticated.json().await?;
    assert_eq!(unauthenticated_body["code"], "UNAUTHENTICATED");

    let session_response = client.get(format!("{base}/api/v1/session")).send().await?;
    assert_eq!(session_response.status(), reqwest::StatusCode::OK);
    let cookie = session_response
        .headers()
        .get(header::SET_COOKIE)
        .ok_or("session bootstrap omitted its cookie")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("session cookie was empty")?
        .to_owned();
    let session: Value = session_response.json().await?;
    let csrf = session["csrf_token"]
        .as_str()
        .ok_or("session bootstrap omitted its CSRF token")?;

    let response = client
        .post(format!("{base}/api/v1/inference/reconcile-execution"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ORIGIN, &base)
        .header(header::COOKIE, &cookie)
        .header("x-csrf-token", csrf)
        .body(body)
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("application/json"))
    );

    let result: Value = response.json().await?;
    assert_eq!(result["status"], "local_record_reconciliation_pass");
    assert_eq!(result["report"]["status"], "COMPLETED");
    assert_eq!(result["authority_granted"], false);
    assert_eq!(result["inputs_persisted"], false);
    let report: ReceiptReconciliationReportV1 = serde_json::from_value(result["report"].clone())?;
    assert_eq!(report.status, ReceiptReconciliationStatusV1::Completed);
    assert_eq!(report.report_digest, report.digest()?);

    let response_json = serde_json::to_string(&result)?;
    assert!(!response_json.contains(support::STREAM_CANARY));
    for omitted in [
        "\"chunks\"",
        "\"messages\"",
        "\"frames\"",
        "\"content\"",
        "\"signature_hex\"",
        "\"public_key_hex\"",
        "\"credential\"",
    ] {
        assert!(!response_json.contains(omitted));
    }
    assert!(!directory_contains_bytes(
        &service_home,
        support::STREAM_CANARY.as_bytes()
    )?);

    let stop = client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(&bearer)
        .send()
        .await?;
    assert_eq!(stop.status(), reqwest::StatusCode::ACCEPTED);
    task.await??;

    let home_root = paths.root.clone();
    temporary.close()?;
    assert!(!home_root.exists());
    Ok(())
}
