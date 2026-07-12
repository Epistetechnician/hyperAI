//! Cross-slice outcome use cases and the hermetic computational-market scenario.

use std::{collections::HashMap, fs, path::Path};

use hyperliquid_hip4::{Hip4Error, Hip4MockBackend, capability_matrix, parse_outcome_meta};

use outcomes_blobstore::{BlobStoreError, FileBlobStore};
use outcomes_corpus::{ChallengeCorpusAdapter, CorpusError, LocalFixtureCorpus};
use outcomes_domain::{
    ClaimSpecError, ClaimTruth, DisputeDecisionKind, EvidenceClassification, ExecutionManifestV1,
    ExecutionOutcome, MarketBindingV1, MutationContext, MutationScope, OutcomeEvidenceBundleV1,
    ReconciliationResultV1, ReconciliationStatus, ResolutionAttestationV1, ResolutionFinality,
    ResolutionRecordV1, SettlementNetwork, Sha256Digest, StateSlice, SubmissionReceiptV1,
    UnsignedSettlementIntentV1, UtcTimestamp, domain_separated_digest,
    fixtures::local_demo_claim_draft, submission_receipt_digest,
};
use outcomes_evidence::{
    AttestorRegistry, DevelopmentSigner, EvidenceBundleInput, EvidenceError, RegisteredAttestor,
    build_evidence_bundle, verify_evidence_bundle,
};
use outcomes_persistence_sqlx::{
    AppendOutcome, PersistenceError, SettlementRecoveryAction, SqliteStore,
};
use outcomes_resolution::{
    ResolutionBlocker, ResolutionError, ResolutionEvaluation, decide_dispute,
    evaluate_existential_claim, finalize_resolution, finalize_superseding_resolution,
    link_resolution_supersession, open_dispute, propose_resolution, propose_superseding_resolution,
    resolution_aggregation_digest,
};
use outcomes_runner::{
    RunnerError, RunnerRequest, ScriptedRunner, VerificationRunner, evaluate_admission,
};
use outcomes_settlement::{
    SettlementEligibilityAssessment, SettlementEligibilityBlocker, SettlementError,
    assess_settlement_eligibility, bind_market, build_intent, build_manual_handoff, preflight,
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
    /// A deterministic rejection remains unresolved before a later valid candidate finalizes true.
    InvalidThenValid,
    /// A counterexample supersedes a disputed provisional false before finality.
    DisputedFalseCounterexampleTrue,
    /// Complete empty observation finalizes false after deadline and grace.
    False,
    /// A pagination gap after deadline keeps the claim unresolved.
    IncompleteObservation,
    /// Conflicting replay outcomes keep the claim unresolved.
    DivergentReplays,
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
    /// Read-only refusal before backend preflight when no final resolution exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settlement_eligibility: Option<SettlementEligibilityAssessment>,
}

/// Retained proof of the bounded local lost-receipt restart contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalRecoveryReportV1 {
    /// Fixed report schema.
    pub schema_version: String,
    /// Relative retained `SQLite` artifact.
    pub database_file: String,
    /// Durable settlement intent identity.
    pub intent_id: String,
    /// Request digest persisted before the only submission.
    pub request_digest: Sha256Digest,
    /// Canonical recovered receipt identity.
    pub receipt_digest: Sha256Digest,
    /// Canonical terminal reconciliation identity.
    pub reconciliation_digest: Sha256Digest,
    /// Restart action immediately after the caller loses the receipt.
    pub first_restart_action: String,
    /// Restart action after receipt recovery and reconciliation are durable.
    pub final_restart_action: String,
    /// Exact durable intent cardinality.
    pub intent_count: u64,
    /// Exact durable receipt cardinality.
    pub receipt_count: u64,
    /// Exact durable reconciliation cardinality.
    pub reconciliation_count: u64,
    /// Calls crossing the mock submission boundary.
    pub submission_call_count: usize,
    /// Distinct accepted mock effects.
    pub mock_effect_count: usize,
    /// Precise evidence ceiling for this probe.
    pub claim_ceiling: String,
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
    /// Durable local persistence failed closed.
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
    /// A composed acceptance invariant was violated.
    #[error("scenario invariant violated: {0}")]
    ScenarioInvariant(&'static str),
    /// A retained recovery artifact or transition violated the bounded contract.
    #[error("local recovery invariant violated: {0}")]
    RecoveryInvariant(String),
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
    if scenario == LocalScenario::DisputedFalseCounterexampleTrue {
        return run_disputed_false_counterexample_true(&claim, artifact_root);
    }
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
fn run_disputed_false_counterexample_true(
    claim: &outcomes_domain::ClaimSpecV1,
    artifact_root: &Path,
) -> Result<ScenarioReport, ApplicationError> {
    let cutoff = timestamp("2026-07-10T12:05:00Z")?;
    let proposed_at = timestamp("2026-07-10T12:06:00Z")?;
    let dispute_end = proposed_at.checked_add_seconds(claim.dispute_policy.window_seconds)?;

    // The first adapter view is intentionally faulty: it attests COMPLETE while
    // omitting a candidate already present in the independent authoritative view.
    // No timestamp is changed when the counterexample is later presented.
    let mut faulty_ledger =
        LocalSubmissionLedger::new(claim.claim_id.clone(), timestamp("2026-07-10T10:00:00Z")?);
    faulty_ledger.finalize_through(cutoff.clone());
    let original_observation =
        faulty_ledger.snapshot(None, &cutoff, &timestamp("2026-07-10T12:05:01Z")?)?;
    let original_evaluation =
        evaluate_existential_claim(claim, &cutoff, original_observation.completeness, &[], 0)?;
    if original_evaluation.truth != ClaimTruth::False {
        return Err(ApplicationError::ScenarioInvariant(
            "faulty complete empty observation must propose false",
        ));
    }
    let (original_bundle, original_blobs) = build_application_bundle(
        claim,
        std::slice::from_ref(&original_observation),
        &[],
        &[],
        &[],
        &[],
        &original_evaluation,
        Vec::new(),
        artifact_root,
    )?;
    verify_evidence_bundle(&original_bundle, &original_blobs)?;

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
    let policy_digest =
        domain_separated_digest("hsai.resolution-policy.v1", &claim.resolution_policy)?;
    let original_attestations = sign_pair(
        [&signer_a, &signer_b],
        claim,
        &original_evaluation,
        ResolutionFinality::Provisional,
        policy_digest,
        original_bundle.bundle_digest,
        original_bundle.completeness_assessment_digest,
        &proposed_at,
        &dispute_end,
        &proposed_at,
        1,
    )?;
    let original_provisional = propose_resolution(
        claim,
        &original_evaluation,
        original_bundle.bundle_digest,
        &original_attestations,
        &registry,
        &proposed_at,
    )?;

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
    let refused = preflight(
        &binding,
        &original_provisional,
        market.metadata_digest,
        &capabilities,
    );
    if refused.ready
        || !refused
            .blockers
            .iter()
            .any(|value| value == "FINAL_RESOLUTION_REQUIRED")
    {
        return Err(ApplicationError::ScenarioInvariant(
            "provisional false must not be settlement-ready",
        ));
    }

    let mut authoritative_ledger =
        LocalSubmissionLedger::new(claim.claim_id.clone(), timestamp("2026-07-10T10:00:00Z")?);
    let candidate = authoritative_ledger.ingest(
        claim.claim_id.clone(),
        "local.authoritative",
        "counterexample-1",
        "1",
        None,
        timestamp("2026-07-10T11:55:00Z")?,
        timestamp("2026-07-10T11:56:00Z")?,
        "application/octet-stream",
        b"hsai-pass-v1".to_vec(),
    )?;
    authoritative_ledger.finalize_through(cutoff.clone());
    let dispute = open_dispute(
        claim,
        &original_provisional,
        "local-demo-operator",
        "COUNTEREXAMPLE",
        vec![candidate.artifact_digest],
        Some(candidate.artifact_digest),
        timestamp("2026-07-10T12:08:00Z")?,
    )?;
    if !matches!(
        finalize_resolution(
            claim,
            &original_provisional,
            &dispute_end,
            true,
            false,
            &original_attestations,
            &registry,
        ),
        Err(ResolutionError::BlockingDispute)
    ) {
        return Err(ApplicationError::ScenarioInvariant(
            "open dispute must block original finalization",
        ));
    }

    let admission = evaluate_admission(claim, &candidate)?;
    let mut runner = ScriptedRunner::exact_bytes_fixture();
    let mut executions = Vec::new();
    for index in 0..2 {
        let started = timestamp("2026-07-10T12:09:00Z")?;
        let ended = timestamp("2026-07-10T12:09:01Z")?;
        executions.push(runner.execute(&RunnerRequest {
            claim,
            candidate: &candidate,
            candidate_bytes: b"hsai-pass-v1",
            admission: &admission,
            run_id: &format!("counterexample-run-{index}"),
            started_at: &started,
            ended_at: &ended,
            logical_time: &started,
        })?);
    }
    let corrected_observation =
        authoritative_ledger.snapshot(None, &cutoff, &timestamp("2026-07-10T12:09:02Z")?)?;
    let replacement_evaluation = evaluate_existential_claim(
        claim,
        &cutoff,
        corrected_observation.completeness,
        &executions,
        0,
    )?;
    let lineage_payloads = vec![
        serde_json::to_vec(&original_bundle)?,
        serde_json::to_vec(&original_provisional)?,
        serde_json::to_vec(&dispute)?,
    ];
    let (replacement_bundle, replacement_blobs) = build_application_bundle(
        claim,
        &[original_observation, corrected_observation],
        std::slice::from_ref(&candidate),
        std::slice::from_ref(&admission),
        &executions,
        &[dispute.record_digest],
        &replacement_evaluation,
        lineage_payloads,
        artifact_root,
    )?;
    verify_evidence_bundle(&replacement_bundle, &replacement_blobs)?;
    let decision = decide_dispute(
        claim,
        &original_provisional,
        &dispute,
        "local-demo-operator",
        DisputeDecisionKind::AcceptedSupersededBeforeFinality,
        "pinned pre-cutoff counterexample passed both required replays".to_owned(),
        vec![candidate.artifact_digest],
        Some(replacement_bundle.bundle_digest),
        Some(resolution_aggregation_digest(&replacement_evaluation)?),
        Some(&replacement_evaluation),
        timestamp("2026-07-10T12:10:00Z")?,
    )?;
    // The terminal decision binds bundle B, so including the decision in B
    // would create a digest cycle. It is exported alongside the acyclic bundle.
    let replacement_attestations = sign_superseding_pair(
        [&signer_a, &signer_b],
        claim,
        &replacement_evaluation,
        ResolutionFinality::Provisional,
        policy_digest,
        replacement_bundle.bundle_digest,
        replacement_bundle.completeness_assessment_digest,
        &proposed_at,
        &dispute_end,
        &timestamp("2026-07-10T12:10:01Z")?,
        2,
        original_provisional.record_digest,
    )?;
    let replacement_provisional = propose_superseding_resolution(
        claim,
        &original_provisional,
        &decision,
        &replacement_evaluation,
        replacement_bundle.bundle_digest,
        &replacement_attestations,
        &registry,
    )?;
    let final_attestations = sign_superseding_pair(
        [&signer_a, &signer_b],
        claim,
        &replacement_evaluation,
        ResolutionFinality::Final,
        policy_digest,
        replacement_bundle.bundle_digest,
        replacement_bundle.completeness_assessment_digest,
        &proposed_at,
        &dispute_end,
        &dispute_end,
        3,
        original_provisional.record_digest,
    )?;
    let final_record = finalize_superseding_resolution(
        claim,
        &original_provisional,
        &replacement_provisional,
        &dispute_end,
        false,
        &final_attestations,
        &registry,
    )?;
    let supersession = link_resolution_supersession(
        &original_provisional,
        &decision,
        &final_record,
        dispute_end.clone(),
    )?;
    let ready = preflight(
        &binding,
        &final_record,
        market.metadata_digest,
        &capabilities,
    );
    let intent = build_intent(
        &binding,
        &final_record,
        &ready,
        dispute_end.checked_add_seconds(3_600)?,
    )?;
    let mut backend = Hip4MockBackend::default();
    let receipt = backend.submit(&intent, dispute_end.clone())?;
    let retry = backend.submit(&intent, dispute_end.clone())?;
    if receipt != retry || backend.effect_count() != 1 {
        return Err(ApplicationError::ScenarioInvariant(
            "mock settlement must effect exactly once",
        ));
    }
    let reconciliation =
        backend.reconcile(&binding, &final_record, dispute_end.checked_add_seconds(1)?)?;
    fs::create_dir_all(artifact_root)?;
    for (name, value) in [
        (
            "original-evidence-bundle.json",
            serde_json::to_value(&original_bundle)?,
        ),
        (
            "original-provisional-false.json",
            serde_json::to_value(&original_provisional)?,
        ),
        (
            "original-settlement-preflight-refused.json",
            serde_json::to_value(&refused)?,
        ),
        (
            "counterexample-dispute.json",
            serde_json::to_value(&dispute)?,
        ),
        (
            "replacement-evidence-bundle.json",
            serde_json::to_value(&replacement_bundle)?,
        ),
        (
            "counterexample-decision.json",
            serde_json::to_value(&decision)?,
        ),
        (
            "replacement-provisional-true.json",
            serde_json::to_value(&replacement_provisional)?,
        ),
        (
            "final-resolution.json",
            serde_json::to_value(&final_record)?,
        ),
        (
            "resolution-supersession.json",
            serde_json::to_value(&supersession)?,
        ),
        ("settlement-intent.json", serde_json::to_value(&intent)?),
        ("mock-receipt.json", serde_json::to_value(&receipt)?),
        (
            "mock-reconciliation.json",
            serde_json::to_value(&reconciliation)?,
        ),
    ] {
        fs::write(artifact_root.join(name), serde_json::to_vec_pretty(&value)?)?;
    }
    Ok(ScenarioReport {
        claim_id: claim.claim_id.to_string(),
        execution_outcomes: execution_outcomes(&executions),
        truth: final_record.truth,
        finality: final_record.finality,
        evidence_bundle_digest: replacement_bundle.bundle_digest,
        evidence_verified: true,
        external_effect: "one idempotent mock HIP-4 effect after accepted pre-finality supersession; reconciled CONFIRMED".to_owned(),
        mock_effect_count: backend.effect_count(),
        reconciliation_status: Some(reconciliation.status),
        evidence_bundle_file: "replacement-evidence-bundle.json".to_owned(),
        settlement_eligibility: None,
    })
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

    if !matches!(
        scenario,
        LocalScenario::False | LocalScenario::IncompleteObservation
    ) {
        let candidate_inputs: Vec<(&str, &[u8], usize)> = match scenario {
            LocalScenario::True | LocalScenario::DivergentReplays => {
                vec![("submission-1", b"hsai-pass-v1", 2)]
            }
            LocalScenario::InvalidThenValid => vec![
                ("submission-1", b"hsai-invalid-v1", 1),
                ("submission-2", b"hsai-pass-v1", 2),
            ],
            LocalScenario::RunnerError => vec![("submission-1", b"hsai-error-v1", 1)],
            LocalScenario::DisputedFalseCounterexampleTrue => {
                unreachable!("dispute scenario has a dedicated composition")
            }
            LocalScenario::False | LocalScenario::IncompleteObservation => {
                unreachable!("empty-observation scenarios have no candidates")
            }
        };
        let mut scripted_runner = if scenario == LocalScenario::DivergentReplays {
            ScriptedRunner::new(vec![ExecutionOutcome::Pass, ExecutionOutcome::Fail])
        } else {
            ScriptedRunner::exact_bytes_fixture()
        };
        for (candidate_index, (source_id, bytes, replay_count)) in
            candidate_inputs.into_iter().enumerate()
        {
            let candidate = ledger.ingest(
                claim.claim_id.clone(),
                "local.demo",
                source_id,
                "1",
                None,
                timestamp(if candidate_index == 0 {
                    "2026-07-10T10:30:00Z"
                } else {
                    "2026-07-10T10:35:00Z"
                })?,
                timestamp(if candidate_index == 0 {
                    "2026-07-10T10:30:01Z"
                } else {
                    "2026-07-10T10:35:01Z"
                })?,
                "application/octet-stream",
                bytes.to_vec(),
            )?;
            let admission = evaluate_admission(claim, &candidate)?;
            for index in 0..replay_count {
                let started = timestamp(if candidate_index == 0 {
                    "2026-07-10T10:31:00Z"
                } else {
                    "2026-07-10T10:36:00Z"
                })?;
                let ended = timestamp(if candidate_index == 0 {
                    "2026-07-10T10:31:01Z"
                } else {
                    "2026-07-10T10:36:01Z"
                })?;
                let request = RunnerRequest {
                    claim,
                    candidate: &candidate,
                    candidate_bytes: bytes,
                    admission: &admission,
                    run_id: &format!("run-{candidate_index}-{index}"),
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

            if scenario == LocalScenario::InvalidThenValid && candidate_index == 0 {
                let intermediate_cutoff = timestamp("2026-07-10T10:34:00Z")?;
                let intermediate_observation = ledger.snapshot(
                    None,
                    &intermediate_cutoff,
                    &timestamp("2026-07-10T10:34:01Z")?,
                )?;
                let intermediate = evaluate_existential_claim(
                    claim,
                    &intermediate_cutoff,
                    intermediate_observation.completeness,
                    &executions,
                    0,
                )?;
                if intermediate.truth != ClaimTruth::Unresolved {
                    return Err(ApplicationError::ScenarioInvariant(
                        "an invalid first candidate must remain unresolved",
                    ));
                }
            }
        }
    }

    let (cutoff, observed_at) = match scenario {
        LocalScenario::True | LocalScenario::InvalidThenValid | LocalScenario::RunnerError => (
            timestamp("2026-07-10T10:40:00Z")?,
            timestamp("2026-07-10T10:40:01Z")?,
        ),
        LocalScenario::False | LocalScenario::IncompleteObservation => {
            ledger.finalize_through(timestamp("2026-07-10T12:05:00Z")?);
            if scenario == LocalScenario::IncompleteObservation {
                ledger.set_pagination_complete(false);
            }
            (
                timestamp("2026-07-10T12:05:00Z")?,
                timestamp("2026-07-10T12:05:01Z")?,
            )
        }
        LocalScenario::DivergentReplays => {
            ledger.finalize_through(timestamp("2026-07-10T12:05:00Z")?);
            (
                timestamp("2026-07-10T12:05:00Z")?,
                timestamp("2026-07-10T12:05:01Z")?,
            )
        }
        LocalScenario::DisputedFalseCounterexampleTrue => {
            unreachable!("dispute scenario has a dedicated composition")
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
    let execution_manifest_digests = executions
        .iter()
        .map(|execution| domain_separated_digest("hsai.execution-manifest.v1", execution))
        .collect::<Result<Vec<_>, _>>()?;
    let qualifying_candidate_digests = candidates
        .iter()
        .filter(|candidate| {
            executions.iter().zip(&execution_manifest_digests).any(
                |(execution, execution_digest)| {
                    execution.candidate_digest == candidate.artifact_digest
                        && evaluation.qualifying_run_digests.contains(execution_digest)
                },
            )
        })
        .map(|candidate| candidate.artifact_digest)
        .collect();
    let bundle = build_evidence_bundle(EvidenceBundleInput {
        claim_id: claim.claim_id.clone(),
        claim_spec_digest: claim.claim_spec_digest()?,
        challenge_descriptor_digest: claim.challenge_descriptor_digest,
        observation_snapshot_digests: vec![observation_digest],
        completeness_assessment_digest: completeness_digest,
        qualifying_candidate_digests,
        admission_decision_digests: admissions
            .iter()
            .map(|admission| admission.decision_digest)
            .collect(),
        execution_manifest_digests,
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
        let mut settlement_eligibility = None;
        if matches!(
            scenario,
            LocalScenario::IncompleteObservation | LocalScenario::DivergentReplays
        ) {
            let expected_blocker = if scenario == LocalScenario::IncompleteObservation {
                ResolutionBlocker::ObservationIncomplete
            } else {
                ResolutionBlocker::ReplayDivergence
            };
            if evaluation.blockers != vec![expected_blocker] {
                return Err(ApplicationError::ScenarioInvariant(
                    "acceptance refusal must retain its exact resolution blocker",
                ));
            }
            if !matches!(
                propose_resolution(
                    claim,
                    &evaluation,
                    bundle.bundle_digest,
                    &[],
                    &AttestorRegistry::default(),
                    &cutoff,
                ),
                Err(ResolutionError::UnresolvedProposal)
            ) {
                return Err(ApplicationError::ScenarioInvariant(
                    "unresolved evidence must not produce a resolution record",
                ));
            }
            let eligibility =
                assess_settlement_eligibility(&claim.claim_id, bundle.bundle_digest, None);
            if eligibility.eligible_for_preflight
                || eligibility.blockers
                    != vec![
                        SettlementEligibilityBlocker::ResolutionRequired,
                        SettlementEligibilityBlocker::FinalResolutionRequired,
                    ]
            {
                return Err(ApplicationError::ScenarioInvariant(
                    "unresolved evidence must fail settlement eligibility",
                ));
            }
            fs::write(
                artifact_root.join("observation-snapshot.json"),
                serde_json::to_vec_pretty(&observation)?,
            )?;
            fs::write(
                artifact_root.join("resolution-evaluation.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "truth": evaluation.truth,
                    "qualifying_run_digests": evaluation.qualifying_run_digests,
                    "blockers": evaluation
                        .blockers
                        .iter()
                        .map(|blocker| format!("{blocker:?}"))
                        .collect::<Vec<_>>(),
                }))?,
            )?;
            fs::write(
                artifact_root.join("settlement-eligibility.json"),
                serde_json::to_vec_pretty(&eligibility)?,
            )?;
            settlement_eligibility = Some(eligibility);
        }
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
            settlement_eligibility,
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
    if matches!(
        scenario,
        LocalScenario::True | LocalScenario::InvalidThenValid
    ) {
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
            settlement_eligibility: None,
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
        settlement_eligibility: None,
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

#[allow(clippy::too_many_arguments)]
fn build_application_bundle(
    claim: &outcomes_domain::ClaimSpecV1,
    observations: &[outcomes_domain::ObservationSnapshotV1],
    candidates: &[outcomes_domain::CandidateEnvelopeV1],
    admissions: &[outcomes_domain::AdmissionDecisionV1],
    executions: &[ExecutionManifestV1],
    dispute_record_digests: &[Sha256Digest],
    evaluation: &ResolutionEvaluation,
    extra_payloads: Vec<Vec<u8>>,
    artifact_root: &Path,
) -> Result<
    (
        outcomes_domain::OutcomeEvidenceBundleV1,
        HashMap<Sha256Digest, Vec<u8>>,
    ),
    ApplicationError,
> {
    let store = FileBlobStore::new(artifact_root);
    let mut bytes_by_digest = HashMap::new();
    let mut references = Vec::new();
    let mut payloads = Vec::new();
    for observation in observations {
        payloads.push(serde_json::to_vec(observation)?);
    }
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
    payloads.extend(extra_payloads);
    for bytes in payloads {
        let reference = store.put(&bytes, "application/json", EvidenceClassification::Public)?;
        bytes_by_digest.insert(reference.digest, bytes);
        references.push(reference);
    }
    let execution_digests = executions
        .iter()
        .map(|value| domain_separated_digest("hsai.execution-manifest.v1", value))
        .collect::<Result<Vec<_>, _>>()?;
    let qualifying_candidates = candidates
        .iter()
        .filter(|candidate| {
            executions
                .iter()
                .zip(&execution_digests)
                .any(|(run, digest)| {
                    run.candidate_digest == candidate.artifact_digest
                        && evaluation.qualifying_run_digests.contains(digest)
                })
        })
        .map(|value| value.artifact_digest)
        .collect();
    let completeness_assessment_digest = domain_separated_digest(
        "hsai.completeness-assessment.v1",
        &observations
            .iter()
            .map(|value| (value.completeness, value.completeness_reason.as_str()))
            .collect::<Vec<_>>(),
    )?;
    let bundle = build_evidence_bundle(EvidenceBundleInput {
        claim_id: claim.claim_id.clone(),
        claim_spec_digest: claim.claim_spec_digest()?,
        challenge_descriptor_digest: claim.challenge_descriptor_digest,
        observation_snapshot_digests: observations
            .iter()
            .map(|value| domain_separated_digest("hsai.observation-snapshot.v1", value))
            .collect::<Result<Vec<_>, _>>()?,
        completeness_assessment_digest,
        qualifying_candidate_digests: qualifying_candidates,
        admission_decision_digests: admissions
            .iter()
            .map(|value| value.decision_digest)
            .collect(),
        execution_manifest_digests: execution_digests,
        resolution_aggregation_digest: resolution_aggregation_digest(evaluation)?,
        dispute_record_digests: dispute_record_digests.to_vec(),
        code_build_provenance_digest: domain_separated_digest(
            "hsai.build-provenance.v1",
            &env!("CARGO_PKG_VERSION"),
        )?,
        evidence_publication_policy_digest: domain_separated_digest(
            "hsai.evidence-publication-policy.v1",
            &claim.evidence_publication_policy,
        )?,
        blobs: references,
    })?;
    Ok((bundle, bytes_by_digest))
}

const LOCAL_RECOVERY_DB: &str = "settlement-recovery.sqlite3";
const LOCAL_RECOVERY_REPORT: &str = "local-recovery-report.json";
const LOCAL_RECOVERY_CEILING: &str = "Application-restart recovery using file-backed SQLite and a retained in-memory mock authority; no backend-process durability, host-crash recovery, live status lookup, PostgreSQL runtime recovery, distributed exactly-once delivery, or HA/DR claim.";

/// Exercise one lost-receipt recovery through two real `SQLite` reopen boundaries.
///
/// This use case performs one in-memory mock submission and no network writes.
///
/// # Errors
///
/// Returns a typed construction, persistence, artifact, or invariant failure.
#[allow(clippy::too_many_lines)]
pub async fn run_local_lost_receipt_recovery(
    artifact_root: &Path,
) -> Result<LocalRecoveryReportV1, ApplicationError> {
    fs::create_dir_all(artifact_root)?;
    let database_path = artifact_root.join(LOCAL_RECOVERY_DB);
    if database_path.exists() {
        return Err(ApplicationError::RecoveryInvariant(
            "retained recovery database already exists".to_owned(),
        ));
    }
    let claim = local_demo_claim_draft()?.freeze()?;
    let (bundle, final_resolution, binding, intent) =
        build_local_recovery_prerequisites(&claim, artifact_root)?;
    let request_digest = domain_separated_digest("hsai.mock-hip4-request.v1", &intent)?;
    let store = SqliteStore::connect_path(&database_path, 1).await?;
    store.migrate().await?;
    store
        .freeze_claim(
            &recovery_freeze_context("local-recovery-freeze"),
            &claim,
            &timestamp("2026-07-10T10:45:00Z")?,
            env!("CARGO_PKG_VERSION"),
        )
        .await?;
    store
        .persist_settlement_prerequisites(
            &recovery_prerequisite_context("local-recovery-prerequisites"),
            &claim,
            &bundle,
            &final_resolution,
            &binding,
            &timestamp("2026-07-10T10:46:00Z")?,
        )
        .await?;
    store
        .persist_settlement_intent(
            &recovery_settlement_context("local-recovery-intent"),
            &intent,
            request_digest,
            &timestamp("2026-07-10T10:47:00Z")?,
        )
        .await?;

    let mut backend = Hip4MockBackend::default();
    let lost_receipt = backend.submit(&intent, timestamp("2026-07-10T10:48:00Z")?)?;
    if lost_receipt.request_digest != request_digest {
        return Err(ApplicationError::RecoveryInvariant(
            "mock receipt differs from the pre-submit request digest".to_owned(),
        ));
    }
    drop(lost_receipt);
    store.checkpoint_and_close().await?;

    let restarted = SqliteStore::connect_path(&database_path, 1).await?;
    restarted.migrate().await?;
    let first = restarted
        .settlement_recovery_integrity(&intent.intent_id)
        .await?
        .ok_or_else(|| ApplicationError::RecoveryInvariant("durable intent missing".to_owned()))?;
    if first.recovery.action != SettlementRecoveryAction::QueryExternalStatus
        || first.intent_count != 1
        || first.receipt_count != 0
        || first.reconciliation_count != 0
        || backend.submission_call_count() != 1
        || backend.effect_count() != 1
    {
        return Err(ApplicationError::RecoveryInvariant(
            "first restart did not require authoritative lookup with one effect".to_owned(),
        ));
    }
    let receipt = backend
        .recover_receipt(&first.recovery.intent)?
        .ok_or_else(|| {
            ApplicationError::RecoveryInvariant("authoritative receipt missing".to_owned())
        })?;
    if backend.submission_call_count() != 1 {
        return Err(ApplicationError::RecoveryInvariant(
            "receipt recovery crossed the submission boundary".to_owned(),
        ));
    }
    let receipt_recorded_at = timestamp("2026-07-10T10:48:01Z")?;
    if restarted
        .record_settlement_receipt(
            &recovery_settlement_context("local-recovery-receipt"),
            &receipt,
            &receipt_recorded_at,
        )
        .await?
        != AppendOutcome::Inserted
    {
        return Err(ApplicationError::RecoveryInvariant(
            "recovered receipt was not inserted exactly once".to_owned(),
        ));
    }
    let reconciliation = backend.reconcile(
        &binding,
        &final_resolution,
        timestamp("2026-07-10T10:49:00Z")?,
    )?;
    if restarted
        .record_settlement_reconciliation(
            &recovery_settlement_context("local-recovery-reconciliation"),
            &intent.intent_id,
            &reconciliation,
        )
        .await?
        != AppendOutcome::Inserted
    {
        return Err(ApplicationError::RecoveryInvariant(
            "reconciliation was not inserted exactly once".to_owned(),
        ));
    }
    if restarted
        .record_settlement_receipt(
            &recovery_settlement_context("local-recovery-receipt-replay"),
            &receipt,
            &receipt_recorded_at,
        )
        .await?
        != AppendOutcome::ExistingIdentical
        || restarted
            .record_settlement_reconciliation(
                &recovery_settlement_context("local-recovery-reconciliation-replay"),
                &intent.intent_id,
                &reconciliation,
            )
            .await?
            != AppendOutcome::ExistingIdentical
    {
        return Err(ApplicationError::RecoveryInvariant(
            "exact recovery replay was not idempotent".to_owned(),
        ));
    }
    restarted.checkpoint_and_close().await?;

    let completed_store = SqliteStore::connect_path(&database_path, 1).await?;
    completed_store.migrate().await?;
    let completed = completed_store
        .settlement_recovery_integrity(&intent.intent_id)
        .await?
        .ok_or_else(|| {
            ApplicationError::RecoveryInvariant("completed recovery missing".to_owned())
        })?;
    if completed.recovery.action != SettlementRecoveryAction::Complete
        || completed.intent_count != 1
        || completed.receipt_count != 1
        || completed.reconciliation_count != 1
        || completed.recovery.intent != intent
        || completed.recovery.receipt.as_ref() != Some(&receipt)
        || completed.recovery.latest_reconciliation.as_ref() != Some(&reconciliation)
    {
        return Err(ApplicationError::RecoveryInvariant(
            "terminal durable recovery integrity differs from the exact contract".to_owned(),
        ));
    }
    completed_store.checkpoint_and_close().await?;
    reject_sqlite_sidecars(&database_path)?;

    let receipt_digest = submission_receipt_digest(&receipt)?;
    let reconciliation_digest = domain_separated_digest(
        "hsai.persisted-settlement-reconciliation.v1",
        &reconciliation,
    )?;
    let report = LocalRecoveryReportV1 {
        schema_version: "hsai.local-lost-receipt-recovery-report.v1".to_owned(),
        database_file: LOCAL_RECOVERY_DB.to_owned(),
        intent_id: intent.intent_id.clone(),
        request_digest,
        receipt_digest,
        reconciliation_digest,
        first_restart_action: "QUERY_EXTERNAL_STATUS".to_owned(),
        final_restart_action: "COMPLETE".to_owned(),
        intent_count: completed.intent_count,
        receipt_count: completed.receipt_count,
        reconciliation_count: completed.reconciliation_count,
        submission_call_count: backend.submission_call_count(),
        mock_effect_count: backend.effect_count(),
        claim_ceiling: LOCAL_RECOVERY_CEILING.to_owned(),
    };
    for (name, value) in [
        ("settlement-intent.json", serde_json::to_value(&intent)?),
        ("recovered-receipt.json", serde_json::to_value(&receipt)?),
        (
            "reconciliation.json",
            serde_json::to_value(&reconciliation)?,
        ),
        (LOCAL_RECOVERY_REPORT, serde_json::to_value(&report)?),
    ] {
        fs::write(artifact_root.join(name), serde_json::to_vec_pretty(&value)?)?;
    }
    verify_local_lost_receipt_recovery(artifact_root, &report).await?;
    Ok(report)
}

/// Read-only verification of a retained local recovery database and typed artifacts.
///
/// # Errors
///
/// Returns an artifact, persistence, identity, cardinality, or report mismatch.
pub async fn verify_local_lost_receipt_recovery(
    artifact_root: &Path,
    expected_report: &LocalRecoveryReportV1,
) -> Result<(), ApplicationError> {
    let report: LocalRecoveryReportV1 =
        serde_json::from_slice(&fs::read(artifact_root.join(LOCAL_RECOVERY_REPORT))?)?;
    if &report != expected_report
        || report.schema_version != "hsai.local-lost-receipt-recovery-report.v1"
        || report.database_file != LOCAL_RECOVERY_DB
        || report.claim_ceiling != LOCAL_RECOVERY_CEILING
        || report.first_restart_action != "QUERY_EXTERNAL_STATUS"
        || report.final_restart_action != "COMPLETE"
        || report.submission_call_count != 1
        || report.mock_effect_count != 1
    {
        return Err(ApplicationError::RecoveryInvariant(
            "retained recovery report differs from the expected bounded contract".to_owned(),
        ));
    }
    let intent: UnsignedSettlementIntentV1 =
        serde_json::from_slice(&fs::read(artifact_root.join("settlement-intent.json"))?)?;
    let receipt: SubmissionReceiptV1 =
        serde_json::from_slice(&fs::read(artifact_root.join("recovered-receipt.json"))?)?;
    let reconciliation: ReconciliationResultV1 =
        serde_json::from_slice(&fs::read(artifact_root.join("reconciliation.json"))?)?;
    if intent.intent_id != report.intent_id
        || domain_separated_digest("hsai.mock-hip4-request.v1", &intent)? != report.request_digest
        || submission_receipt_digest(&receipt)? != report.receipt_digest
        || domain_separated_digest(
            "hsai.persisted-settlement-reconciliation.v1",
            &reconciliation,
        )? != report.reconciliation_digest
    {
        return Err(ApplicationError::RecoveryInvariant(
            "exported recovery records fail canonical identity verification".to_owned(),
        ));
    }
    reject_sqlite_sidecars(&artifact_root.join(LOCAL_RECOVERY_DB))?;
    let store = SqliteStore::connect_read_only(&artifact_root.join(LOCAL_RECOVERY_DB)).await?;
    let integrity = store
        .settlement_recovery_integrity(&report.intent_id)
        .await?
        .ok_or_else(|| {
            ApplicationError::RecoveryInvariant("read-only recovery state missing".to_owned())
        })?;
    store.close().await;
    if integrity.recovery.action != SettlementRecoveryAction::Complete
        || integrity.recovery.intent != intent
        || integrity.recovery.receipt != Some(receipt)
        || integrity.recovery.latest_reconciliation != Some(reconciliation)
        || integrity.intent_count != report.intent_count
        || integrity.receipt_count != report.receipt_count
        || integrity.reconciliation_count != report.reconciliation_count
        || integrity.receipt_digest != Some(report.receipt_digest)
        || integrity.latest_reconciliation_digest != Some(report.reconciliation_digest)
    {
        return Err(ApplicationError::RecoveryInvariant(
            "read-only SQLite integrity differs from the retained report".to_owned(),
        ));
    }
    Ok(())
}

fn recovery_freeze_context(request_id: &str) -> MutationContext {
    MutationContext {
        actor_id: "local-beta-recovery".to_owned(),
        request_id: request_id.to_owned(),
        scope: MutationScope::new([
            StateSlice::ClaimRegistryV1,
            StateSlice::OperationsV1,
            StateSlice::DeliveryV1,
        ]),
    }
}

fn recovery_prerequisite_context(request_id: &str) -> MutationContext {
    MutationContext {
        actor_id: "local-beta-recovery".to_owned(),
        request_id: request_id.to_owned(),
        scope: MutationScope::new([
            StateSlice::EvidenceStoreV1,
            StateSlice::ResolutionLedgerV1,
            StateSlice::MarketBindingV1,
            StateSlice::OperationsV1,
            StateSlice::DeliveryV1,
        ]),
    }
}

fn recovery_settlement_context(request_id: &str) -> MutationContext {
    MutationContext {
        actor_id: "local-beta-recovery".to_owned(),
        request_id: request_id.to_owned(),
        scope: MutationScope::new([
            StateSlice::SettlementLedgerV1,
            StateSlice::OperationsV1,
            StateSlice::DeliveryV1,
        ]),
    }
}

fn reject_sqlite_sidecars(database_path: &Path) -> Result<(), ApplicationError> {
    for suffix in ["-wal", "-shm"] {
        let sidecar = std::path::PathBuf::from(format!("{}{suffix}", database_path.display()));
        if sidecar.exists() {
            return Err(ApplicationError::RecoveryInvariant(format!(
                "SQLite sidecar remains after checkpoint: {suffix}"
            )));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn build_local_recovery_prerequisites(
    claim: &outcomes_domain::ClaimSpecV1,
    artifact_root: &Path,
) -> Result<
    (
        OutcomeEvidenceBundleV1,
        ResolutionRecordV1,
        MarketBindingV1,
        UnsignedSettlementIntentV1,
    ),
    ApplicationError,
> {
    let mut ledger =
        LocalSubmissionLedger::new(claim.claim_id.clone(), timestamp("2026-07-10T10:00:00Z")?);
    let candidate = ledger.ingest(
        claim.claim_id.clone(),
        "local.recovery",
        "submission-1",
        "1",
        None,
        timestamp("2026-07-10T10:30:00Z")?,
        timestamp("2026-07-10T10:30:01Z")?,
        "application/octet-stream",
        b"hsai-pass-v1".to_vec(),
    )?;
    let admission = evaluate_admission(claim, &candidate)?;
    let mut runner = ScriptedRunner::exact_bytes_fixture();
    let mut executions = Vec::new();
    for index in 0..2 {
        let started = timestamp("2026-07-10T10:31:00Z")?;
        let ended = timestamp("2026-07-10T10:31:01Z")?;
        executions.push(runner.execute(&RunnerRequest {
            claim,
            candidate: &candidate,
            candidate_bytes: b"hsai-pass-v1",
            admission: &admission,
            run_id: &format!("recovery-run-{index}"),
            started_at: &started,
            ended_at: &ended,
            logical_time: &started,
        })?);
    }
    let cutoff = timestamp("2026-07-10T10:40:00Z")?;
    let observation = ledger.snapshot(None, &cutoff, &timestamp("2026-07-10T10:40:01Z")?)?;
    let evaluation =
        evaluate_existential_claim(claim, &cutoff, observation.completeness, &executions, 0)?;
    if evaluation.truth != ClaimTruth::True {
        return Err(ApplicationError::RecoveryInvariant(
            "recovery prerequisite evaluation is not TRUE".to_owned(),
        ));
    }
    let evidence_root = artifact_root.join("recovery-evidence");
    let (bundle, blobs) = build_application_bundle(
        claim,
        std::slice::from_ref(&observation),
        std::slice::from_ref(&candidate),
        std::slice::from_ref(&admission),
        &executions,
        &[],
        &evaluation,
        Vec::new(),
        &evidence_root,
    )?;
    verify_evidence_bundle(&bundle, &blobs)?;
    let proposed_at = cutoff;
    let dispute_end = proposed_at.checked_add_seconds(claim.dispute_policy.window_seconds)?;
    let policy_digest =
        domain_separated_digest("hsai.resolution-policy.v1", &claim.resolution_policy)?;
    let signer_a = DevelopmentSigner::generate("recovery-attestor-a", "recovery-key-a")?;
    let signer_b = DevelopmentSigner::generate("recovery-attestor-b", "recovery-key-b")?;
    let mut registry = AttestorRegistry::default();
    register_signers(
        &mut registry,
        [
            (&signer_a, "recovery-attestor-a", "recovery-key-a"),
            (&signer_b, "recovery-attestor-b", "recovery-key-b"),
        ],
    )?;
    let provisional_attestations = sign_pair(
        [&signer_a, &signer_b],
        claim,
        &evaluation,
        ResolutionFinality::Provisional,
        policy_digest,
        bundle.bundle_digest,
        bundle.completeness_assessment_digest,
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
        bundle.completeness_assessment_digest,
        &proposed_at,
        &dispute_end,
        &dispute_end,
        2,
    )?;
    let final_resolution = finalize_resolution(
        claim,
        &provisional,
        &dispute_end,
        false,
        false,
        &final_attestations,
        &registry,
    )?;
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
        "local-beta-recovery",
        proposed_at,
        "development-recovery-binding-signature".to_owned(),
    )?;
    let readiness = preflight(
        &binding,
        &final_resolution,
        market.metadata_digest,
        &capabilities,
    );
    let intent = build_intent(
        &binding,
        &final_resolution,
        &readiness,
        dispute_end.checked_add_seconds(3_600)?,
    )?;
    Ok((bundle, final_resolution, binding, intent))
}

#[allow(clippy::too_many_arguments)]
fn sign_superseding_pair(
    signers: [&DevelopmentSigner; 2],
    claim: &outcomes_domain::ClaimSpecV1,
    evaluation: &ResolutionEvaluation,
    finality: ResolutionFinality,
    policy_digest: Sha256Digest,
    evidence_digest: Sha256Digest,
    completeness_digest: Sha256Digest,
    dispute_start: &UtcTimestamp,
    dispute_end: &UtcTimestamp,
    issued_at: &UtcTimestamp,
    sequence: u64,
    predecessor_digest: Sha256Digest,
) -> Result<Vec<ResolutionAttestationV1>, ApplicationError> {
    let claim_digest = claim.claim_spec_digest()?;
    signers
        .into_iter()
        .map(|signer| -> Result<_, ApplicationError> {
            Ok(signer.sign_attestation_superseding(
                claim.claim_id.clone(),
                claim_digest,
                evaluation.truth,
                finality,
                policy_digest,
                evidence_digest,
                evaluation.qualifying_run_digests.clone(),
                Some(completeness_digest),
                dispute_start.clone(),
                dispute_end.clone(),
                issued_at.clone(),
                sequence,
                Some(predecessor_digest),
            )?)
        })
        .collect()
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
        OciSandboxPolicy, docker_environment_digest,
    };

    #[tokio::test]
    async fn contract_local_lost_receipt_recovery_is_retained_and_read_only_verifiable()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let report = run_local_lost_receipt_recovery(directory.path()).await?;
        assert_eq!(report.first_restart_action, "QUERY_EXTERNAL_STATUS");
        assert_eq!(report.final_restart_action, "COMPLETE");
        assert_eq!(report.intent_count, 1);
        assert_eq!(report.receipt_count, 1);
        assert_eq!(report.reconciliation_count, 1);
        assert_eq!(report.submission_call_count, 1);
        assert_eq!(report.mock_effect_count, 1);
        verify_local_lost_receipt_recovery(directory.path(), &report).await?;
        assert!(directory.path().join(LOCAL_RECOVERY_DB).is_file());
        assert!(
            !directory
                .path()
                .join(format!("{LOCAL_RECOVERY_DB}-wal"))
                .exists()
        );
        assert!(
            !directory
                .path()
                .join(format!("{LOCAL_RECOVERY_DB}-shm"))
                .exists()
        );
        Ok(())
    }

    #[tokio::test]
    async fn contract_local_recovery_verifier_rejects_altered_intent_and_missing_receipt()
    -> Result<(), Box<dyn std::error::Error>> {
        let altered = tempfile::tempdir()?;
        let report = run_local_lost_receipt_recovery(altered.path()).await?;
        let intent_path = altered.path().join("settlement-intent.json");
        let mut intent: serde_json::Value = serde_json::from_slice(&fs::read(&intent_path)?)?;
        intent["expires_at"] = serde_json::Value::String("2026-07-10T15:00:01Z".to_owned());
        fs::write(&intent_path, serde_json::to_vec_pretty(&intent)?)?;
        assert!(
            verify_local_lost_receipt_recovery(altered.path(), &report)
                .await
                .is_err()
        );

        let missing = tempfile::tempdir()?;
        let report = run_local_lost_receipt_recovery(missing.path()).await?;
        fs::remove_file(missing.path().join("recovered-receipt.json"))?;
        assert!(
            verify_local_lost_receipt_recovery(missing.path(), &report)
                .await
                .is_err()
        );
        Ok(())
    }

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
    fn acceptance_incomplete_observation_and_divergent_replays_refuse_settlement()
    -> Result<(), Box<dyn std::error::Error>> {
        for (scenario, expected_outcomes, expected_blocker) in [
            (
                LocalScenario::IncompleteObservation,
                Vec::new(),
                "ObservationIncomplete",
            ),
            (
                LocalScenario::DivergentReplays,
                vec![ExecutionOutcome::Pass, ExecutionOutcome::Fail],
                "ReplayDivergence",
            ),
        ] {
            let directory = tempfile::tempdir()?;
            let report = run_local_scenario(scenario, directory.path())?;

            assert_eq!(report.execution_outcomes, expected_outcomes);
            assert_eq!(report.truth, ClaimTruth::Unresolved);
            assert_eq!(report.finality, ResolutionFinality::None);
            assert!(report.evidence_verified);
            assert_eq!(report.mock_effect_count, 0);
            assert_eq!(report.reconciliation_status, None);
            let eligibility = report
                .settlement_eligibility
                .as_ref()
                .ok_or("missing settlement eligibility assessment")?;
            assert!(!eligibility.eligible_for_preflight);
            assert_eq!(
                eligibility.blockers,
                vec![
                    SettlementEligibilityBlocker::ResolutionRequired,
                    SettlementEligibilityBlocker::FinalResolutionRequired,
                ]
            );

            let bundle: outcomes_domain::OutcomeEvidenceBundleV1 =
                serde_json::from_slice(&fs::read(directory.path().join("evidence-bundle.json"))?)?;
            let store = FileBlobStore::new(directory.path());
            let mut standalone_blobs = HashMap::new();
            for reference in &bundle.blobs {
                standalone_blobs.insert(reference.digest, store.get(reference)?);
            }
            verify_evidence_bundle(&bundle, &standalone_blobs)?;
            let observation: outcomes_domain::ObservationSnapshotV1 = serde_json::from_slice(
                &fs::read(directory.path().join("observation-snapshot.json"))?,
            )?;
            let observation_digest =
                domain_separated_digest("hsai.observation-snapshot.v1", &observation)?;
            assert_eq!(
                bundle.observation_snapshot_digests,
                vec![observation_digest]
            );
            assert!(bundle.qualifying_candidate_digests.is_empty());
            assert!(
                bundle.execution_manifest_digests.is_empty()
                    || scenario == LocalScenario::DivergentReplays
            );
            if scenario == LocalScenario::IncompleteObservation {
                assert!(!observation.pagination_complete);
                assert_eq!(
                    observation.completeness,
                    outcomes_domain::CompletenessStatus::Incomplete
                );
                assert!(bundle.execution_manifest_digests.is_empty());
                assert!(bundle.admission_decision_digests.is_empty());
            } else {
                assert_eq!(bundle.execution_manifest_digests.len(), 2);
                assert_eq!(bundle.admission_decision_digests.len(), 1);
            }

            let evaluation: serde_json::Value = serde_json::from_slice(&fs::read(
                directory.path().join("resolution-evaluation.json"),
            )?)?;
            assert_eq!(evaluation["truth"], "UNRESOLVED");
            assert_eq!(evaluation["qualifying_run_digests"], serde_json::json!([]));
            assert_eq!(
                evaluation["blockers"],
                serde_json::json!([expected_blocker])
            );
            assert!(
                directory
                    .path()
                    .join("settlement-eligibility.json")
                    .is_file()
            );
            for forbidden in [
                "provisional-resolution.json",
                "final-resolution.json",
                "settlement-intent.json",
                "mock-receipt.json",
                "mock-reconciliation.json",
            ] {
                assert!(
                    !directory.path().join(forbidden).exists(),
                    "unexpected {forbidden}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn acceptance_invalid_candidate_then_valid_candidate_finalizes_true_once()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let report = run_local_scenario(LocalScenario::InvalidThenValid, directory.path())?;

        assert_eq!(
            report.execution_outcomes,
            vec![
                ExecutionOutcome::Fail,
                ExecutionOutcome::Pass,
                ExecutionOutcome::Pass,
            ]
        );
        assert_eq!(report.truth, ClaimTruth::True);
        assert_eq!(report.finality, ResolutionFinality::Final);
        assert!(report.evidence_verified);
        assert_eq!(report.mock_effect_count, 1);
        assert_eq!(
            report.reconciliation_status,
            Some(ReconciliationStatus::Confirmed)
        );

        let bundle: serde_json::Value =
            serde_json::from_slice(&fs::read(directory.path().join("evidence-bundle.json"))?)?;
        assert_eq!(
            bundle["qualifying_candidate_digests"]
                .as_array()
                .map(Vec::len),
            Some(1),
            "the failed candidate must remain evidence, not become qualifying evidence"
        );
        assert_eq!(
            bundle["blobs"].as_array().map(Vec::len),
            Some(8),
            "observation, both candidates/admissions, and all three executions must be retained"
        );
        Ok(())
    }

    #[test]
    fn acceptance_counterexample_supersedes_provisional_false_and_settles_true_once()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let report = run_local_scenario(
            LocalScenario::DisputedFalseCounterexampleTrue,
            directory.path(),
        )?;
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
        for name in [
            "original-evidence-bundle.json",
            "original-provisional-false.json",
            "original-settlement-preflight-refused.json",
            "counterexample-dispute.json",
            "replacement-evidence-bundle.json",
            "counterexample-decision.json",
            "replacement-provisional-true.json",
            "final-resolution.json",
            "resolution-supersession.json",
            "settlement-intent.json",
            "mock-receipt.json",
            "mock-reconciliation.json",
        ] {
            assert!(directory.path().join(name).is_file(), "missing {name}");
        }
        let predecessor: serde_json::Value = serde_json::from_slice(&fs::read(
            directory.path().join("original-provisional-false.json"),
        )?)?;
        assert_eq!(predecessor["truth"], "FALSE");
        assert_eq!(predecessor["finality"], "PROVISIONAL");
        let refused: serde_json::Value = serde_json::from_slice(&fs::read(
            directory
                .path()
                .join("original-settlement-preflight-refused.json"),
        )?)?;
        assert_eq!(refused["ready"], false);
        assert!(refused["blockers"].as_array().is_some_and(|values| {
            values
                .iter()
                .any(|value| value == "FINAL_RESOLUTION_REQUIRED")
        }));
        assert!(
            !directory
                .path()
                .join("original-settlement-intent.json")
                .exists()
        );
        let successor: serde_json::Value = serde_json::from_slice(&fs::read(
            directory.path().join("resolution-supersession.json"),
        )?)?;
        assert_eq!(
            successor["predecessor_resolution_digest"],
            predecessor["record_digest"]
        );
        Ok(())
    }

    #[test]
    #[ignore = "requires an explicitly selected preloaded local Alpine image"]
    fn external_docker_true_path_reaches_final_mock_settlement()
    -> Result<(), Box<dyn std::error::Error>> {
        let (Ok(image_repository), Ok(image_digest), Ok(platform)) = (
            std::env::var("HSAI_TEST_OCI_IMAGE_REPOSITORY"),
            std::env::var("HSAI_TEST_OCI_IMAGE_DIGEST"),
            std::env::var("HSAI_TEST_OCI_PLATFORM"),
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
        draft.verification_policy.cpu_architecture = platform
            .strip_prefix("linux/")
            .unwrap_or(&platform)
            .to_owned();
        draft.verification_policy.resource_limits.wall_clock_millis = 5_000;
        let policy = OciSandboxPolicy {
            image_repository,
            image_digest,
            platform,
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
        draft.verification_policy.environment_digest = docker_environment_digest(
            &policy,
            &draft.verification_policy.locale,
            &draft.verification_policy.timezone,
        )?;
        let claim = draft.freeze()?;
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
