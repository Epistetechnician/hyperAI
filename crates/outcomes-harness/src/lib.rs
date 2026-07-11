//! Agent-harness plans, traces, candidates, and cost telemetry ports.

use outcomes_domain::{
    AgentHarnessDescriptorV1, AgentRunPlanV1, AgentRunResultV1, AgentTraceBundleV1,
    ChallengePackageV1, Sha256Digest, SolverViewManifestV1, UtcTimestamp, domain_separated_digest,
};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use thiserror::Error;

/// Candidate bytes extracted from a non-authoritative agent run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessCandidate {
    /// Harness-local source reference.
    pub source_ref: String,
    /// Untrusted bytes that must pass the HSAI verifier.
    pub bytes: Vec<u8>,
    /// Plain content digest.
    pub digest: Sha256Digest,
}

/// Harness failures.
#[derive(Debug, Error)]
pub enum HarnessError {
    /// Solver view was not bound to the selected challenge package.
    #[error("solver view is not bound to the challenge package")]
    SolverViewMismatch,
    /// Live model execution was requested without explicit authorization.
    #[error("live model execution requires explicit authorization")]
    LiveModelNotAuthorized,
    /// Run plan targets another harness.
    #[error("run plan targets another harness")]
    HarnessMismatch,
    /// Payload could not be canonicalized.
    #[error("harness payload is not canonicalizable: {0}")]
    Canonicalization(#[from] outcomes_domain::CanonicalizationError),
    /// Bundled harness-profile JSON was invalid.
    #[error("harness profile JSON is invalid: {0}")]
    ProfileJson(#[from] serde_json::Error),
    /// Harness profile did not preserve verifier authority or immutable pinning.
    #[error("unsafe harness profile {0}")]
    UnsafeProfile(String),
}

/// Pinned external harness profile used only through the adapter port.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessProfile {
    /// Stable harness adapter ID.
    pub profile_id: String,
    /// Official public repository.
    pub repository: String,
    /// Immutable 40-hex Git commit.
    pub commit: String,
    /// Supported adapter entrypoints.
    pub entrypoints: Vec<String>,
    /// Default offline execution mode.
    pub default_mode: String,
    /// Whether live model execution is enabled by default.
    pub live_model_enabled: bool,
    /// Authority that decides candidate validity.
    pub truth_authority: String,
}

/// Dated harness-profile catalog.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessProfileCatalog {
    /// Catalog schema version.
    pub schema_version: String,
    /// Verification timestamp.
    pub verified_at: UtcTimestamp,
    /// Pinned harness profiles.
    pub profiles: Vec<HarnessProfile>,
}

/// Load the pinned NYU harness profile without executing a model or container.
///
/// # Errors
///
/// Returns JSON, schema, immutable-commit, entrypoint, live-mode, or verifier
/// authority errors.
pub fn bundled_harness_profiles() -> Result<HarnessProfileCatalog, HarnessError> {
    let catalog: HarnessProfileCatalog =
        serde_json::from_str(include_str!("../../../config/harness-profiles.json"))?;
    if catalog.schema_version != "hsai.harness-profile-catalog.v1" {
        return Err(HarnessError::UnsafeProfile("catalog-schema".to_owned()));
    }
    for profile in &catalog.profiles {
        let commit_valid = profile.commit.len() == 40
            && profile
                .commit
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'));
        let required_entrypoints = ["d-cipher", "single-executor", "nyu-baseline"];
        if !commit_valid
            || profile.live_model_enabled
            || profile.default_mode != "RECORDED_TRACE"
            || profile.truth_authority != "PINNED_HSAI_CHALLENGE_VERIFIER"
            || !required_entrypoints
                .iter()
                .all(|entrypoint| profile.entrypoints.iter().any(|value| value == entrypoint))
        {
            return Err(HarnessError::UnsafeProfile(profile.profile_id.clone()));
        }
    }
    Ok(catalog)
}

/// Separate port for agent/evaluation harnesses.
// Each fallible method returns `HarnessError`; implementations document the
// supported authorization and fixture boundaries.
#[allow(clippy::missing_errors_doc)]
pub trait AgentHarnessAdapter {
    /// Describe the pinned harness implementation.
    fn describe_harness(&self) -> &AgentHarnessDescriptorV1;
    /// Prepare a run that exposes only the solver view.
    fn prepare_run(
        &self,
        challenge: &ChallengePackageV1,
        solver_view: &SolverViewManifestV1,
        configuration_digest: Sha256Digest,
    ) -> Result<AgentRunPlanV1, HarnessError>;
    /// Execute an offline or explicitly authorized run plan.
    fn execute(
        &self,
        plan: &AgentRunPlanV1,
        completed_at: UtcTimestamp,
    ) -> Result<AgentRunResultV1, HarnessError>;
    /// Extract candidate bytes. Harness reward is never consulted.
    fn extract_candidates(
        &self,
        result: &AgentRunResultV1,
    ) -> Result<Vec<HarnessCandidate>, HarnessError>;
    /// Export recorded trace commitments as evaluation evidence.
    fn export_trace(&self, result: &AgentRunResultV1) -> Result<AgentTraceBundleV1, HarnessError>;
}

/// Offline recorded-trace harness used for conformance and NYU-style integration.
#[derive(Debug, Clone)]
pub struct RecordedHarness {
    descriptor: AgentHarnessDescriptorV1,
    candidate: HarnessCandidate,
    recorded_reward_microunits: Option<i64>,
    trace_digest: Sha256Digest,
}

impl RecordedHarness {
    /// Construct a pinned offline harness with recorded candidate and telemetry.
    ///
    /// # Errors
    ///
    /// Returns an error if fixture metadata cannot be canonicalized.
    pub fn new(
        candidate_bytes: Vec<u8>,
        recorded_reward_microunits: Option<i64>,
    ) -> Result<Self, HarnessError> {
        let candidate_digest = plain_digest(&candidate_bytes);
        let container_digest =
            domain_separated_digest("hsai.harness-container.v1", &"recorded-offline-harness")?;
        let trace_digest = domain_separated_digest(
            "hsai.agent-trace.v1",
            &(candidate_digest, recorded_reward_microunits),
        )?;
        Ok(Self {
            descriptor: AgentHarnessDescriptorV1 {
                schema_version: "hsai.agent-harness-descriptor.v1".to_owned(),
                harness_id: "recorded-nyu-style-harness".to_owned(),
                harness_version: "1.0.0".to_owned(),
                source_commit: "recorded-fixture-commit".to_owned(),
                container_digest,
                supported_entrypoints: vec![
                    "d-cipher".to_owned(),
                    "single-executor".to_owned(),
                    "nyu-baseline".to_owned(),
                ],
                live_model_supported: false,
            },
            candidate: HarnessCandidate {
                source_ref: "recorded-run/candidate-0".to_owned(),
                bytes: candidate_bytes,
                digest: candidate_digest,
            },
            recorded_reward_microunits,
            trace_digest,
        })
    }
}

impl AgentHarnessAdapter for RecordedHarness {
    fn describe_harness(&self) -> &AgentHarnessDescriptorV1 {
        &self.descriptor
    }

    fn prepare_run(
        &self,
        challenge: &ChallengePackageV1,
        solver_view: &SolverViewManifestV1,
        configuration_digest: Sha256Digest,
    ) -> Result<AgentRunPlanV1, HarnessError> {
        if solver_view.challenge_package_digest != challenge.package_digest
            || solver_view.manifest_digest != challenge.solver_view_manifest_digest
        {
            return Err(HarnessError::SolverViewMismatch);
        }
        let plan_digest = domain_separated_digest(
            "hsai.agent-run-plan.v1",
            &(
                &self.descriptor.harness_id,
                challenge.package_digest,
                solver_view.manifest_digest,
                configuration_digest,
            ),
        )?;
        Ok(AgentRunPlanV1 {
            schema_version: "hsai.agent-run-plan.v1".to_owned(),
            plan_id: format!("agent_plan_{}", &plan_digest.to_hex()[..32]),
            harness_id: self.descriptor.harness_id.clone(),
            challenge_package_digest: challenge.package_digest,
            solver_view_manifest_digest: solver_view.manifest_digest,
            configuration_digest,
            prompt_digests: Vec::new(),
            model_provider_identity: "recorded-offline-fixture".to_owned(),
            tool_allowlist: Vec::new(),
            token_budget: 0,
            wall_clock_budget_millis: 1,
            live_authorized: false,
        })
    }

    fn execute(
        &self,
        plan: &AgentRunPlanV1,
        completed_at: UtcTimestamp,
    ) -> Result<AgentRunResultV1, HarnessError> {
        if plan.harness_id != self.descriptor.harness_id {
            return Err(HarnessError::HarnessMismatch);
        }
        if plan.live_authorized {
            return Err(HarnessError::LiveModelNotAuthorized);
        }
        let run_digest = domain_separated_digest(
            "hsai.agent-run.v1",
            &(&plan.plan_id, self.candidate.digest, self.trace_digest),
        )?;
        Ok(AgentRunResultV1 {
            schema_version: "hsai.agent-run-result.v1".to_owned(),
            run_id: format!("agent_run_{}", &run_digest.to_hex()[..32]),
            plan_id: plan.plan_id.clone(),
            candidate_digests: vec![self.candidate.digest],
            harness_reward_microunits: self.recorded_reward_microunits,
            token_usage: 0,
            tool_call_count: 0,
            runtime_millis: 1,
            completed_at,
        })
    }

    fn extract_candidates(
        &self,
        result: &AgentRunResultV1,
    ) -> Result<Vec<HarnessCandidate>, HarnessError> {
        if result.candidate_digests.contains(&self.candidate.digest) {
            Ok(vec![self.candidate.clone()])
        } else {
            Ok(Vec::new())
        }
    }

    fn export_trace(&self, result: &AgentRunResultV1) -> Result<AgentTraceBundleV1, HarnessError> {
        let bundle_digest = domain_separated_digest(
            "hsai.agent-trace-bundle.v1",
            &(&result.run_id, self.trace_digest),
        )?;
        Ok(AgentTraceBundleV1 {
            schema_version: "hsai.agent-trace-bundle.v1".to_owned(),
            run_id: result.run_id.clone(),
            trace_blob_digests: vec![self.trace_digest],
            public_summary_digest: self.trace_digest,
            bundle_digest,
        })
    }
}

fn plain_digest(bytes: &[u8]) -> Sha256Digest {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Sha256Digest::from_bytes(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use outcomes_corpus::{ChallengeCorpusAdapter, LocalFixtureCorpus};

    use super::*;

    fn timestamp(value: &str) -> Result<UtcTimestamp, outcomes_domain::TimestampError> {
        value.parse()
    }

    #[test]
    fn contract_recorded_reward_never_changes_candidate_bytes()
    -> Result<(), Box<dyn std::error::Error>> {
        let corpus = LocalFixtureCorpus::new(timestamp("2026-07-10T12:00:00Z")?)?;
        let snapshot = corpus.snapshot_corpus(corpus.describe_corpus().discovery_ref.as_str())?;
        let package = corpus.materialize_challenge(snapshot.snapshot_digest, "exact-bytes-v1")?;
        let solver_view = corpus.solver_view(package.package_digest)?;
        let config = domain_separated_digest("hsai.test-harness-config.v1", &"offline")?;
        let positive_reward = RecordedHarness::new(b"invalid".to_vec(), Some(1_000_000))?;
        let plan = positive_reward.prepare_run(package, solver_view, config)?;
        let result = positive_reward.execute(&plan, timestamp("2026-07-10T12:01:00Z")?)?;
        let candidates = positive_reward.extract_candidates(&result)?;
        assert_eq!(candidates[0].bytes, b"invalid");
        assert_eq!(result.harness_reward_microunits, Some(1_000_000));
        Ok(())
    }

    #[test]
    fn contract_live_mode_is_unreachable_for_recorded_harness()
    -> Result<(), Box<dyn std::error::Error>> {
        let harness = RecordedHarness::new(b"candidate".to_vec(), None)?;
        let result = AgentRunResultV1 {
            schema_version: "hsai.agent-run-result.v1".to_owned(),
            run_id: "run".to_owned(),
            plan_id: "plan".to_owned(),
            candidate_digests: Vec::new(),
            harness_reward_microunits: None,
            token_usage: 0,
            tool_call_count: 0,
            runtime_millis: 0,
            completed_at: timestamp("2026-07-10T12:01:00Z")?,
        };
        assert!(harness.extract_candidates(&result)?.is_empty());
        Ok(())
    }

    #[test]
    fn contract_nyu_harness_profile_is_pinned_offline_and_non_authoritative()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = bundled_harness_profiles()?;
        assert_eq!(catalog.profiles.len(), 1);
        let profile = &catalog.profiles[0];
        assert_eq!(profile.commit.len(), 40);
        assert!(!profile.live_model_enabled);
        assert_eq!(profile.truth_authority, "PINNED_HSAI_CHALLENGE_VERIFIER");
        Ok(())
    }
}
