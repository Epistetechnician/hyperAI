//! Populated browser journey for the local `HyperIntel` reconciliation view.

#[path = "../../hyperintel-adapter/tests/support/reconciliation.rs"]
mod support;

use std::{error::Error, io, path::PathBuf, process::Stdio};

use hyperai_hyperintel_adapter::MAX_INPUT_BYTES;
use serde_json::Value;
use tokio::{io::AsyncWriteExt, process::Command};

#[tokio::test]
#[ignore = "requires a built hyperai-ui binary and local Chrome or Chromium"]
async fn browser_reconciles_synthetic_lineage_and_links_activity_report()
-> Result<(), Box<dyn Error + Send + Sync>> {
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
    let request = serde_json::to_vec(&serde_json::json!({
        "execution_input": input.execution_input,
        "trusted_keys_input": input.trusted_keys_input,
    }))?;
    let repository_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;

    let mut child = Command::new("node")
        .arg("scripts/verify-interface-browser.mjs")
        .arg("--reconciliation-stdin")
        .current_dir(repository_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or("browser runner stdin was not piped")?;
    stdin.write_all(&request).await?;
    drop(stdin);

    let output = child.wait_with_output().await?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "browser runner failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
        .into());
    }

    let result: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(result["status"], "passed");
    assert_eq!(result["reconciliation"]["status"], "passed");
    assert_eq!(result["reconciliation"]["reportStatus"], "COMPLETED");
    assert_eq!(result["reconciliation"]["authority"], "None");
    assert_eq!(result["reconciliation"]["activitySource"], "HYPERINTEL");
    assert_eq!(
        result["reconciliation"]["activityStatus"],
        "LOCAL RECONCILIATION"
    );
    assert!(
        result["reconciliation"]["activityDigestMatches"]
            .as_bool()
            .unwrap_or(false)
    );
    assert!(
        result["reconciliation"]["returnedToInference"]
            .as_bool()
            .unwrap_or(false)
    );
    assert!(
        result["reconciliation"]["canaryAbsentFromDomAndStorage"]
            .as_bool()
            .unwrap_or(false)
    );

    let digest = result["reconciliation"]["reportDigest"]
        .as_str()
        .ok_or("browser runner omitted the report digest")?;
    assert_eq!(digest.len(), 64);
    assert!(
        digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    );
    assert_eq!(result["reconciliation"]["viewport"]["width"], 390);
    assert!(
        result["methods"]
            .as_array()
            .is_some_and(|methods| { methods.iter().any(|method| method == "POST") })
    );
    Ok(())
}
