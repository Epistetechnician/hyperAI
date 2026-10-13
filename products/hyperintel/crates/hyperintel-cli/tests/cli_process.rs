//! Real-process contract coverage for the offline verifier boundary.

use std::{path::PathBuf, process::Command};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn run_cli(arguments: &[PathBuf]) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_hyperintel-cli"))
        .args(arguments)
        .output()?)
}

#[test]
fn contract_cli_process_verifies_v1_through_v4_with_separate_caller_trust()
-> Result<(), Box<dyn std::error::Error>> {
    for (evidence, trust, bundle_digest) in [
        (
            "evidence-v1.json",
            "trusted-keys-v1-v2.json",
            "7614469c8d4febc50753d52d34741bc70cc3bcc4954c6ea93b1cb8621c157785",
        ),
        (
            "evidence-v2.json",
            "trusted-keys-v1-v2.json",
            "781fab0921e649922f53ceb67f9037ddfd0f039922d3663ef54b29f7bd042cab",
        ),
        (
            "evidence-v3.json",
            "trusted-keys-v4.json",
            "59db4b80a9fec85a2fa8e72d7e6cb36d446918c8a85a896883be7d83d50c8fbf",
        ),
        (
            "evidence-v4.json",
            "trusted-keys-v4.json",
            "05397586672fb65f764812cf45176653caabd7a316968f268d5111a4a6a2daa8",
        ),
    ] {
        let output = run_cli(&[fixture_path(evidence), fixture_path(trust)])?;
        assert!(output.status.success(), "{evidence}");
        assert_eq!(
            String::from_utf8(output.stdout)?,
            format!("verified market evidence bundle {bundle_digest}\n")
        );
        assert!(output.stderr.is_empty(), "{evidence}");
    }
    Ok(())
}

#[test]
fn contract_cli_process_rejects_missing_trust_and_unknown_schema()
-> Result<(), Box<dyn std::error::Error>> {
    let untrusted = run_cli(&[
        fixture_path("evidence-v4.json"),
        fixture_path("empty-trusted-keys.json"),
    ])?;
    assert!(!untrusted.status.success());
    assert!(untrusted.stdout.is_empty());
    assert!(String::from_utf8(untrusted.stderr)?.contains(
        "hyperintel verification failed: bundle verification key is not an exact caller-trusted key"
    ));

    let unsupported = run_cli(&[
        fixture_path("unsupported-v5.json"),
        fixture_path("trusted-keys-v4.json"),
    ])?;
    assert!(!unsupported.status.success());
    assert!(unsupported.stdout.is_empty());
    assert_eq!(
        String::from_utf8(unsupported.stderr)?,
        "hyperintel verification failed: unsupported market evidence bundle schema\n"
    );
    Ok(())
}

#[test]
fn contract_cli_process_rejects_invalid_argument_and_file_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let no_arguments = run_cli(&[])?;
    assert!(!no_arguments.status.success());
    assert!(String::from_utf8(no_arguments.stderr)?.contains("usage: "));

    let one_argument = run_cli(&[fixture_path("evidence-v4.json")])?;
    assert!(!one_argument.status.success());
    assert_eq!(
        String::from_utf8(one_argument.stderr)?,
        "hyperintel verification failed: missing caller-controlled trusted-key file\n"
    );

    let extra_argument = run_cli(&[
        fixture_path("evidence-v4.json"),
        fixture_path("trusted-keys-v4.json"),
        fixture_path("empty-trusted-keys.json"),
    ])?;
    assert!(!extra_argument.status.success());
    assert_eq!(
        String::from_utf8(extra_argument.stderr)?,
        "hyperintel verification failed: expected exactly two input paths\n"
    );

    let missing_file = run_cli(&[
        fixture_path("absent-evidence.json"),
        fixture_path("trusted-keys-v4.json"),
    ])?;
    assert!(!missing_file.status.success());
    assert!(missing_file.stdout.is_empty());
    assert!(
        String::from_utf8(missing_file.stderr)?.starts_with("hyperintel verification failed: ")
    );
    Ok(())
}
