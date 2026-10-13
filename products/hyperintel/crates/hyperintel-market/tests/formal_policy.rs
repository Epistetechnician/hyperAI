//! Offline environment-pinned process gate for the source-bound abstract routing property.

use std::{
    io::Write as _,
    process::{Command, Stdio},
};

use data_encoding::HEXLOWER;
use sha2::{Digest as _, Sha256};

const ROUTING_SOURCE_SHA256: &str =
    "a0ea4bc9129b509692ac3d4b55e215ba3514056cb09c270cdbaf95142d1bb425";
const PINNED_Z3: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/pinned-z3");
const Z3_VERSION: &str = "Z3 version 4.15.0 - 64 bit\n";

#[test]
fn contract_policy_non_widening_model_is_unsat_and_weakened_replay_is_sat()
-> Result<(), Box<dyn std::error::Error>> {
    let routing_source = include_bytes!("../src/routing.rs");
    let source_digest = HEXLOWER.encode(&Sha256::digest(routing_source));
    assert_eq!(source_digest, ROUTING_SOURCE_SHA256);

    let version = Command::new(PINNED_Z3).arg("--version").output()?;
    assert!(version.status.success());
    assert!(version.stderr.is_empty());
    assert_eq!(String::from_utf8(version.stdout)?, Z3_VERSION);

    let mut child = Command::new(PINNED_Z3)
        .args(["-in", "-smt2", "-T:5"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("Z3 stdin was not piped")?;
    stdin.write_all(include_bytes!("../formal/policy_non_widening_v1.smt2"))?;
    drop(stdin);
    let output = child.wait_with_output()?;

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(String::from_utf8(output.stdout)?, "unsat\nsat\n");
    Ok(())
}
