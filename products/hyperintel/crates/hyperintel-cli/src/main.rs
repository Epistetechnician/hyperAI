//! Offline verifier for a portable `HyperIntel` market evidence bundle.

use std::{collections::BTreeMap, env, fs, path::PathBuf, process::ExitCode};

use hyperintel_assurance::{RegisteredVerificationKeyV1, VerificationKeyRegistry};
use hyperintel_core::Sha256Digest;
use hyperintel_market::{
    MarketEvidenceBundleV1, ReliabilityArtifactVerifiedMarketEvidenceBundleV3,
    ReplayProtectedMarketEvidenceBundleV4, SignedMarketEvidenceBundleV2,
    SignedReliabilityArtifactV1, verify_market_evidence_bundle_v1,
    verify_reliability_artifact_verified_market_evidence_bundle_v3,
    verify_replay_protected_market_evidence_bundle_v4, verify_signed_market_evidence_bundle_v2,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationInputV1 {
    bundle: MarketEvidenceBundleV1,
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationInputV2 {
    bundle: SignedMarketEvidenceBundleV2,
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationInputV3 {
    bundle: ReliabilityArtifactVerifiedMarketEvidenceBundleV3,
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
    reliability_artifacts: BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationInputV4 {
    bundle: ReplayProtectedMarketEvidenceBundleV4,
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
    reliability_artifacts: BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
}

enum VerificationInput {
    ReplayProtectedV4(Box<VerificationInputV4>),
    ReliabilityV3(Box<VerificationInputV3>),
    SignedV2(Box<VerificationInputV2>),
    LegacyV1(Box<VerificationInputV1>),
}

#[derive(Deserialize)]
struct VerificationInputProbe {
    bundle: BundleSchemaProbe,
}

#[derive(Deserialize)]
struct BundleSchemaProbe {
    schema_version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrustInputV1 {
    trusted_verification_keys: Vec<RegisteredVerificationKeyV1>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("hyperintel verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = env::args_os();
    let program = arguments.next().unwrap_or_else(|| "hyperintel-cli".into());
    let evidence_path = arguments.next().map(PathBuf::from).ok_or_else(|| {
        format!(
            "usage: {} <evidence-input.json> <trusted-keys.json>",
            PathBuf::from(program).display()
        )
    })?;
    let trust_path = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("missing caller-controlled trusted-key file")?;
    if arguments.next().is_some() {
        return Err("expected exactly two input paths".into());
    }

    let input_bytes = fs::read(evidence_path)?;
    let input = decode_verification_input(&input_bytes)?;
    let trust_bytes = fs::read(trust_path)?;
    let trust: TrustInputV1 = serde_json::from_slice(&trust_bytes)?;
    let mut trusted_registry = VerificationKeyRegistry::default();
    for key in trust.trusted_verification_keys {
        trusted_registry.register(key)?;
    }
    let bundle_digest = match input {
        VerificationInput::ReplayProtectedV4(input) => {
            verify_replay_protected_market_evidence_bundle_v4(
                &input.bundle,
                &input.assurance_blobs,
                &input.reliability_artifacts,
                &trusted_registry,
            )?;
            input.bundle.bundle_digest
        }
        VerificationInput::ReliabilityV3(input) => {
            verify_reliability_artifact_verified_market_evidence_bundle_v3(
                &input.bundle,
                &input.assurance_blobs,
                &input.reliability_artifacts,
                &trusted_registry,
            )?;
            input.bundle.bundle_digest
        }
        VerificationInput::SignedV2(input) => {
            verify_signed_market_evidence_bundle_v2(
                &input.bundle,
                &input.assurance_blobs,
                &trusted_registry,
            )?;
            input.bundle.bundle_digest
        }
        VerificationInput::LegacyV1(input) => {
            verify_market_evidence_bundle_v1(
                &input.bundle,
                &input.assurance_blobs,
                &trusted_registry,
            )?;
            input.bundle.bundle_digest
        }
    };
    println!("verified market evidence bundle {bundle_digest}");
    Ok(())
}

fn decode_verification_input(
    input_bytes: &[u8],
) -> Result<VerificationInput, Box<dyn std::error::Error>> {
    let probe: VerificationInputProbe = serde_json::from_slice(input_bytes)?;
    match probe.bundle.schema_version.as_str() {
        "hyperintel.market-evidence-bundle.v4" => Ok(VerificationInput::ReplayProtectedV4(
            Box::new(serde_json::from_slice(input_bytes)?),
        )),
        "hyperintel.market-evidence-bundle.v3" => Ok(VerificationInput::ReliabilityV3(Box::new(
            serde_json::from_slice(input_bytes)?,
        ))),
        "hyperintel.market-evidence-bundle.v2" => Ok(VerificationInput::SignedV2(Box::new(
            serde_json::from_slice(input_bytes)?,
        ))),
        "hyperintel.market-evidence-bundle.v1" => Ok(VerificationInput::LegacyV1(Box::new(
            serde_json::from_slice(input_bytes)?,
        ))),
        _ => Err("unsupported market evidence bundle schema".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_cli_dispatches_only_explicit_v1_through_v4_bundle_schemas() {
        let unknown = br#"{"bundle":{"schema_version":"hyperintel.market-evidence-bundle.v5"}}"#;
        assert!(
            decode_verification_input(unknown).is_err_and(
                |error| error.to_string() == "unsupported market evidence bundle schema"
            )
        );

        let incomplete_v1 =
            br#"{"bundle":{"schema_version":"hyperintel.market-evidence-bundle.v1"}}"#;
        let incomplete_v2 =
            br#"{"bundle":{"schema_version":"hyperintel.market-evidence-bundle.v2"}}"#;
        let incomplete_v3 =
            br#"{"bundle":{"schema_version":"hyperintel.market-evidence-bundle.v3"}}"#;
        let incomplete_v4 =
            br#"{"bundle":{"schema_version":"hyperintel.market-evidence-bundle.v4"}}"#;
        assert!(
            decode_verification_input(incomplete_v1).is_err_and(
                |error| error.to_string() != "unsupported market evidence bundle schema"
            )
        );
        assert!(
            decode_verification_input(incomplete_v2).is_err_and(
                |error| error.to_string() != "unsupported market evidence bundle schema"
            )
        );
        assert!(
            decode_verification_input(incomplete_v3).is_err_and(
                |error| error.to_string() != "unsupported market evidence bundle schema"
            )
        );
        assert!(
            decode_verification_input(incomplete_v4).is_err_and(
                |error| error.to_string() != "unsupported market evidence bundle schema"
            )
        );
    }
}
