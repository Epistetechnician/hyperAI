//! Data-only subprocess quarantine validation.

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HarnessError {
    #[error("subprocess evidence must be written below a fresh directory")]
    NotFreshDirectory,
    #[error("accepted_evidence_write must remain false")]
    AcceptedEvidenceWrite,
    #[error("rejected outcomes must be observable")]
    RejectedOutcomeNotObservable,
    #[error("missing subprocess artifact digest: {0}")]
    MissingDigest(&'static str),
    #[error("invalid subprocess field: {0}")]
    InvalidField(&'static str),
    #[error("subprocess artifact digest does not match supplied bytes: {0}")]
    ArtifactDigestMismatch(&'static str),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuarantineManifest {
    pub state_slice: String,
    pub run_id: String,
    pub fresh_directory_ref: String,
    pub fresh_directory: bool,
    pub accepted_evidence_write: bool,
    pub outcome: String,
    pub stdout_digest: String,
    pub stderr_digest: String,
    pub manifest_digest: String,
}

pub fn validate_quarantine_manifest(manifest: &QuarantineManifest) -> Result<(), HarnessError> {
    validate_quarantine_artifacts(manifest, None, None)
}

pub fn validate_quarantine_artifacts(
    manifest: &QuarantineManifest,
    stdout: Option<&[u8]>,
    stderr: Option<&[u8]>,
) -> Result<(), HarnessError> {
    if manifest.state_slice != "justai-operations-v1" {
        return Err(HarnessError::InvalidField("state_slice"));
    }
    if manifest.run_id.trim().is_empty() {
        return Err(HarnessError::InvalidField("run_id"));
    }
    if manifest.fresh_directory_ref.trim().is_empty() {
        return Err(HarnessError::InvalidField("fresh_directory_ref"));
    }
    if !manifest.fresh_directory {
        return Err(HarnessError::NotFreshDirectory);
    }
    if manifest.accepted_evidence_write {
        return Err(HarnessError::AcceptedEvidenceWrite);
    }
    if manifest.outcome.trim().is_empty() {
        return Err(HarnessError::RejectedOutcomeNotObservable);
    }
    for (name, value) in [
        ("stdout_digest", &manifest.stdout_digest),
        ("stderr_digest", &manifest.stderr_digest),
        ("manifest_digest", &manifest.manifest_digest),
    ] {
        if !is_digest(value) {
            return Err(HarnessError::MissingDigest(name));
        }
    }
    if let Some(stdout) = stdout {
        if digest(stdout) != manifest.stdout_digest {
            return Err(HarnessError::ArtifactDigestMismatch("stdout_digest"));
        }
    }
    if let Some(stderr) = stderr {
        if digest(stderr) != manifest.stderr_digest {
            return Err(HarnessError::ArtifactDigestMismatch("stderr_digest"));
        }
    }
    if manifest_digest(manifest) != manifest.manifest_digest {
        return Err(HarnessError::ArtifactDigestMismatch("manifest_digest"));
    }
    Ok(())
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn digest(value: &[u8]) -> String {
    hex::encode(Sha256::digest(value))
}

pub fn manifest_digest(manifest: &QuarantineManifest) -> String {
    let body = json!({
        "state_slice": manifest.state_slice,
        "run_id": manifest.run_id,
        "fresh_directory": manifest.fresh_directory,
        "fresh_directory_ref": manifest.fresh_directory_ref,
        "accepted_evidence_write": manifest.accepted_evidence_write,
        "outcome": manifest.outcome,
        "stdout_digest": manifest.stdout_digest,
        "stderr_digest": manifest.stderr_digest,
    });
    let canonical = serde_jcs::to_vec(&body).expect("manifest fields are serializable");
    digest(&canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> QuarantineManifest {
        QuarantineManifest {
            state_slice: "justai-operations-v1".into(),
            run_id: "run-1".into(),
            fresh_directory_ref: "fresh-dir-attestation-1".into(),
            fresh_directory: true,
            accepted_evidence_write: false,
            outcome: "rejected".into(),
            stdout_digest: "a".repeat(64),
            stderr_digest: "b".repeat(64),
            manifest_digest: "".into(),
        }
    }

    #[test]
    fn keeps_rejected_subprocess_outcomes_observable_and_unaccepted() {
        let mut value = manifest();
        value.manifest_digest = manifest_digest(&value);
        assert!(validate_quarantine_manifest(&value).is_ok());
        assert!(validate_quarantine_artifacts(&value, Some(b"out"), Some(b"err")).is_err());
    }

    #[test]
    fn rejects_nonfresh_or_accepted_evidence_manifests() {
        let mut value = manifest();
        value.fresh_directory = false;
        assert_eq!(
            validate_quarantine_manifest(&value),
            Err(HarnessError::NotFreshDirectory)
        );
        value.fresh_directory = true;
        value.accepted_evidence_write = true;
        assert_eq!(
            validate_quarantine_manifest(&value),
            Err(HarnessError::AcceptedEvidenceWrite)
        );
    }
}
