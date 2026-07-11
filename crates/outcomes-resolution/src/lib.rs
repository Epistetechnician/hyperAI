//! Aggregation, quorum, disputes, finality, pauses, and outcome mapping.

use std::collections::HashMap;

use outcomes_domain::{
    ClaimSpecV1, ClaimTruth, CompletenessStatus, DisputeDecisionKind, DisputeDecisionRecordV1,
    DisputeRecordV1, DisputeStatus, ExecutionManifestV1, ExecutionOutcome, ResolutionAttestationV1,
    ResolutionFinality, ResolutionRecordV1, ResolutionSupersessionRecordV1, Sha256Digest,
    UtcTimestamp, domain_separated_digest,
};
use outcomes_evidence::{AttestorRegistry, EvidenceError, verify_quorum};
use serde::Serialize;
use thiserror::Error;

/// Exact reason an existential claim remains unresolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionBlocker {
    /// Submission deadline plus grace period has not elapsed.
    DeadlineOrGraceOpen,
    /// Source observation is incomplete or unknown.
    ObservationIncomplete,
    /// Eligible candidates still require processing.
    PendingEligibleCandidates(u64),
    /// A validator attempt was inconclusive.
    InconclusiveExecution,
    /// A validator or runner attempt errored.
    ErrorExecution,
    /// Required deterministic replays diverged.
    ReplayDivergence,
    /// No configured positive or negative prerequisites are currently satisfied.
    InsufficientEvidence,
}

/// Pure evaluation result before attestations and finality are applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionEvaluation {
    /// Current truth ceiling.
    pub truth: ClaimTruth,
    /// Digests of agreeing PASS runs when truth is TRUE.
    pub qualifying_run_digests: Vec<Sha256Digest>,
    /// Reasons truth remains unresolved.
    pub blockers: Vec<ResolutionBlocker>,
}

/// Digest the exact aggregation payload used by dispute recomputation.
///
/// # Errors
///
/// Returns an error when the aggregation payload cannot be canonicalized.
pub fn resolution_aggregation_digest(
    evaluation: &ResolutionEvaluation,
) -> Result<Sha256Digest, ResolutionError> {
    Ok(domain_separated_digest(
        "hsai.resolution-aggregation.v1",
        &(
            evaluation.truth,
            &evaluation.qualifying_run_digests,
            format!("{:?}", evaluation.blockers),
        ),
    )?)
}

/// Resolution, dispute, quorum, and finality failures.
#[derive(Debug, Error)]
pub enum ResolutionError {
    /// Domain timestamp arithmetic failed.
    #[error("invalid resolution timestamp")]
    Timestamp(#[from] outcomes_domain::TimestampError),
    /// Resolution-critical payload could not be canonicalized.
    #[error("resolution payload is not canonicalizable: {0}")]
    Canonicalization(#[from] outcomes_domain::CanonicalizationError),
    /// Evidence attestation or quorum verification failed.
    #[error("attestation verification failed: {0}")]
    Evidence(#[from] EvidenceError),
    /// A provisional record was requested for unresolved truth.
    #[error("unresolved evaluation cannot produce a resolution record")]
    UnresolvedProposal,
    /// Attestation semantics do not bind the evaluated claim and evidence.
    #[error("attestations do not bind the evaluated claim and evidence")]
    AttestationBindingMismatch,
    /// Finalization was attempted before the dispute window closed.
    #[error("dispute window remains open")]
    DisputeWindowOpen,
    /// A blocking dispute remains active.
    #[error("blocking dispute remains active")]
    BlockingDispute,
    /// Operator pause prevents finalization.
    #[error("claim is paused")]
    Paused,
    /// State is not provisional and cannot be finalized or disputed.
    #[error("resolution is not provisional")]
    NotProvisional,
    /// Dispute reason is absent from the frozen policy.
    #[error("dispute reason {0} is not allowed by the frozen policy")]
    DisallowedReason(String),
    /// Dispute was opened outside its configured window.
    #[error("dispute was opened outside the configured window")]
    DisputeOutsideWindow,
    /// Dispute decision is not a supported terminal state.
    #[error("invalid terminal dispute decision")]
    InvalidDisputeDecision,
    /// A dispute decision or supersession record does not bind its inputs.
    #[error("dispute decision or resolution supersession binding mismatch")]
    SupersessionBindingMismatch,
    /// A bounded dispute field is empty or exceeds its protocol ceiling.
    #[error("invalid bounded dispute decision payload")]
    InvalidDisputePayload,
}

/// Evaluate the existential predicate without mutating operational state.
///
/// PASS requires agreeing replays for one candidate. FAIL rejects only that
/// candidate. ERROR, INCONCLUSIVE, divergence, incomplete observation, pending
/// candidates, or an open deadline keep truth unresolved.
///
/// # Errors
///
/// Returns an error if frozen deadline arithmetic or manifest hashing fails.
pub fn evaluate_existential_claim(
    claim: &ClaimSpecV1,
    now: &UtcTimestamp,
    completeness: CompletenessStatus,
    executions: &[ExecutionManifestV1],
    pending_eligible_candidates: u64,
) -> Result<ResolutionEvaluation, ResolutionError> {
    let mut by_candidate: HashMap<Sha256Digest, Vec<&ExecutionManifestV1>> = HashMap::new();
    for execution in executions {
        by_candidate
            .entry(execution.candidate_digest)
            .or_default()
            .push(execution);
    }
    let required_replays = usize::from(claim.verification_policy.required_replay_count);
    for attempts in by_candidate.values() {
        if attempts.len() >= required_replays
            && attempts
                .iter()
                .all(|attempt| attempt.normalized_outcome == ExecutionOutcome::Pass)
        {
            let qualifying_run_digests = attempts
                .iter()
                .map(|attempt| {
                    domain_separated_digest("hsai.execution-manifest.v1", *attempt)
                        .map_err(ResolutionError::from)
                })
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(ResolutionEvaluation {
                truth: ClaimTruth::True,
                qualifying_run_digests,
                blockers: Vec::new(),
            });
        }
    }

    let mut blockers = Vec::new();
    if by_candidate.values().any(|attempts| {
        attempts
            .iter()
            .any(|attempt| attempt.normalized_outcome == ExecutionOutcome::Pass)
            && attempts
                .iter()
                .any(|attempt| attempt.normalized_outcome != ExecutionOutcome::Pass)
    }) {
        blockers.push(ResolutionBlocker::ReplayDivergence);
    }
    if executions
        .iter()
        .any(|attempt| attempt.normalized_outcome == ExecutionOutcome::Inconclusive)
    {
        blockers.push(ResolutionBlocker::InconclusiveExecution);
    }
    if executions
        .iter()
        .any(|attempt| attempt.normalized_outcome == ExecutionOutcome::Error)
    {
        blockers.push(ResolutionBlocker::ErrorExecution);
    }
    let negative_boundary = claim
        .deadline_policy
        .submission_deadline
        .checked_add_seconds(claim.deadline_policy.ingestion_grace_period_seconds)?;
    if now < &negative_boundary {
        blockers.push(ResolutionBlocker::DeadlineOrGraceOpen);
    }
    if completeness != CompletenessStatus::Complete {
        blockers.push(ResolutionBlocker::ObservationIncomplete);
    }
    if pending_eligible_candidates > 0 {
        blockers.push(ResolutionBlocker::PendingEligibleCandidates(
            pending_eligible_candidates,
        ));
    }
    if blockers.is_empty() {
        Ok(ResolutionEvaluation {
            truth: ClaimTruth::False,
            qualifying_run_digests: Vec::new(),
            blockers,
        })
    } else {
        Ok(ResolutionEvaluation {
            truth: ClaimTruth::Unresolved,
            qualifying_run_digests: Vec::new(),
            blockers,
        })
    }
}

/// Create a quorum-bound provisional resolution record.
///
/// # Errors
///
/// Returns an unresolved, attestation-binding, signature/quorum, timestamp, or
/// canonicalization error.
pub fn propose_resolution(
    claim: &ClaimSpecV1,
    evaluation: &ResolutionEvaluation,
    evidence_bundle_digest: Sha256Digest,
    attestations: &[ResolutionAttestationV1],
    registry: &AttestorRegistry,
    proposed_at: &UtcTimestamp,
) -> Result<ResolutionRecordV1, ResolutionError> {
    if evaluation.truth == ClaimTruth::Unresolved {
        return Err(ResolutionError::UnresolvedProposal);
    }
    verify_attestation_binding(
        claim,
        evaluation,
        evidence_bundle_digest,
        attestations,
        None,
    )?;
    verify_quorum(attestations, registry, claim.attestation_policy.threshold)?;
    let dispute_end = proposed_at.checked_add_seconds(claim.dispute_policy.window_seconds)?;
    build_record(
        claim,
        evaluation.truth,
        ResolutionFinality::Provisional,
        evidence_bundle_digest,
        attestations,
        proposed_at.clone(),
        dispute_end,
    )
}

/// Finalize a provisional record after the window, quorum, disputes, and pause checks.
///
/// # Errors
///
/// Returns a state, window, dispute, pause, attestation, or canonicalization
/// error. The input record is never mutated.
pub fn finalize_resolution(
    claim: &ClaimSpecV1,
    provisional: &ResolutionRecordV1,
    now: &UtcTimestamp,
    blocking_disputes: bool,
    paused: bool,
    final_attestations: &[ResolutionAttestationV1],
    registry: &AttestorRegistry,
) -> Result<ResolutionRecordV1, ResolutionError> {
    if provisional.finality != ResolutionFinality::Provisional {
        return Err(ResolutionError::NotProvisional);
    }
    if now < &provisional.dispute_window_end {
        return Err(ResolutionError::DisputeWindowOpen);
    }
    if blocking_disputes {
        return Err(ResolutionError::BlockingDispute);
    }
    if paused {
        return Err(ResolutionError::Paused);
    }
    let evaluation = ResolutionEvaluation {
        truth: provisional.truth,
        qualifying_run_digests: final_attestations
            .first()
            .map_or_else(Vec::new, |value| value.qualifying_run_digests.clone()),
        blockers: Vec::new(),
    };
    verify_attestation_binding(
        claim,
        &evaluation,
        provisional.evidence_bundle_digest,
        final_attestations,
        None,
    )?;
    if final_attestations
        .iter()
        .any(|attestation| attestation.finality != ResolutionFinality::Final)
    {
        return Err(ResolutionError::AttestationBindingMismatch);
    }
    verify_quorum(
        final_attestations,
        registry,
        claim.attestation_policy.threshold,
    )?;
    build_record(
        claim,
        provisional.truth,
        ResolutionFinality::Final,
        provisional.evidence_bundle_digest,
        final_attestations,
        provisional.dispute_window_start.clone(),
        provisional.dispute_window_end.clone(),
    )
}

/// Open an authenticated append-only dispute inside the frozen window.
///
/// # Errors
///
/// Returns a state, time-window, reason-policy, timestamp, or canonicalization
/// error.
#[allow(clippy::too_many_arguments)]
pub fn open_dispute(
    claim: &ClaimSpecV1,
    provisional: &ResolutionRecordV1,
    actor_id: &str,
    reason_code: &str,
    evidence_digests: Vec<Sha256Digest>,
    counterexample_candidate_digest: Option<Sha256Digest>,
    opened_at: UtcTimestamp,
) -> Result<DisputeRecordV1, ResolutionError> {
    if provisional.finality != ResolutionFinality::Provisional {
        return Err(ResolutionError::NotProvisional);
    }
    if opened_at < provisional.dispute_window_start || opened_at > provisional.dispute_window_end {
        return Err(ResolutionError::DisputeOutsideWindow);
    }
    if !claim
        .dispute_policy
        .allowed_reason_codes
        .iter()
        .any(|allowed| allowed == reason_code)
    {
        return Err(ResolutionError::DisallowedReason(reason_code.to_owned()));
    }
    let payload = (
        &claim.claim_id,
        provisional.record_digest,
        actor_id,
        reason_code,
        &evidence_digests,
        counterexample_candidate_digest,
        &opened_at,
    );
    let record_digest = domain_separated_digest("hsai.dispute-record.v1", &payload)?;
    Ok(DisputeRecordV1 {
        schema_version: "hsai.dispute-record.v1".to_owned(),
        dispute_id: format!("dispute_{}", &record_digest.to_hex()[..32]),
        claim_id: claim.claim_id.clone(),
        provisional_resolution_digest: provisional.record_digest,
        opened_at,
        closes_at: provisional.dispute_window_end.clone(),
        authenticated_actor_id: actor_id.to_owned(),
        reason_code: reason_code.to_owned(),
        evidence_digests,
        counterexample_candidate_digest,
        status: DisputeStatus::Open,
        decision_rationale: None,
        record_digest,
    })
}

/// Append an immutable terminal decision for an immutable open dispute.
///
/// # Errors
///
/// Rejects invalid authority, time, predecessor, counterexample, recomputation,
/// or bounded payload semantics.
#[allow(clippy::too_many_arguments)]
pub fn decide_dispute(
    claim: &ClaimSpecV1,
    predecessor: &ResolutionRecordV1,
    dispute: &DisputeRecordV1,
    authority_id: &str,
    decision: DisputeDecisionKind,
    rationale: String,
    decision_evidence_digests: Vec<Sha256Digest>,
    recomputation_evidence_bundle_digest: Option<Sha256Digest>,
    recomputation_aggregation_digest: Option<Sha256Digest>,
    recomputed: Option<&ResolutionEvaluation>,
    decided_at: UtcTimestamp,
) -> Result<DisputeDecisionRecordV1, ResolutionError> {
    if dispute.status != DisputeStatus::Open
        || dispute.claim_id != claim.claim_id
        || predecessor.claim_id != claim.claim_id
        || dispute.provisional_resolution_digest != predecessor.record_digest
        || predecessor.finality != ResolutionFinality::Provisional
        || authority_id != claim.dispute_policy.authority_id
        || decided_at < dispute.opened_at
        || decided_at > dispute.closes_at
    {
        return Err(ResolutionError::SupersessionBindingMismatch);
    }
    if rationale.trim().is_empty()
        || rationale.len() > 1_024
        || decision_evidence_digests.is_empty()
        || decision_evidence_digests.len() > 32
    {
        return Err(ResolutionError::InvalidDisputePayload);
    }
    if decision == DisputeDecisionKind::AcceptedSupersededBeforeFinality
        && (predecessor.truth != ClaimTruth::False
            || !claim.dispute_policy.counterexample_candidate_allowed
            || dispute.reason_code != "COUNTEREXAMPLE"
            || dispute.counterexample_candidate_digest.is_none()
            || recomputation_evidence_bundle_digest.is_none()
            || recomputation_aggregation_digest.is_none()
            || recomputed
                .is_none_or(|value| value.truth != ClaimTruth::True || !value.blockers.is_empty()))
    {
        return Err(ResolutionError::InvalidDisputeDecision);
    }
    if let (Some(expected), Some(recomputed)) = (recomputation_aggregation_digest, recomputed)
        && expected != resolution_aggregation_digest(recomputed)?
    {
        return Err(ResolutionError::SupersessionBindingMismatch);
    }
    let identity = (
        "hsai.dispute-decision-record.v1",
        &dispute.dispute_id,
        &claim.claim_id,
        dispute.record_digest,
        predecessor.record_digest,
        authority_id,
        &decided_at,
        decision,
        &rationale,
        &decision_evidence_digests,
        recomputation_evidence_bundle_digest,
        recomputation_aggregation_digest,
    );
    let record_digest = domain_separated_digest("hsai.dispute-decision-record.v1", &identity)?;
    Ok(DisputeDecisionRecordV1 {
        schema_version: "hsai.dispute-decision-record.v1".to_owned(),
        decision_id: format!("decision_{}", &record_digest.to_hex()[..32]),
        dispute_id: dispute.dispute_id.clone(),
        claim_id: claim.claim_id.clone(),
        open_dispute_digest: dispute.record_digest,
        provisional_resolution_digest: predecessor.record_digest,
        authority_id: authority_id.to_owned(),
        decided_at,
        decision,
        rationale,
        decision_evidence_digests,
        recomputation_evidence_bundle_digest,
        recomputation_aggregation_digest,
        record_digest,
    })
}

/// Propose a TRUE replacement for a disputed provisional FALSE record.
///
/// # Errors
///
/// Rejects invalid predecessor, decision, recomputation, supersession
/// attestation, or quorum bindings.
#[allow(clippy::too_many_arguments)]
pub fn propose_superseding_resolution(
    claim: &ClaimSpecV1,
    predecessor: &ResolutionRecordV1,
    decision: &DisputeDecisionRecordV1,
    evaluation: &ResolutionEvaluation,
    evidence_bundle_digest: Sha256Digest,
    attestations: &[ResolutionAttestationV1],
    registry: &AttestorRegistry,
) -> Result<ResolutionRecordV1, ResolutionError> {
    if predecessor.finality != ResolutionFinality::Provisional
        || predecessor.truth != ClaimTruth::False
        || decision.claim_id != claim.claim_id
        || decision.provisional_resolution_digest != predecessor.record_digest
        || decision.decision != DisputeDecisionKind::AcceptedSupersededBeforeFinality
        || decision.recomputation_evidence_bundle_digest != Some(evidence_bundle_digest)
        || decision.recomputation_aggregation_digest
            != Some(resolution_aggregation_digest(evaluation)?)
        || evaluation.truth != ClaimTruth::True
        || !evaluation.blockers.is_empty()
    {
        return Err(ResolutionError::SupersessionBindingMismatch);
    }
    verify_attestation_binding(
        claim,
        evaluation,
        evidence_bundle_digest,
        attestations,
        Some(predecessor.record_digest),
    )?;
    verify_quorum(attestations, registry, claim.attestation_policy.threshold)?;
    build_record(
        claim,
        ClaimTruth::True,
        ResolutionFinality::Provisional,
        evidence_bundle_digest,
        attestations,
        predecessor.dispute_window_start.clone(),
        predecessor.dispute_window_end.clone(),
    )
}

/// Finalize an accepted superseding resolution after the original window.
///
/// # Errors
///
/// Rejects invalid predecessor, window, pause, supersession attestation, or
/// quorum bindings.
#[allow(clippy::too_many_arguments)]
pub fn finalize_superseding_resolution(
    claim: &ClaimSpecV1,
    predecessor: &ResolutionRecordV1,
    provisional: &ResolutionRecordV1,
    now: &UtcTimestamp,
    paused: bool,
    final_attestations: &[ResolutionAttestationV1],
    registry: &AttestorRegistry,
) -> Result<ResolutionRecordV1, ResolutionError> {
    if predecessor.finality != ResolutionFinality::Provisional
        || predecessor.truth != ClaimTruth::False
        || provisional.finality != ResolutionFinality::Provisional
        || provisional.truth != ClaimTruth::True
        || now < &provisional.dispute_window_end
        || paused
    {
        return Err(ResolutionError::SupersessionBindingMismatch);
    }
    let evaluation = ResolutionEvaluation {
        truth: ClaimTruth::True,
        qualifying_run_digests: final_attestations
            .first()
            .map_or_else(Vec::new, |value| value.qualifying_run_digests.clone()),
        blockers: Vec::new(),
    };
    verify_attestation_binding(
        claim,
        &evaluation,
        provisional.evidence_bundle_digest,
        final_attestations,
        Some(predecessor.record_digest),
    )?;
    if final_attestations
        .iter()
        .any(|value| value.finality != ResolutionFinality::Final)
    {
        return Err(ResolutionError::AttestationBindingMismatch);
    }
    verify_quorum(
        final_attestations,
        registry,
        claim.attestation_policy.threshold,
    )?;
    build_record(
        claim,
        ClaimTruth::True,
        ResolutionFinality::Final,
        provisional.evidence_bundle_digest,
        final_attestations,
        provisional.dispute_window_start.clone(),
        provisional.dispute_window_end.clone(),
    )
}

/// Bind the acyclic predecessor-decision-successor supersession edge.
///
/// # Errors
///
/// Rejects a final predecessor or mismatched claim, decision, or successor.
pub fn link_resolution_supersession(
    predecessor: &ResolutionRecordV1,
    decision: &DisputeDecisionRecordV1,
    successor: &ResolutionRecordV1,
    linked_at: UtcTimestamp,
) -> Result<ResolutionSupersessionRecordV1, ResolutionError> {
    if predecessor.finality == ResolutionFinality::Final
        || predecessor.claim_id != successor.claim_id
        || decision.claim_id != predecessor.claim_id
        || decision.provisional_resolution_digest != predecessor.record_digest
        || decision.decision != DisputeDecisionKind::AcceptedSupersededBeforeFinality
        || successor.truth != ClaimTruth::True
        || !matches!(
            successor.finality,
            ResolutionFinality::Provisional | ResolutionFinality::Final
        )
    {
        return Err(ResolutionError::SupersessionBindingMismatch);
    }
    let identity = (
        "hsai.resolution-supersession-record.v1",
        &predecessor.claim_id,
        predecessor.record_digest,
        decision.record_digest,
        successor.record_digest,
        &linked_at,
    );
    let record_digest =
        domain_separated_digest("hsai.resolution-supersession-record.v1", &identity)?;
    Ok(ResolutionSupersessionRecordV1 {
        schema_version: "hsai.resolution-supersession-record.v1".to_owned(),
        supersession_id: format!("supersession_{}", &record_digest.to_hex()[..32]),
        claim_id: predecessor.claim_id.clone(),
        predecessor_resolution_digest: predecessor.record_digest,
        dispute_decision_digest: decision.record_digest,
        successor_resolution_digest: successor.record_digest,
        linked_at,
        record_digest,
    })
}

fn verify_attestation_binding(
    claim: &ClaimSpecV1,
    evaluation: &ResolutionEvaluation,
    evidence_bundle_digest: Sha256Digest,
    attestations: &[ResolutionAttestationV1],
    expected_supersession: Option<Sha256Digest>,
) -> Result<(), ResolutionError> {
    let claim_digest = claim
        .claim_spec_digest()
        .map_err(|_| ResolutionError::AttestationBindingMismatch)?;
    let policy_digest =
        domain_separated_digest("hsai.resolution-policy.v1", &claim.resolution_policy)?;
    if attestations.is_empty()
        || attestations.iter().any(|attestation| {
            attestation.claim_id != claim.claim_id
                || attestation.claim_spec_digest != claim_digest
                || attestation.truth != evaluation.truth
                || attestation.resolution_policy_digest != policy_digest
                || attestation.evidence_bundle_digest != evidence_bundle_digest
                || attestation.qualifying_run_digests != evaluation.qualifying_run_digests
                || attestation.supersedes_non_settling_record != expected_supersession
        })
    {
        return Err(ResolutionError::AttestationBindingMismatch);
    }
    Ok(())
}

#[derive(Serialize)]
struct ResolutionIdentity<'a> {
    schema_version: &'static str,
    claim_id: &'a outcomes_domain::ClaimId,
    truth: ClaimTruth,
    finality: ResolutionFinality,
    evidence_bundle_digest: Sha256Digest,
    attestation_digests: &'a [Sha256Digest],
    dispute_window_start: &'a UtcTimestamp,
    dispute_window_end: &'a UtcTimestamp,
}

#[allow(clippy::too_many_arguments)]
fn build_record(
    claim: &ClaimSpecV1,
    truth: ClaimTruth,
    finality: ResolutionFinality,
    evidence_bundle_digest: Sha256Digest,
    attestations: &[ResolutionAttestationV1],
    dispute_window_start: UtcTimestamp,
    dispute_window_end: UtcTimestamp,
) -> Result<ResolutionRecordV1, ResolutionError> {
    let attestation_digests = attestations
        .iter()
        .map(|attestation| {
            domain_separated_digest("hsai.resolution-attestation-record.v1", attestation)
                .map_err(ResolutionError::from)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let identity = ResolutionIdentity {
        schema_version: "hsai.resolution-record.v1",
        claim_id: &claim.claim_id,
        truth,
        finality,
        evidence_bundle_digest,
        attestation_digests: &attestation_digests,
        dispute_window_start: &dispute_window_start,
        dispute_window_end: &dispute_window_end,
    };
    let record_digest = domain_separated_digest("hsai.resolution-record.v1", &identity)?;
    Ok(ResolutionRecordV1 {
        schema_version: "hsai.resolution-record.v1".to_owned(),
        resolution_id: format!("resolution_{}", &record_digest.to_hex()[..32]),
        claim_id: claim.claim_id.clone(),
        truth,
        finality,
        evidence_bundle_digest,
        attestation_digests,
        dispute_window_start,
        dispute_window_end,
        record_digest,
    })
}

#[cfg(test)]
mod tests {
    use outcomes_domain::{DeterminismTier, ResourceUsageV1, fixtures::local_demo_claim_draft};
    use outcomes_evidence::{DevelopmentSigner, RegisteredAttestor};

    use super::*;

    fn timestamp(value: &str) -> Result<UtcTimestamp, outcomes_domain::TimestampError> {
        value.parse()
    }

    fn execution(
        claim: &ClaimSpecV1,
        candidate_byte: u8,
        run_id: &str,
        outcome: ExecutionOutcome,
    ) -> Result<ExecutionManifestV1, Box<dyn std::error::Error>> {
        let digest = Sha256Digest::from_bytes([candidate_byte; 32]);
        Ok(ExecutionManifestV1 {
            schema_version: "hsai.execution-manifest.v1".to_owned(),
            run_id: run_id.to_owned(),
            claim_spec_digest: claim.claim_spec_digest()?,
            candidate_digest: digest,
            admission_decision_digest: digest,
            verifier_digest: claim.verification_policy.validator_artifact_digest,
            environment_digest: claim.verification_policy.environment_digest,
            runner_implementation: "test".to_owned(),
            runner_build_version: "test".to_owned(),
            host_compatibility_facts: std::collections::BTreeMap::new(),
            argv: claim.verification_policy.argv.clone(),
            environment: std::collections::BTreeMap::new(),
            sandbox_policy_digest: digest,
            input_manifest_digest: digest,
            output_manifest_digest: Some(digest),
            resource_limits_digest: digest,
            resource_usage: ResourceUsageV1 {
                cpu_millis: 1,
                peak_memory_bytes: 1,
                output_bytes: 1,
                disk_bytes: 0,
                wall_clock_millis: 1,
            },
            started_at: timestamp("2026-07-10T10:30:00Z")?,
            ended_at: timestamp("2026-07-10T10:30:01Z")?,
            logical_time: timestamp("2026-07-10T10:30:00Z")?,
            raw_exit_status: Some(0),
            signal: None,
            timed_out: false,
            out_of_memory: false,
            infrastructure_classification: None,
            stdout_digest: digest,
            stderr_digest: digest,
            normalized_outcome: outcome,
            normalization_policy_version: "v1".to_owned(),
            determinism_tier: DeterminismTier::SemanticIdentical,
            declared_nondeterminism: Vec::new(),
        })
    }

    #[test]
    fn contract_fail_before_deadline_remains_unresolved() -> Result<(), Box<dyn std::error::Error>>
    {
        let claim = local_demo_claim_draft()?.freeze()?;
        let result = evaluate_existential_claim(
            &claim,
            &timestamp("2026-07-10T11:59:59Z")?,
            CompletenessStatus::Complete,
            &[execution(&claim, 1, "run-fail", ExecutionOutcome::Fail)?],
            0,
        )?;
        assert_eq!(result.truth, ClaimTruth::Unresolved);
        assert!(
            result
                .blockers
                .contains(&ResolutionBlocker::DeadlineOrGraceOpen)
        );
        Ok(())
    }

    #[test]
    fn contract_complete_empty_source_becomes_false_only_after_grace()
    -> Result<(), Box<dyn std::error::Error>> {
        let claim = local_demo_claim_draft()?.freeze()?;
        let before = evaluate_existential_claim(
            &claim,
            &timestamp("2026-07-10T12:04:59Z")?,
            CompletenessStatus::Complete,
            &[],
            0,
        )?;
        let after = evaluate_existential_claim(
            &claim,
            &timestamp("2026-07-10T12:05:00Z")?,
            CompletenessStatus::Complete,
            &[],
            0,
        )?;
        assert_eq!(before.truth, ClaimTruth::Unresolved);
        assert_eq!(after.truth, ClaimTruth::False);
        Ok(())
    }

    #[test]
    fn contract_error_and_incomplete_observation_block_false()
    -> Result<(), Box<dyn std::error::Error>> {
        let claim = local_demo_claim_draft()?.freeze()?;
        let result = evaluate_existential_claim(
            &claim,
            &timestamp("2026-07-10T12:10:00Z")?,
            CompletenessStatus::Incomplete,
            &[execution(&claim, 1, "run-error", ExecutionOutcome::Error)?],
            0,
        )?;
        assert_eq!(result.truth, ClaimTruth::Unresolved);
        assert!(result.blockers.contains(&ResolutionBlocker::ErrorExecution));
        assert!(
            result
                .blockers
                .contains(&ResolutionBlocker::ObservationIncomplete)
        );
        Ok(())
    }

    #[test]
    fn contract_two_agreeing_pass_replays_establish_true() -> Result<(), Box<dyn std::error::Error>>
    {
        let claim = local_demo_claim_draft()?.freeze()?;
        let executions = [
            execution(&claim, 1, "run-pass-1", ExecutionOutcome::Pass)?,
            execution(&claim, 1, "run-pass-2", ExecutionOutcome::Pass)?,
        ];
        let result = evaluate_existential_claim(
            &claim,
            &timestamp("2026-07-10T10:40:00Z")?,
            CompletenessStatus::Incomplete,
            &executions,
            0,
        )?;
        assert_eq!(result.truth, ClaimTruth::True);
        assert_eq!(result.qualifying_run_digests.len(), 2);
        Ok(())
    }

    #[test]
    fn contract_divergent_replays_remain_unresolved() -> Result<(), Box<dyn std::error::Error>> {
        let claim = local_demo_claim_draft()?.freeze()?;
        let executions = [
            execution(&claim, 1, "run-pass", ExecutionOutcome::Pass)?,
            execution(&claim, 1, "run-fail", ExecutionOutcome::Fail)?,
        ];
        let result = evaluate_existential_claim(
            &claim,
            &timestamp("2026-07-10T12:10:00Z")?,
            CompletenessStatus::Complete,
            &executions,
            0,
        )?;
        assert_eq!(result.truth, ClaimTruth::Unresolved);
        assert!(
            result
                .blockers
                .contains(&ResolutionBlocker::ReplayDivergence)
        );
        Ok(())
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn contract_quorum_dispute_window_and_finality_are_enforced()
    -> Result<(), Box<dyn std::error::Error>> {
        let claim = local_demo_claim_draft()?.freeze()?;
        let executions = [
            execution(&claim, 1, "run-pass-1", ExecutionOutcome::Pass)?,
            execution(&claim, 1, "run-pass-2", ExecutionOutcome::Pass)?,
        ];
        let evaluation = evaluate_existential_claim(
            &claim,
            &timestamp("2026-07-10T10:40:00Z")?,
            CompletenessStatus::Incomplete,
            &executions,
            0,
        )?;
        let evidence_digest = Sha256Digest::from_bytes([44; 32]);
        let policy_digest =
            domain_separated_digest("hsai.resolution-policy.v1", &claim.resolution_policy)?;
        let start = timestamp("2026-07-10T10:40:00Z")?;
        let end = timestamp("2026-07-10T10:50:00Z")?;
        let signer_a = DevelopmentSigner::generate("attestor-a", "key-a")?;
        let signer_b = DevelopmentSigner::generate("attestor-b", "key-b")?;
        let mut registry = AttestorRegistry::default();
        for (signer, signer_id, key_id) in [
            (&signer_a, "attestor-a", "key-a"),
            (&signer_b, "attestor-b", "key-b"),
        ] {
            registry.register(RegisteredAttestor {
                signer_id: signer_id.to_owned(),
                key_id: key_id.to_owned(),
                verifying_key: signer.verifying_key(),
                active_from: timestamp("2026-07-10T00:00:00Z")?,
                revoked_at: None,
            });
        }
        let provisional_attestations = vec![
            signer_a.sign_attestation(
                claim.claim_id.clone(),
                claim.claim_spec_digest()?,
                ClaimTruth::True,
                ResolutionFinality::Provisional,
                policy_digest,
                evidence_digest,
                evaluation.qualifying_run_digests.clone(),
                None,
                start.clone(),
                end.clone(),
                start.clone(),
                1,
            )?,
            signer_b.sign_attestation(
                claim.claim_id.clone(),
                claim.claim_spec_digest()?,
                ClaimTruth::True,
                ResolutionFinality::Provisional,
                policy_digest,
                evidence_digest,
                evaluation.qualifying_run_digests.clone(),
                None,
                start.clone(),
                end.clone(),
                start.clone(),
                1,
            )?,
        ];
        let provisional = propose_resolution(
            &claim,
            &evaluation,
            evidence_digest,
            &provisional_attestations,
            &registry,
            &start,
        )?;
        assert!(matches!(
            finalize_resolution(
                &claim,
                &provisional,
                &timestamp("2026-07-10T10:49:59Z")?,
                false,
                false,
                &provisional_attestations,
                &registry,
            ),
            Err(ResolutionError::DisputeWindowOpen)
        ));
        let dispute = open_dispute(
            &claim,
            &provisional,
            "operator-local",
            "COUNTEREXAMPLE",
            vec![Sha256Digest::from_bytes([45; 32])],
            Some(Sha256Digest::from_bytes([46; 32])),
            timestamp("2026-07-10T10:45:00Z")?,
        )?;
        assert_eq!(dispute.status, DisputeStatus::Open);
        let issued_at = timestamp("2026-07-10T10:51:00Z")?;
        let final_attestations = vec![
            signer_a.sign_attestation(
                claim.claim_id.clone(),
                claim.claim_spec_digest()?,
                ClaimTruth::True,
                ResolutionFinality::Final,
                policy_digest,
                evidence_digest,
                evaluation.qualifying_run_digests.clone(),
                None,
                start.clone(),
                end.clone(),
                issued_at.clone(),
                2,
            )?,
            signer_b.sign_attestation(
                claim.claim_id.clone(),
                claim.claim_spec_digest()?,
                ClaimTruth::True,
                ResolutionFinality::Final,
                policy_digest,
                evidence_digest,
                evaluation.qualifying_run_digests.clone(),
                None,
                start.clone(),
                end.clone(),
                issued_at.clone(),
                2,
            )?,
        ];
        assert!(matches!(
            finalize_resolution(
                &claim,
                &provisional,
                &issued_at,
                true,
                false,
                &final_attestations,
                &registry,
            ),
            Err(ResolutionError::BlockingDispute)
        ));
        let final_record = finalize_resolution(
            &claim,
            &provisional,
            &issued_at,
            false,
            false,
            &final_attestations,
            &registry,
        )?;
        assert_eq!(final_record.finality, ResolutionFinality::Final);
        assert_eq!(final_record.truth, ClaimTruth::True);
        Ok(())
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn contract_counterexample_supersession_is_append_only_and_quorum_bound()
    -> Result<(), Box<dyn std::error::Error>> {
        let claim = local_demo_claim_draft()?.freeze()?;
        let start = timestamp("2026-07-10T12:05:00Z")?;
        let end = timestamp("2026-07-10T12:15:00Z")?;
        let false_evidence = Sha256Digest::from_bytes([70; 32]);
        let predecessor = build_record(
            &claim,
            ClaimTruth::False,
            ResolutionFinality::Provisional,
            false_evidence,
            &[],
            start.clone(),
            end.clone(),
        )?;
        let counterexample = Sha256Digest::from_bytes([71; 32]);
        let dispute = open_dispute(
            &claim,
            &predecessor,
            "authenticated-challenger",
            "COUNTEREXAMPLE",
            vec![Sha256Digest::from_bytes([72; 32])],
            Some(counterexample),
            timestamp("2026-07-10T12:07:00Z")?,
        )?;
        let executions = [
            execution(&claim, 71, "counterexample-pass-1", ExecutionOutcome::Pass)?,
            execution(&claim, 71, "counterexample-pass-2", ExecutionOutcome::Pass)?,
        ];
        let evaluation = evaluate_existential_claim(
            &claim,
            &timestamp("2026-07-10T12:08:00Z")?,
            CompletenessStatus::Complete,
            &executions,
            0,
        )?;
        let replacement_evidence = Sha256Digest::from_bytes([73; 32]);
        assert!(matches!(
            decide_dispute(
                &claim,
                &predecessor,
                &dispute,
                "local-demo-operator",
                DisputeDecisionKind::AcceptedSupersededBeforeFinality,
                "Two deterministic verifier replays accept the eligible counterexample.".to_owned(),
                vec![Sha256Digest::from_bytes([74; 32])],
                Some(replacement_evidence),
                Some(Sha256Digest::from_bytes([75; 32])),
                Some(&evaluation),
                timestamp("2026-07-10T12:09:00Z")?,
            ),
            Err(ResolutionError::SupersessionBindingMismatch)
        ));
        let decision = decide_dispute(
            &claim,
            &predecessor,
            &dispute,
            "local-demo-operator",
            DisputeDecisionKind::AcceptedSupersededBeforeFinality,
            "Two deterministic verifier replays accept the eligible counterexample.".to_owned(),
            vec![Sha256Digest::from_bytes([74; 32])],
            Some(replacement_evidence),
            Some(resolution_aggregation_digest(&evaluation)?),
            Some(&evaluation),
            timestamp("2026-07-10T12:09:00Z")?,
        )?;
        assert_eq!(dispute.status, DisputeStatus::Open);

        let signer_a = DevelopmentSigner::generate("attestor-a", "key-a")?;
        let signer_b = DevelopmentSigner::generate("attestor-b", "key-b")?;
        let mut registry = AttestorRegistry::default();
        for (signer, signer_id, key_id) in [
            (&signer_a, "attestor-a", "key-a"),
            (&signer_b, "attestor-b", "key-b"),
        ] {
            registry.register(RegisteredAttestor {
                signer_id: signer_id.to_owned(),
                key_id: key_id.to_owned(),
                verifying_key: signer.verifying_key(),
                active_from: timestamp("2026-07-10T00:00:00Z")?,
                revoked_at: None,
            });
        }
        let policy_digest =
            domain_separated_digest("hsai.resolution-policy.v1", &claim.resolution_policy)?;
        let claim_spec_digest = claim.claim_spec_digest()?;
        let sign = |signer: &DevelopmentSigner,
                    finality: ResolutionFinality,
                    issued_at: UtcTimestamp,
                    sequence: u64| {
            signer.sign_attestation_superseding(
                claim.claim_id.clone(),
                claim_spec_digest,
                ClaimTruth::True,
                finality,
                policy_digest,
                replacement_evidence,
                evaluation.qualifying_run_digests.clone(),
                None,
                start.clone(),
                end.clone(),
                issued_at,
                sequence,
                Some(predecessor.record_digest),
            )
        };
        let provisional_attestations = vec![
            sign(
                &signer_a,
                ResolutionFinality::Provisional,
                decision.decided_at.clone(),
                2,
            )?,
            sign(
                &signer_b,
                ResolutionFinality::Provisional,
                decision.decided_at.clone(),
                2,
            )?,
        ];
        assert!(matches!(
            propose_resolution(
                &claim,
                &evaluation,
                replacement_evidence,
                &provisional_attestations,
                &registry,
                &decision.decided_at,
            ),
            Err(ResolutionError::AttestationBindingMismatch)
        ));
        let replacement = propose_superseding_resolution(
            &claim,
            &predecessor,
            &decision,
            &evaluation,
            replacement_evidence,
            &provisional_attestations,
            &registry,
        )?;
        let link = link_resolution_supersession(
            &predecessor,
            &decision,
            &replacement,
            decision.decided_at.clone(),
        )?;
        assert_eq!(
            link.predecessor_resolution_digest,
            predecessor.record_digest
        );

        let finalized_at = timestamp("2026-07-10T12:16:00Z")?;
        let final_attestations = vec![
            sign(
                &signer_a,
                ResolutionFinality::Final,
                finalized_at.clone(),
                3,
            )?,
            sign(
                &signer_b,
                ResolutionFinality::Final,
                finalized_at.clone(),
                3,
            )?,
        ];
        let final_record = finalize_superseding_resolution(
            &claim,
            &predecessor,
            &replacement,
            &finalized_at,
            false,
            &final_attestations,
            &registry,
        )?;
        assert_eq!(
            (final_record.truth, final_record.finality),
            (ClaimTruth::True, ResolutionFinality::Final)
        );
        assert!(
            link_resolution_supersession(&final_record, &decision, &replacement, finalized_at,)
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn contract_accepted_dispute_rejects_missing_counterexample_and_unbounded_payload()
    -> Result<(), Box<dyn std::error::Error>> {
        let claim = local_demo_claim_draft()?.freeze()?;
        let start = timestamp("2026-07-10T12:05:00Z")?;
        let predecessor = build_record(
            &claim,
            ClaimTruth::False,
            ResolutionFinality::Provisional,
            Sha256Digest::from_bytes([80; 32]),
            &[],
            start.clone(),
            timestamp("2026-07-10T12:15:00Z")?,
        )?;
        let dispute = open_dispute(
            &claim,
            &predecessor,
            "challenger",
            "COUNTEREXAMPLE",
            vec![Sha256Digest::from_bytes([81; 32])],
            None,
            timestamp("2026-07-10T12:06:00Z")?,
        )?;
        let evaluation = ResolutionEvaluation {
            truth: ClaimTruth::True,
            qualifying_run_digests: vec![Sha256Digest::from_bytes([82; 32])],
            blockers: Vec::new(),
        };
        assert!(matches!(
            decide_dispute(
                &claim,
                &predecessor,
                &dispute,
                "local-demo-operator",
                DisputeDecisionKind::AcceptedSupersededBeforeFinality,
                "valid rationale".to_owned(),
                vec![Sha256Digest::from_bytes([83; 32])],
                Some(Sha256Digest::from_bytes([84; 32])),
                Some(Sha256Digest::from_bytes([85; 32])),
                Some(&evaluation),
                timestamp("2026-07-10T12:07:00Z")?,
            ),
            Err(ResolutionError::InvalidDisputeDecision)
        ));
        assert!(matches!(
            decide_dispute(
                &claim,
                &predecessor,
                &dispute,
                "local-demo-operator",
                DisputeDecisionKind::Rejected,
                " ".to_owned(),
                vec![],
                None,
                None,
                None,
                timestamp("2026-07-10T12:07:00Z")?,
            ),
            Err(ResolutionError::InvalidDisputePayload)
        ));
        Ok(())
    }
}
