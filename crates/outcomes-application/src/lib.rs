//! Cross-slice outcome use cases and the hermetic computational-market scenario.

use std::{collections::HashMap, fs, path::Path};

use hyperliquid_hip4::{Hip4Error, Hip4MockBackend, capability_matrix, parse_outcome_meta};

use outcomes_blobstore::{BlobStoreError, FileBlobStore};
use outcomes_corpus::{ChallengeCorpusAdapter, CorpusError, LocalFixtureCorpus};
use outcomes_domain::{
    ClaimSpecError, ClaimTruth, EvidenceClassification, ExecutionManifestV1, ExecutionOutcome,
    ReconciliationStatus, ResolutionAttestationV1, ResolutionFinality, SettlementNetwork,
    Sha256Digest, UtcTimestamp, domain_separated_digest, fixtures::local_demo_claim_draft,
};
use outcomes_evidence::{
    AttestorRegistry, DevelopmentSigner, EvidenceBundleInput, EvidenceError, RegisteredAttestor,
    build_evidence_bundle, verify_evidence_bundle,
};
use outcomes_resolution::{
    ResolutionError, ResolutionEvaluation, evaluate_existential_claim, finalize_resolution,
    propose_resolution,
};
use outcomes_runner::{
    RunnerError, RunnerRequest, ScriptedRunner, VerificationRunner, evaluate_admission,
};
use outcomes_settlement::{
    SettlementError, bind_market, build_intent, build_manual_handoff, preflight,
};
use outcomes_source::{LocalSubmissionLedger, SourceError, SubmissionSourceAdapter};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Hermetic demo/acceptance scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LocalScenario {
    /// Eligible candidate passes two agreeing replays and finalizes true.
    True,
    /// Complete empty observation finalizes false after deadline and grace.
    False,
    /// Runner error leaves truth unresolved and settlement-ineligible.
    RunnerError,
}

/// Machine-readable result of the hermetic scenario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioReport {
    /// Frozen derived claim identifier.
    pub claim_id: String,
    /// Normalized execution outcomes.
    pub execution_outcomes: Vec<ExecutionOutcome>,
    /// Current truth ceiling.
    pub truth: ClaimTruth,
    /// Current resolution finality.
    pub finality: ResolutionFinality,
    /// Exported evidence-bundle digest.
    pub evidence_bundle_digest: Sha256Digest,
    /// Whether the bundle verified without database access.
    pub evidence_verified: bool,
    /// Exact external-effect statement.
    pub external_effect: String,
    /// Number of distinct mock external effects.
    pub mock_effect_count: usize,
    /// External reconciliation state, when settlement was exercised.
    pub reconciliation_status: Option<ReconciliationStatus>,
    /// Relative exported bundle path under the selected artifact root.
    pub evidence_bundle_file: String,
}

/// Application composition failures.
#[derive(Debug, Error)]
pub enum ApplicationError {
    /// Frozen claim fixture was invalid.
    #[error(transparent)]
    Claim(#[from] ClaimSpecError),
    /// Timestamp fixture was invalid.
    #[error(transparent)]
    Timestamp(#[from] outcomes_domain::TimestampError),
    /// Corpus adapter failed.
    #[error(transparent)]
    Corpus(#[from] CorpusError),
    /// Submission source failed.
    #[error(transparent)]
    Source(#[from] SourceError),
    /// Admission or runner failed.
    #[error(transparent)]
    Runner(#[from] RunnerError),
    /// Evidence construction or verification failed.
    #[error(transparent)]
    Evidence(#[from] EvidenceError),
    /// Resolution or finality failed.
    #[error(transparent)]
    Resolution(#[from] ResolutionError),
    /// Content-addressed storage failed.
    #[error(transparent)]
    Blob(#[from] BlobStoreError),
    /// HIP-4 adapter failed.
    #[error(transparent)]
    Hip4(#[from] Hip4Error),
    /// Settlement preflight or intent construction failed.
    #[error(transparent)]
    Settlement(#[from] SettlementError),
    /// Artifact export failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// JSON encoding failed.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// Domain payload hashing failed.
    #[error(transparent)]
    Canonicalization(#[from] outcomes_domain::CanonicalizationError),
}

/// Run the local claim-to-final-resolution scenario without network access.
///
/// This function performs no market, chain, testnet, or mainnet mutation.
///
/// # Errors
///
/// Returns the typed adapter, runner, evidence, signature, resolution, storage,
/// timestamp, or serialization failure that stopped the scenario.
pub fn run_local_scenario(
    scenario: LocalScenario,
    artifact_root: &Path,
) -> Result<ScenarioReport, ApplicationError> {
    let claim = local_demo_claim_draft()?.freeze()?;
    run_scenario_with_runner(scenario, artifact_root, &claim, None)
}

/// Run the complete local-fixture TRUE workflow with a supplied verifier.
///
/// This composition point keeps Docker or another runner outside the domain,
/// while preserving the same evidence, quorum, finality, binding, and mock
/// settlement path used by the hermetic demo.
///
/// # Errors
///
/// Returns the typed adapter, runner, evidence, resolution, settlement,
/// storage, timestamp, serialization, or artifact-export failure.
pub fn run_local_true_scenario_with_runner(
    claim: &outcomes_domain::ClaimSpecV1,
    runner: &mut dyn VerificationRunner,
    artifact_root: &Path,
) -> Result<ScenarioReport, ApplicationError> {
    claim.verify_identity()?;
    run_scenario_with_runner(LocalScenario::True, artifact_root, claim, Some(runner))
}

#[allow(clippy::too_many_lines)]
fn run_scenario_with_runner(
    scenario: LocalScenario,
    artifact_root: &Path,
    claim: &outcomes_domain::ClaimSpecV1,
    mut supplied_runner: Option<&mut dyn VerificationRunner>,
) -> Result<ScenarioReport, ApplicationError> {
    let corpus = LocalFixtureCorpus::new(timestamp("2026-07-10T10:00:00Z")?)?;
    let snapshot = corpus.snapshot_corpus(corpus.describe_corpus().discovery_ref.as_str())?;
    let _package = corpus.materialize_challenge(snapshot.snapshot_digest, "exact-bytes-v1")?;
    let mut ledger =
        LocalSubmissionLedger::new(claim.claim_id.clone(), timestamp("2026-07-10T10:00:00Z")?);
    let mut candidates = Vec::new();
    let mut admissions = Vec::new();
    let mut executions = Vec::new();

    if scenario != LocalScenario::False {
        let bytes = if scenario == LocalScenario::True {
            b"hsai-pass-v1".as_slice()
        } else {
            b"hsai-error-v1".as_slice()
        };
        let candidate = ledger.ingest(
            claim.claim_id.clone(),
            "local.demo",
            "submission-1",
            "1",
            None,
            timestamp("2026-07-10T10:30:00Z")?,
            timestamp("2026-07-10T10:30:01Z")?,
            "application/octet-stream",
            bytes.to_vec(),
        )?;
        let admission = evaluate_admission(claim, &candidate)?;
        let replay_count = if scenario == LocalScenario::True {
            2
        } else {
            1
        };
        let mut scripted_runner = ScriptedRunner::exact_bytes_fixture();
        for index in 0..replay_count {
            let started = timestamp("2026-07-10T10:31:00Z")?;
            let ended = timestamp("2026-07-10T10:31:01Z")?;
            let request = RunnerRequest {
                claim,
                candidate: &candidate,
                candidate_bytes: bytes,
                admission: &admission,
                run_id: &format!("run-{index}"),
                started_at: &started,
                ended_at: &ended,
                logical_time: &started,
            };
            executions.push(if let Some(runner) = supplied_runner.as_deref_mut() {
                runner.execute(&request)?
            } else {
                scripted_runner.execute(&request)?
            });
        }
        candidates.push(candidate);
        admissions.push(admission);
    }

    let (cutoff, observed_at) = match scenario {
        LocalScenario::True | LocalScenario::RunnerError => (
            timestamp("2026-07-10T10:40:00Z")?,
            timestamp("2026-07-10T10:40:01Z")?,
        ),
        LocalScenario::False => {
            ledger.finalize_through(timestamp("2026-07-10T12:05:00Z")?);
            (
                timestamp("2026-07-10T12:05:00Z")?,
                timestamp("2026-07-10T12:05:01Z")?,
            )
        }
    };
    let observation = ledger.snapshot(None, &cutoff, &observed_at)?;
    let evaluation =
        evaluate_existential_claim(claim, &cutoff, observation.completeness, &executions, 0)?;

    let blob_store = FileBlobStore::new(artifact_root);
    let mut blob_bytes = HashMap::new();
    let mut blob_refs = Vec::new();
    for bytes in evidence_payloads(&observation, &candidates, &admissions, &executions)? {
        let reference =
            blob_store.put(&bytes, "application/json", EvidenceClassification::Public)?;
        blob_bytes.insert(reference.digest, bytes);
        blob_refs.push(reference);
    }
    let observation_digest = domain_separated_digest("hsai.observation-snapshot.v1", &observation)?;
    let completeness_digest = domain_separated_digest(
        "hsai.completeness-assessment.v1",
        &(observation.completeness, &observation.completeness_reason),
    )?;
    let aggregation_digest = domain_separated_digest(
        "hsai.resolution-aggregation.v1",
        &(
            evaluation.truth,
            &evaluation.qualifying_run_digests,
            format!("{:?}", evaluation.blockers),
        ),
    )?;
    let bundle = build_evidence_bundle(EvidenceBundleInput {
        claim_id: claim.claim_id.clone(),
        claim_spec_digest: claim.claim_spec_digest()?,
        challenge_descriptor_digest: claim.challenge_descriptor_digest,
        observation_snapshot_digests: vec![observation_digest],
        completeness_assessment_digest: completeness_digest,
        qualifying_candidate_digests: candidates
            .iter()
            .map(|candidate| candidate.artifact_digest)
            .collect(),
        admission_decision_digests: admissions
            .iter()
            .map(|admission| admission.decision_digest)
            .collect(),
        execution_manifest_digests: executions
            .iter()
            .map(|execution| domain_separated_digest("hsai.execution-manifest.v1", execution))
            .collect::<Result<Vec<_>, _>>()?,
        resolution_aggregation_digest: aggregation_digest,
        dispute_record_digests: Vec::new(),
        code_build_provenance_digest: domain_separated_digest(
            "hsai.build-provenance.v1",
            &env!("CARGO_PKG_VERSION"),
        )?,
        evidence_publication_policy_digest: domain_separated_digest(
            "hsai.evidence-publication-policy.v1",
            &claim.evidence_publication_policy,
        )?,
        blobs: blob_refs,
    })?;
    verify_evidence_bundle(&bundle, &blob_bytes)?;
    fs::create_dir_all(artifact_root)?;
    fs::write(
        artifact_root.join("evidence-bundle.json"),
        serde_json::to_vec_pretty(&bundle)?,
    )?;

    if evaluation.truth == ClaimTruth::Unresolved {
        return Ok(ScenarioReport {
            claim_id: claim.claim_id.to_string(),
            execution_outcomes: execution_outcomes(&executions),
            truth: ClaimTruth::Unresolved,
            finality: ResolutionFinality::None,
            evidence_bundle_digest: bundle.bundle_digest,
            evidence_verified: true,
            external_effect: "none; unresolved claims are not settlement-ready".to_owned(),
            mock_effect_count: 0,
            reconciliation_status: None,
            evidence_bundle_file: "evidence-bundle.json".to_owned(),
        });
    }

    let proposed_at = cutoff.clone();
    let dispute_end = proposed_at.checked_add_seconds(claim.dispute_policy.window_seconds)?;
    let policy_digest =
        domain_separated_digest("hsai.resolution-policy.v1", &claim.resolution_policy)?;
    let signer_a = DevelopmentSigner::generate("attestor-a", "key-a")?;
    let signer_b = DevelopmentSigner::generate("attestor-b", "key-b")?;
    let mut registry = AttestorRegistry::default();
    register_signers(
        &mut registry,
        [
            (&signer_a, "attestor-a", "key-a"),
            (&signer_b, "attestor-b", "key-b"),
        ],
    )?;
    let provisional_attestations = sign_pair(
        [&signer_a, &signer_b],
        claim,
        &evaluation,
        ResolutionFinality::Provisional,
        policy_digest,
        bundle.bundle_digest,
        completeness_digest,
        &proposed_at,
        &dispute_end,
        &proposed_at,
        1,
    )?;
    let provisional = propose_resolution(
        claim,
        &evaluation,
        bundle.bundle_digest,
        &provisional_attestations,
        &registry,
        &proposed_at,
    )?;
    let final_attestations = sign_pair(
        [&signer_a, &signer_b],
        claim,
        &evaluation,
        ResolutionFinality::Final,
        policy_digest,
        bundle.bundle_digest,
        completeness_digest,
        &proposed_at,
        &dispute_end,
        &dispute_end,
        2,
    )?;
    let final_record = finalize_resolution(
        claim,
        &provisional,
        &dispute_end,
        false,
        false,
        &final_attestations,
        &registry,
    )?;
    if scenario == LocalScenario::True {
        let market = parse_outcome_meta(
            SettlementNetwork::Mock,
            include_bytes!("../../../fixtures/hip4/outcome-meta.json"),
        )?;
        let capabilities = capability_matrix(SettlementNetwork::Mock, proposed_at.clone());
        let binding = bind_market(
            claim,
            &market,
            0,
            1,
            domain_separated_digest("hsai.backend-capability.v1", &capabilities)?,
            "local-demo-operator",
            proposed_at.clone(),
            "development-binding-signature".to_owned(),
        )?;
        let readiness = preflight(
            &binding,
            &final_record,
            market.metadata_digest,
            &capabilities,
        );
        let intent = build_intent(
            &binding,
            &final_record,
            &readiness,
            dispute_end.checked_add_seconds(3_600)?,
        )?;
        let handoff = build_manual_handoff(
            &binding,
            &final_record,
            &intent,
            "cas://local/evidence-bundle.json".to_owned(),
            bundle.bundle_digest,
            "documented-external-settlement-authority".to_owned(),
            "development-handoff-signature".to_owned(),
        )?;
        let mut backend = Hip4MockBackend::default();
        let receipt = backend.submit(&intent, dispute_end.clone())?;
        let idempotent_retry = backend.submit(&intent, dispute_end.clone())?;
        if receipt != idempotent_retry || backend.effect_count() != 1 {
            return Err(ApplicationError::Settlement(
                SettlementError::DigestMismatch,
            ));
        }
        let reconciliation =
            backend.reconcile(&binding, &final_record, dispute_end.checked_add_seconds(1)?)?;
        for (name, value) in [
            ("market-binding.json", serde_json::to_value(&binding)?),
            (
                "final-resolution.json",
                serde_json::to_value(&final_record)?,
            ),
            (
                "hip4-capabilities.json",
                serde_json::to_value(&capabilities)?,
            ),
            (
                "settlement-preflight.json",
                serde_json::to_value(&readiness)?,
            ),
            ("settlement-intent.json", serde_json::to_value(&intent)?),
            ("settlement-handoff.json", serde_json::to_value(&handoff)?),
            ("mock-receipt.json", serde_json::to_value(&receipt)?),
            (
                "mock-reconciliation.json",
                serde_json::to_value(&reconciliation)?,
            ),
        ] {
            fs::write(artifact_root.join(name), serde_json::to_vec_pretty(&value)?)?;
        }
        return Ok(ScenarioReport {
            claim_id: claim.claim_id.to_string(),
            execution_outcomes: execution_outcomes(&executions),
            truth: final_record.truth,
            finality: final_record.finality,
            evidence_bundle_digest: bundle.bundle_digest,
            evidence_verified: true,
            external_effect: "one idempotent mock HIP-4 effect; reconciled CONFIRMED; no testnet or mainnet write".to_owned(),
            mock_effect_count: backend.effect_count(),
            reconciliation_status: Some(reconciliation.status),
            evidence_bundle_file: "evidence-bundle.json".to_owned(),
        });
    }

    Ok(ScenarioReport {
        claim_id: claim.claim_id.to_string(),
        execution_outcomes: execution_outcomes(&executions),
        truth: final_record.truth,
        finality: final_record.finality,
        evidence_bundle_digest: bundle.bundle_digest,
        evidence_verified: true,
        external_effect: format!(
            "none; local {:?} resolution and evidence only",
            final_record.truth
        ),
        mock_effect_count: 0,
        reconciliation_status: None,
        evidence_bundle_file: "evidence-bundle.json".to_owned(),
    })
}

fn evidence_payloads(
    observation: &outcomes_domain::ObservationSnapshotV1,
    candidates: &[outcomes_domain::CandidateEnvelopeV1],
    admissions: &[outcomes_domain::AdmissionDecisionV1],
    executions: &[ExecutionManifestV1],
) -> Result<Vec<Vec<u8>>, serde_json::Error> {
    let mut payloads = vec![serde_json::to_vec(observation)?];
    payloads.extend(
        candidates
            .iter()
            .map(serde_json::to_vec)
            .collect::<Result<Vec<_>, _>>()?,
    );
    payloads.extend(
        admissions
            .iter()
            .map(serde_json::to_vec)
            .collect::<Result<Vec<_>, _>>()?,
    );
    payloads.extend(
        executions
            .iter()
            .map(serde_json::to_vec)
            .collect::<Result<Vec<_>, _>>()?,
    );
    Ok(payloads)
}

fn execution_outcomes(executions: &[ExecutionManifestV1]) -> Vec<ExecutionOutcome> {
    executions
        .iter()
        .map(|execution| execution.normalized_outcome)
        .collect()
}

fn register_signers(
    registry: &mut AttestorRegistry,
    signers: [(&DevelopmentSigner, &str, &str); 2],
) -> Result<(), outcomes_domain::TimestampError> {
    for (signer, signer_id, key_id) in signers {
        registry.register(RegisteredAttestor {
            signer_id: signer_id.to_owned(),
            key_id: key_id.to_owned(),
            verifying_key: signer.verifying_key(),
            active_from: timestamp("2026-07-10T00:00:00Z")?,
            revoked_at: None,
        });
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn sign_pair(
    signers: [&DevelopmentSigner; 2],
    claim: &outcomes_domain::ClaimSpecV1,
    evaluation: &ResolutionEvaluation,
    finality: ResolutionFinality,
    policy_digest: Sha256Digest,
    evidence_digest: Sha256Digest,
    completeness_digest: Sha256Digest,
    start: &UtcTimestamp,
    end: &UtcTimestamp,
    issued_at: &UtcTimestamp,
    sequence: u64,
) -> Result<Vec<ResolutionAttestationV1>, ApplicationError> {
    signers
        .into_iter()
        .map(|signer| {
            signer
                .sign_attestation(
                    claim.claim_id.clone(),
                    claim.claim_spec_digest()?,
                    evaluation.truth,
                    finality,
                    policy_digest,
                    evidence_digest,
                    evaluation.qualifying_run_digests.clone(),
                    (evaluation.truth == ClaimTruth::False).then_some(completeness_digest),
                    start.clone(),
                    end.clone(),
                    issued_at.clone(),
                    sequence,
                )
                .map_err(ApplicationError::from)
        })
        .collect()
}

fn timestamp(value: &str) -> Result<UtcTimestamp, outcomes_domain::TimestampError> {
    value.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use outcomes_runner::{
        CONTAINER_CANDIDATE_PATH, CONTAINER_VERIFIER_PATH, DockerVerificationRunner,
        OciSandboxPolicy,
    };

    #[test]
    fn contract_true_false_and_error_scenarios_preserve_boundaries() -> Result<(), ApplicationError>
    {
        for (scenario, truth, finality) in [
            (
                LocalScenario::True,
                ClaimTruth::True,
                ResolutionFinality::Final,
            ),
            (
                LocalScenario::False,
                ClaimTruth::False,
                ResolutionFinality::Final,
            ),
            (
                LocalScenario::RunnerError,
                ClaimTruth::Unresolved,
                ResolutionFinality::None,
            ),
        ] {
            let directory = tempfile::tempdir().map_err(BlobStoreError::from)?;
            let report = run_local_scenario(scenario, directory.path())?;
            assert_eq!(report.truth, truth);
            assert_eq!(report.finality, finality);
            assert!(report.evidence_verified);
            if scenario == LocalScenario::True {
                assert_eq!(report.mock_effect_count, 1);
                assert_eq!(
                    report.reconciliation_status,
                    Some(ReconciliationStatus::Confirmed)
                );
            } else {
                assert!(report.external_effect.starts_with("none;"));
                assert_eq!(report.mock_effect_count, 0);
            }
        }
        Ok(())
    }

    #[test]
    #[ignore = "requires an explicitly selected preloaded local Alpine image"]
    fn external_docker_true_path_reaches_final_mock_settlement()
    -> Result<(), Box<dyn std::error::Error>> {
        let (Ok(image_repository), Ok(image_digest)) = (
            std::env::var("HSAI_TEST_OCI_IMAGE_REPOSITORY"),
            std::env::var("HSAI_TEST_OCI_IMAGE_DIGEST"),
        ) else {
            return Ok(());
        };
        let expected = b"hsai-pass-v1";
        let argv = vec![
            "/usr/bin/cmp".to_owned(),
            CONTAINER_CANDIDATE_PATH.to_owned(),
            CONTAINER_VERIFIER_PATH.to_owned(),
        ];
        let mut draft = local_demo_claim_draft()?;
        draft.verification_policy.validator_artifact_digest =
            "1d7e49a5b5f6f72ea8a2b87812cea7d69ce0b7a0034094c7dacfebda4ed1c6dd".parse()?;
        draft.verification_policy.argv.clone_from(&argv);
        draft.verification_policy.resource_limits.wall_clock_millis = 5_000;
        let claim = draft.freeze()?;
        let policy = OciSandboxPolicy {
            image_repository,
            image_digest,
            user_id: 65_534,
            read_only_root: true,
            drop_all_capabilities: true,
            network_disabled: true,
            host_socket_mounted: false,
            argv,
            environment_allowlist: vec!["LANG".to_owned(), "TZ".to_owned()],
            pids_limit: 16,
            memory_bytes: 64 * 1024 * 1024,
            cpu_millis: 250,
            scratch_bytes: 4 * 1024 * 1024,
            timeout_millis: 5_000,
        };
        let mut runner = DockerVerificationRunner::new(policy, expected.to_vec())?;
        let directory = tempfile::tempdir()?;

        let report = run_local_true_scenario_with_runner(&claim, &mut runner, directory.path())?;

        assert_eq!(
            report.execution_outcomes,
            vec![ExecutionOutcome::Pass, ExecutionOutcome::Pass]
        );
        assert_eq!(report.truth, ClaimTruth::True);
        assert_eq!(report.finality, ResolutionFinality::Final);
        assert!(report.evidence_verified);
        assert_eq!(report.mock_effect_count, 1);
        assert_eq!(
            report.reconciliation_status,
            Some(ReconciliationStatus::Confirmed)
        );
        assert!(directory.path().join("evidence-bundle.json").is_file());
        Ok(())
    }
}
