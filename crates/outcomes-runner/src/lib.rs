//! Admission-aware validator execution, normalization, and replay ports.

use std::collections::BTreeMap;

use outcomes_domain::{
    AdmissionDecisionKind, AdmissionDecisionV1, CandidateEnvelopeV1, ClaimSpecV1, DeterminismTier,
    ExecutionManifestV1, ExecutionOutcome, ResourceUsageV1, Sha256Digest, UtcTimestamp,
    domain_separated_digest,
};
use sha2::{Digest as _, Sha256};
use thiserror::Error;

/// Runner failures are infrastructure states, never a false claim result.
#[derive(Debug, Error)]
pub enum RunnerError {
    /// Admission decision was deny or manual review.
    #[error("execution blocked by admission decision {0:?}")]
    AdmissionBlocked(AdmissionDecisionKind),
    /// Candidate bytes do not match their immutable envelope commitment.
    #[error("candidate artifact length does not match the envelope")]
    ArtifactLengthMismatch,
    /// A built-in runner record could not be canonicalized.
    #[error("runner record is not canonicalizable: {0}")]
    Canonicalization(#[from] outcomes_domain::CanonicalizationError),
    /// Scripted test runner had no configured outcome remaining.
    #[error("scripted runner exhausted configured outcomes")]
    ScriptExhausted,
    /// OCI policy is insufficient for untrusted execution.
    #[error("unsafe OCI runner policy: {0}")]
    UnsafeOciPolicy(String),
    /// Local OCI execution is unavailable and was not falsely simulated.
    #[error("OCI runner unavailable: {0}")]
    OciUnavailable(String),
}

/// Immutable input to one verification attempt.
#[derive(Debug, Clone)]
pub struct RunnerRequest<'a> {
    /// Frozen claim semantics.
    pub claim: &'a ClaimSpecV1,
    /// Candidate envelope.
    pub candidate: &'a CandidateEnvelopeV1,
    /// Untrusted candidate bytes.
    pub candidate_bytes: &'a [u8],
    /// Admission decision authorizing this exact execution.
    pub admission: &'a AdmissionDecisionV1,
    /// Unique run identifier.
    pub run_id: &'a str,
    /// Receipt start time.
    pub started_at: &'a UtcTimestamp,
    /// Receipt end time.
    pub ended_at: &'a UtcTimestamp,
    /// Frozen logical time injected into the validator.
    pub logical_time: &'a UtcTimestamp,
}

/// Versioned runner port. Implementations return typed manifests, not log guesses.
pub trait VerificationRunner {
    /// Execute one admitted attempt.
    ///
    /// # Errors
    ///
    /// Returns a runner error for blocked admission, exhausted test scripts, or
    /// unavailable/unsafe execution infrastructure.
    fn execute(&mut self, request: &RunnerRequest<'_>) -> Result<ExecutionManifestV1, RunnerError>;
}

/// Deterministic runner used by hermetic contract and end-to-end tests.
#[derive(Debug, Clone)]
pub struct ScriptedRunner {
    outcomes: Vec<ExecutionOutcome>,
    next: usize,
}

impl ScriptedRunner {
    /// Construct a runner that emits the supplied normalized outcomes in order.
    #[must_use]
    pub fn new(outcomes: Vec<ExecutionOutcome>) -> Self {
        Self { outcomes, next: 0 }
    }

    /// Construct the deterministic local exact-bytes validator.
    #[must_use]
    pub fn exact_bytes_fixture() -> Self {
        Self::new(Vec::new())
    }
}

impl VerificationRunner for ScriptedRunner {
    fn execute(&mut self, request: &RunnerRequest<'_>) -> Result<ExecutionManifestV1, RunnerError> {
        if request.admission.decision != AdmissionDecisionKind::Allow {
            return Err(RunnerError::AdmissionBlocked(request.admission.decision));
        }
        if request.candidate.artifact_byte_length != request.candidate_bytes.len() as u64 {
            return Err(RunnerError::ArtifactLengthMismatch);
        }
        if plain_digest(request.candidate_bytes) != request.candidate.artifact_digest {
            return Err(RunnerError::ArtifactLengthMismatch);
        }
        let outcome = if self.outcomes.is_empty() {
            match request.candidate_bytes {
                b"hsai-pass-v1" => ExecutionOutcome::Pass,
                b"hsai-inconclusive-v1" => ExecutionOutcome::Inconclusive,
                b"hsai-error-v1" => ExecutionOutcome::Error,
                _ => ExecutionOutcome::Fail,
            }
        } else {
            let outcome = self
                .outcomes
                .get(self.next)
                .copied()
                .ok_or(RunnerError::ScriptExhausted)?;
            self.next += 1;
            outcome
        };
        build_manifest(request, outcome)
    }
}

/// Result of comparing the configured replay attempts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayAssessment {
    /// Whether all normalized outcomes agree.
    pub agreed: bool,
    /// Determinism tier actually established.
    pub established_tier: Option<DeterminismTier>,
    /// Agreed outcome, absent when attempts diverge or are empty.
    pub normalized_outcome: Option<ExecutionOutcome>,
}

/// Compare normalized attempts under the frozen determinism tier.
#[must_use]
pub fn assess_replays(
    manifests: &[ExecutionManifestV1],
    required_tier: DeterminismTier,
) -> ReplayAssessment {
    let first = manifests
        .first()
        .map(|manifest| manifest.normalized_outcome);
    let agreed = first.is_some()
        && manifests
            .iter()
            .all(|manifest| Some(manifest.normalized_outcome) == first);
    ReplayAssessment {
        agreed,
        established_tier: agreed.then_some(required_tier),
        normalized_outcome: agreed.then_some(first).flatten(),
    }
}

/// Create an admission record for the exact claim, candidate, and runner policy.
///
/// Deny and manual-review decisions remain evidence; they never imply false.
///
/// # Errors
///
/// Returns an error if the decision record cannot be canonicalized.
pub fn evaluate_admission(
    claim: &ClaimSpecV1,
    candidate: &CandidateEnvelopeV1,
) -> Result<AdmissionDecisionV1, RunnerError> {
    let claim_digest = claim.claim_spec_digest().map_err(|error| {
        RunnerError::UnsafeOciPolicy(format!(
            "invalid frozen claim supplied to admission: {error}"
        ))
    })?;
    let media_allowed = claim
        .eligible_submission_policy
        .allowed_artifact_media_types
        .contains(&candidate.artifact_media_type);
    let size_allowed =
        candidate.artifact_byte_length <= claim.eligible_submission_policy.max_artifact_bytes;
    let claim_matches = candidate.claim_id == claim.claim_id;
    let decision = if media_allowed && size_allowed && claim_matches {
        AdmissionDecisionKind::Allow
    } else {
        AdmissionDecisionKind::Deny
    };
    let reason_codes = [
        (!media_allowed).then_some("ARTIFACT_MEDIA_TYPE_DENIED"),
        (!size_allowed).then_some("ARTIFACT_TOO_LARGE"),
        (!claim_matches).then_some("CLAIM_ID_MISMATCH"),
    ]
    .into_iter()
    .flatten()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    let artifact_policy_digest =
        domain_separated_digest("hsai.artifact-policy.v1", &claim.eligible_submission_policy)?;
    let sandbox_policy_digest =
        domain_separated_digest("hsai.sandbox-policy.v1", &claim.verification_policy)?;
    let resource_budget_digest = domain_separated_digest(
        "hsai.resource-budget.v1",
        &claim.verification_policy.resource_limits,
    )?;
    let secrets_policy_digest =
        domain_separated_digest("hsai.secrets-policy.v1", &"no-ambient-secrets")?;
    let decision_payload = (
        claim_digest,
        candidate.artifact_digest,
        claim.verification_policy.validator_artifact_digest,
        claim.verification_policy.environment_digest,
        decision,
        &reason_codes,
    );
    let decision_digest =
        domain_separated_digest("hsai.execution-admission.v1", &decision_payload)?;
    Ok(AdmissionDecisionV1 {
        schema_version: "hsai.execution-admission.v1".to_owned(),
        decision_id: format!("admission_{}", &decision_digest.to_hex()[..32]),
        claim_spec_digest: claim_digest,
        candidate_digest: candidate.artifact_digest,
        validator_digest: claim.verification_policy.validator_artifact_digest,
        environment_digest: claim.verification_policy.environment_digest,
        source_provenance_status: "CONTENT_DIGEST_BOUND".to_owned(),
        artifact_policy_digest,
        sandbox_policy_digest,
        resource_budget_digest,
        secrets_policy_digest,
        risk_classification: "UNTRUSTED_CODE_OR_DATA".to_owned(),
        decision,
        reason_codes,
        policy_version: "hsai.execution-admission.v1".to_owned(),
        decision_digest,
    })
}

/// Minimum OCI controls required by the production-capable runner boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
// These booleans are explicit independently auditable security controls.
#[allow(clippy::struct_excessive_bools)]
pub struct OciSandboxPolicy {
    /// Immutable `sha256:` image reference.
    pub image_digest: String,
    /// Container UID; zero is forbidden.
    pub user_id: u32,
    /// Whether the root filesystem is read-only.
    pub read_only_root: bool,
    /// Whether all Linux capabilities are dropped.
    pub drop_all_capabilities: bool,
    /// Whether networking is disabled.
    pub network_disabled: bool,
    /// Whether the host or Docker socket is mounted.
    pub host_socket_mounted: bool,
    /// Structured argv passed without a shell.
    pub argv: Vec<String>,
    /// Explicit environment allowlist.
    pub environment_allowlist: Vec<String>,
}

/// Validate the fail-closed OCI policy before any process is spawned.
///
/// # Errors
///
/// Returns an exact unsafe-policy reason when a required control is absent.
pub fn validate_oci_policy(policy: &OciSandboxPolicy) -> Result<(), RunnerError> {
    if !policy.image_digest.starts_with("sha256:") || policy.image_digest.len() != 71 {
        return Err(RunnerError::UnsafeOciPolicy(
            "image must be pinned by a 64-hex SHA-256 digest".to_owned(),
        ));
    }
    if policy.user_id == 0 {
        return Err(RunnerError::UnsafeOciPolicy(
            "container user must be non-root".to_owned(),
        ));
    }
    if !policy.read_only_root || !policy.drop_all_capabilities || !policy.network_disabled {
        return Err(RunnerError::UnsafeOciPolicy(
            "read-only root, dropped capabilities, and disabled network are required".to_owned(),
        ));
    }
    if policy.host_socket_mounted {
        return Err(RunnerError::UnsafeOciPolicy(
            "host and Docker socket mounts are forbidden".to_owned(),
        ));
    }
    if policy.argv.is_empty() || policy.argv.iter().any(String::is_empty) {
        return Err(RunnerError::UnsafeOciPolicy(
            "structured non-empty argv is required".to_owned(),
        ));
    }
    Ok(())
}

fn build_manifest(
    request: &RunnerRequest<'_>,
    outcome: ExecutionOutcome,
) -> Result<ExecutionManifestV1, RunnerError> {
    let stdout_digest = domain_separated_digest("hsai.runner-stdout.v1", &format!("{outcome:?}"))?;
    let stderr_digest = domain_separated_digest("hsai.runner-stderr.v1", &"")?;
    let input_manifest_digest = domain_separated_digest(
        "hsai.runner-input-manifest.v1",
        &(
            request.candidate.artifact_digest,
            request.candidate.artifact_byte_length,
        ),
    )?;
    let output_manifest_digest = domain_separated_digest(
        "hsai.runner-output-manifest.v1",
        &(outcome, stdout_digest, stderr_digest),
    )?;
    Ok(ExecutionManifestV1 {
        schema_version: "hsai.execution-manifest.v1".to_owned(),
        run_id: request.run_id.to_owned(),
        claim_spec_digest: request.claim.claim_spec_digest().map_err(|error| {
            RunnerError::UnsafeOciPolicy(format!(
                "invalid frozen claim supplied to runner: {error}"
            ))
        })?,
        candidate_digest: request.candidate.artifact_digest,
        admission_decision_digest: request.admission.decision_digest,
        verifier_digest: request.claim.verification_policy.validator_artifact_digest,
        environment_digest: request.claim.verification_policy.environment_digest,
        runner_implementation: "hsai-scripted-runner".to_owned(),
        runner_build_version: env!("CARGO_PKG_VERSION").to_owned(),
        host_compatibility_facts: BTreeMap::from([(
            "execution_profile".to_owned(),
            "hermetic-scripted-fixture".to_owned(),
        )]),
        argv: request.claim.verification_policy.argv.clone(),
        environment: BTreeMap::from([
            (
                "LANG".to_owned(),
                request.claim.verification_policy.locale.clone(),
            ),
            (
                "TZ".to_owned(),
                request.claim.verification_policy.timezone.clone(),
            ),
        ]),
        sandbox_policy_digest: request.admission.sandbox_policy_digest,
        input_manifest_digest,
        output_manifest_digest: Some(output_manifest_digest),
        resource_limits_digest: request.admission.resource_budget_digest,
        resource_usage: ResourceUsageV1 {
            cpu_millis: 1,
            peak_memory_bytes: 1_048_576,
            output_bytes: 16,
            disk_bytes: 0,
            wall_clock_millis: 1,
        },
        started_at: request.started_at.clone(),
        ended_at: request.ended_at.clone(),
        logical_time: request.logical_time.clone(),
        raw_exit_status: (outcome != ExecutionOutcome::Error).then_some(0),
        signal: None,
        timed_out: false,
        out_of_memory: false,
        infrastructure_classification: (outcome == ExecutionOutcome::Error)
            .then(|| "INJECTED_RUNNER_ERROR".to_owned()),
        stdout_digest,
        stderr_digest,
        normalized_outcome: outcome,
        normalization_policy_version: request
            .claim
            .verification_policy
            .output_normalization_version
            .clone(),
        determinism_tier: request.claim.verification_policy.determinism_tier,
        declared_nondeterminism: Vec::new(),
    })
}

fn plain_digest(bytes: &[u8]) -> Sha256Digest {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Sha256Digest::from_bytes(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use outcomes_domain::{ClaimId, fixtures::local_demo_claim_draft};

    use super::*;

    fn timestamp(value: &str) -> Result<UtcTimestamp, outcomes_domain::TimestampError> {
        value.parse()
    }

    fn candidate(claim_id: ClaimId, bytes: &[u8]) -> Result<CandidateEnvelopeV1, RunnerError> {
        let digest = plain_digest(bytes);
        Ok(CandidateEnvelopeV1 {
            schema_version: "hsai.candidate-envelope.v1".to_owned(),
            candidate_id: "candidate-test".to_owned(),
            claim_id,
            source_namespace: "local.demo".to_owned(),
            source_candidate_ref: "submission-1".to_owned(),
            source_revision: "1".to_owned(),
            author_or_team_id: None,
            observed_source_time: timestamp("2026-07-10T10:30:00Z").map_err(|error| {
                RunnerError::UnsafeOciPolicy(format!("bad test timestamp: {error}"))
            })?,
            received_at: timestamp("2026-07-10T10:30:01Z").map_err(|error| {
                RunnerError::UnsafeOciPolicy(format!("bad test timestamp: {error}"))
            })?,
            artifact_digest: digest,
            artifact_byte_length: bytes.len() as u64,
            artifact_media_type: "application/octet-stream".to_owned(),
            source_provenance_digest: digest,
        })
    }

    #[test]
    fn contract_pass_fail_and_error_are_distinct() -> Result<(), Box<dyn std::error::Error>> {
        let claim = local_demo_claim_draft()?.freeze()?;
        for (index, (bytes, expected)) in [
            (b"hsai-pass-v1".as_slice(), ExecutionOutcome::Pass),
            (b"other".as_slice(), ExecutionOutcome::Fail),
            (b"hsai-error-v1".as_slice(), ExecutionOutcome::Error),
        ]
        .into_iter()
        .enumerate()
        {
            let candidate = candidate(claim.claim_id.clone(), bytes)?;
            let admission = evaluate_admission(&claim, &candidate)?;
            let started = timestamp("2026-07-10T10:31:00Z")?;
            let ended = timestamp("2026-07-10T10:31:01Z")?;
            let mut runner = ScriptedRunner::exact_bytes_fixture();
            let manifest = runner.execute(&RunnerRequest {
                claim: &claim,
                candidate: &candidate,
                candidate_bytes: bytes,
                admission: &admission,
                run_id: &format!("run-{index}"),
                started_at: &started,
                ended_at: &ended,
                logical_time: &started,
            })?;
            assert_eq!(manifest.normalized_outcome, expected);
        }
        Ok(())
    }

    #[test]
    fn contract_denied_admission_blocks_runner() -> Result<(), Box<dyn std::error::Error>> {
        let claim = local_demo_claim_draft()?.freeze()?;
        let mut candidate = candidate(claim.claim_id.clone(), b"hsai-pass-v1")?;
        candidate.artifact_media_type = "application/x-denied".to_owned();
        let admission = evaluate_admission(&claim, &candidate)?;
        assert_eq!(admission.decision, AdmissionDecisionKind::Deny);
        let started = timestamp("2026-07-10T10:31:00Z")?;
        let ended = timestamp("2026-07-10T10:31:01Z")?;
        let mut runner = ScriptedRunner::exact_bytes_fixture();
        assert!(matches!(
            runner.execute(&RunnerRequest {
                claim: &claim,
                candidate: &candidate,
                candidate_bytes: b"hsai-pass-v1",
                admission: &admission,
                run_id: "run-denied",
                started_at: &started,
                ended_at: &ended,
                logical_time: &started,
            }),
            Err(RunnerError::AdmissionBlocked(AdmissionDecisionKind::Deny))
        ));
        Ok(())
    }

    #[test]
    fn contract_divergent_replays_do_not_establish_outcome()
    -> Result<(), Box<dyn std::error::Error>> {
        let claim = local_demo_claim_draft()?.freeze()?;
        let candidate = candidate(claim.claim_id.clone(), b"hsai-pass-v1")?;
        let admission = evaluate_admission(&claim, &candidate)?;
        let started = timestamp("2026-07-10T10:31:00Z")?;
        let ended = timestamp("2026-07-10T10:31:01Z")?;
        let mut runner = ScriptedRunner::new(vec![ExecutionOutcome::Pass, ExecutionOutcome::Fail]);
        let mut manifests = Vec::new();
        for index in 0..2 {
            manifests.push(runner.execute(&RunnerRequest {
                claim: &claim,
                candidate: &candidate,
                candidate_bytes: b"hsai-pass-v1",
                admission: &admission,
                run_id: &format!("run-{index}"),
                started_at: &started,
                ended_at: &ended,
                logical_time: &started,
            })?);
        }
        let assessment = assess_replays(&manifests, DeterminismTier::SemanticIdentical);
        assert!(!assessment.agreed);
        assert_eq!(assessment.normalized_outcome, None);
        Ok(())
    }

    #[test]
    fn contract_oci_policy_rejects_root_and_socket_mounts() {
        let unsafe_policy = OciSandboxPolicy {
            image_digest: format!("sha256:{}", "a".repeat(64)),
            user_id: 0,
            read_only_root: true,
            drop_all_capabilities: true,
            network_disabled: true,
            host_socket_mounted: true,
            argv: vec!["/validator/check".to_owned()],
            environment_allowlist: Vec::new(),
        };
        assert!(validate_oci_policy(&unsafe_policy).is_err());
    }
}
