//! Cross-process acceptance checks for the local synthetic market lifecycle.

use std::{error::Error, fs, process::Command};

use serde_json::Value;
use tempfile::tempdir;

fn run_cli(arguments: &[&str]) -> Result<std::process::Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_hsai"))
        .args(arguments)
        .output()?)
}

#[test]
fn receipt_export_verification_and_lost_receipt_recovery_cross_processes()
-> Result<(), Box<dyn Error>> {
    let temporary = tempdir()?;
    let output_dir = temporary.path().join("market-run");
    let output_dir_arg = output_dir.to_string_lossy().into_owned();
    let demo = run_cli(&["market", "demo", "--output-dir", &output_dir_arg])?;
    assert!(
        demo.status.success(),
        "demo failed: {}",
        String::from_utf8_lossy(&demo.stderr)
    );
    let summary: Value = serde_json::from_slice(&demo.stdout)?;
    assert_eq!(summary["source_count"], 2);
    assert_eq!(summary["duplicate_count"], 1);
    assert_eq!(summary["selected_offer_id"], "offer-gpu-b");
    assert_eq!(summary["settlement"]["disposition"], "release_to_seller");

    let receipt_path = output_dir.join("market-receipt.json");
    let journal_path = output_dir.join("market-journal.json");
    let exported_receipt = fs::read(&receipt_path)?;
    let receipt_arg = receipt_path.to_string_lossy().into_owned();
    let verify = run_cli(&["market", "verify", "--receipt", &receipt_arg])?;
    assert!(
        verify.status.success(),
        "receipt verify failed: {}",
        String::from_utf8_lossy(&verify.stderr)
    );
    let verified: Value = serde_json::from_slice(&verify.stdout)?;
    assert_eq!(verified["status"], "OFFLINE_CONTRACT_CHECK_PASS");
    assert_eq!(verified["receipt_digest"], summary["receipt_digest"]);

    fs::remove_file(&receipt_path)?;
    let recovered_path = temporary.path().join("recovered-market-receipt.json");
    let journal_arg = journal_path.to_string_lossy().into_owned();
    let recovered_arg = recovered_path.to_string_lossy().into_owned();
    let recover = run_cli(&[
        "market",
        "recover",
        "--journal",
        &journal_arg,
        "--output",
        &recovered_arg,
    ])?;
    assert!(
        recover.status.success(),
        "receipt recovery failed: {}",
        String::from_utf8_lossy(&recover.stderr)
    );
    assert_eq!(fs::read(&recovered_path)?, exported_receipt);

    let recovered_verify = run_cli(&["market", "verify", "--receipt", &recovered_arg])?;
    assert!(
        recovered_verify.status.success(),
        "recovered receipt verify failed: {}",
        String::from_utf8_lossy(&recovered_verify.stderr)
    );

    let duplicate_recovery = run_cli(&[
        "market",
        "recover",
        "--journal",
        &journal_arg,
        "--output",
        &recovered_arg,
    ])?;
    assert!(!duplicate_recovery.status.success());
    assert_eq!(fs::read(&recovered_path)?, exported_receipt);

    let duplicate_demo = run_cli(&["market", "demo", "--output-dir", &output_dir_arg])?;
    assert!(!duplicate_demo.status.success());
    assert_eq!(
        fs::read(output_dir.join("market-journal.json"))?,
        fs::read(&journal_path)?
    );
    Ok(())
}
