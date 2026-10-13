//! Pure-data reconciliation of authenticated local execution evidence.
//!
//! This crate rebuilds and binds an authenticated admission, durable replay
//! consumption, metadata-only journal lineage, local execution outcome,
//! provider receipt, metering, route, and settlement intent. Positive reports
//! retain identities only and grant no external authority.

use hyperintel_assurance::VerificationKeyRegistry;
use hyperintel_core::{
    CanonicalizationError, ProtocolId, Sha256Digest, UtcTimestamp, domain_separated_digest,
};
use hyperintel_execution::{
    ChatCompletionChunkV1, ChatFinishReasonV1, ChatMessageRoleV1, CompletedLocalExecutionV1,
    IncompleteExecutionStatusV1, IncompleteLocalExecutionV1, LOCAL_EXECUTION_STATE_SLICE_V1,
    LocalExecutionOutcomeV1,
};
use hyperintel_execution_journal::{
    EXECUTION_JOURNAL_STATE_SLICE_V1, ExecutionJournalError, ExecutionJournalStartDraftV1,
    ExecutionJournalStartV1, ExecutionJournalTerminalDraftV1, ExecutionJournalTerminalStatusV1,
    ExecutionJournalTerminalV1, build_execution_journal_start_v1,
    build_execution_journal_terminal_v1,
};
use hyperintel_market::{
    AuthenticatedExecutionAdmissionV1, AuthenticatedExecutionContextV1, ExecutionEvidenceError,
    RouteDecisionV1, RouteEvaluationOutcomeV1, RoutingError, SettlementError,
    build_metering_from_verified_execution_receipt_v1,
    evaluate_reliability_artifact_verified_route_v1, validate_buyer_replay_state_v1,
    validate_settlement_intent_v1, verify_authenticated_execution_admission_v1,
    verify_signed_provider_execution_receipt_v1,
};
use hyperintel_persistence::{AuthenticatedReplayConsumptionV1, DURABLE_REPLAY_STATE_SLICE_V1};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// State slice emitted by receipt reconciliation.
pub const RECEIPT_RECONCILIATION_STATE_SLICE_V1: &str = "hyperintel-receipt-reconciliation-v1";

/// Closed positive reconciliation status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReceiptReconciliationStatusV1 {
    /// Completed execution, provider receipt, metering, and settlement intent agree.
    Completed,
    /// Durable execution ended by explicit cancellation.
    IncompleteCancelled,
    /// Durable execution exceeded the authenticated deadline.
    IncompleteDeadlineExceeded,
    /// The selected provider failed before returning a completed plan.
    IncompleteProviderFailed,
    /// The selected receipt signer failed after a completed local stream.
    IncompleteReceiptSigningFailed,
}

/// Caller-supplied pure-data inputs for one reconciliation.
pub struct ReceiptReconciliationInputV1<'input, 'context> {
    /// Exact authenticated admission.
    pub admission: &'input AuthenticatedExecutionAdmissionV1,
    /// Exact signed-offer, assurance, reliability, replay, and buyer-policy inputs.
    pub authenticated: &'input AuthenticatedExecutionContextV1<'context>,
    /// Caller-controlled trust registry used to reverify buyer and provider signatures.
    pub verification_keys: &'input VerificationKeyRegistry,
    /// Exact metadata-only start record.
    pub journal_start: &'input ExecutionJournalStartV1,
    /// Exact metadata-only terminal record.
    pub journal_terminal: &'input ExecutionJournalTerminalV1,
    /// Exact completed or incomplete local execution evidence.
    pub execution: &'input LocalExecutionOutcomeV1,
}

/// Identity-only positive reconciliation report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptReconciliationReportV1 {
    /// Fixed report schema.
    pub schema_version: String,
    /// Named state slice.
    pub state_slice: String,
    /// Closed completed or incomplete status.
    pub status: ReceiptReconciliationStatusV1,
    /// Authenticated buyer identifier.
    pub buyer_id: ProtocolId,
    /// Route-selected provider identifier.
    pub provider_id: ProtocolId,
    /// Exact authenticated admission identity.
    pub admission_digest: Sha256Digest,
    /// Exact buyer request-policy identity.
    pub request_policy_digest: Sha256Digest,
    /// Reconstructed full route-evaluation identity.
    pub route_evaluation_digest: Sha256Digest,
    /// Reconstructed selected-route identity.
    pub route_decision_digest: Sha256Digest,
    /// Exact pure replay-transition identity.
    pub replay_transition_digest: Sha256Digest,
    /// Exact durable replay-consumption identity.
    pub replay_consumption_digest: Sha256Digest,
    /// Exact metadata-only start identity.
    pub journal_start_digest: Sha256Digest,
    /// Exact metadata-only terminal identity.
    pub journal_terminal_digest: Sha256Digest,
    /// Completed-execution or incomplete-attempt identity.
    pub execution_outcome_digest: Sha256Digest,
    /// Commitment to the exact in-memory stream, including an empty stream.
    pub stream_digest: Sha256Digest,
    /// Verified provider execution receipt for completed reports only.
    pub provider_receipt_digest: Option<Sha256Digest>,
    /// Reconstructed metering receipt for completed reports only.
    pub metering_receipt_digest: Option<Sha256Digest>,
    /// Reconstructed settlement intent for completed reports only.
    pub settlement_intent_digest: Option<Sha256Digest>,
    /// Reconciliation grants no execution, settlement, or value authority.
    pub authority_granted: bool,
    /// Digest of every preceding field.
    pub report_digest: Sha256Digest,
}

#[derive(Serialize)]
struct ReceiptReconciliationReportIdentityV1<'a> {
    schema_version: &'a str,
    state_slice: &'a str,
    status: ReceiptReconciliationStatusV1,
    buyer_id: &'a ProtocolId,
    provider_id: &'a ProtocolId,
    admission_digest: Sha256Digest,
    request_policy_digest: Sha256Digest,
    route_evaluation_digest: Sha256Digest,
    route_decision_digest: Sha256Digest,
    replay_transition_digest: Sha256Digest,
    replay_consumption_digest: Sha256Digest,
    journal_start_digest: Sha256Digest,
    journal_terminal_digest: Sha256Digest,
    execution_outcome_digest: Sha256Digest,
    stream_digest: Sha256Digest,
    provider_receipt_digest: Option<Sha256Digest>,
    metering_receipt_digest: Option<Sha256Digest>,
    settlement_intent_digest: Option<Sha256Digest>,
    authority_granted: bool,
}

impl ReceiptReconciliationReportV1 {
    /// Recompute the report identity.
    ///
    /// # Errors
    ///
    /// Returns when canonical identity construction fails.
    pub fn digest(&self) -> Result<Sha256Digest, CanonicalizationError> {
        report_digest(self)
    }
}

/// Fail-closed reconciliation failures.
#[derive(Debug, Error)]
pub enum ReceiptReconciliationError {
    /// Admission, buyer policy, route, replay transition, or provider signature failed.
    #[error("receipt reconciliation execution evidence failed: {0}")]
    ExecutionEvidence(#[from] ExecutionEvidenceError),
    /// Route reconstruction failed.
    #[error("receipt reconciliation route reconstruction failed: {0}")]
    Routing(#[from] RoutingError),
    /// Journal record reconstruction failed.
    #[error("receipt reconciliation journal reconstruction failed: {0}")]
    Journal(#[from] ExecutionJournalError),
    /// Settlement-intent reconstruction failed.
    #[error("receipt reconciliation settlement reconstruction failed: {0}")]
    Settlement(#[from] SettlementError),
    /// Canonical identity construction failed.
    #[error("receipt reconciliation canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
    /// The reconstructed route did not select one provider.
    #[error("receipt reconciliation requires one selected route")]
    NoSelectedRoute,
    /// Durable replay consumption is invalid or does not match admission and transition.
    #[error("receipt reconciliation replay-consumption mismatch")]
    ReplayMismatch,
    /// Journal start or terminal does not match the execution lineage.
    #[error("receipt reconciliation journal-lineage mismatch")]
    JournalMismatch,
    /// Journal terminal classification cannot produce a positive report.
    #[error("receipt reconciliation journal terminal is unsupported")]
    UnsupportedJournalTerminal,
    /// Execution schema, identity, status, or authority is invalid.
    #[error("receipt reconciliation execution-outcome mismatch")]
    ExecutionOutcomeMismatch,
    /// Stream commitment or compatibility shape is invalid.
    #[error("receipt reconciliation stream mismatch")]
    StreamMismatch,
    /// Reconstructed metering differs from the completed outcome.
    #[error("receipt reconciliation metering mismatch")]
    MeteringMismatch,
    /// Positive report schema, shape, authority, or digest is invalid.
    #[error("receipt reconciliation report is invalid")]
    InvalidReport,
    /// Materialized report differs from deterministic reconstruction.
    #[error("receipt reconciliation report reconstruction mismatch")]
    ReportMismatch,
}

struct ReconciledOutcomeV1 {
    status: ReceiptReconciliationStatusV1,
    consumption: AuthenticatedReplayConsumptionV1,
    outcome_digest: Sha256Digest,
    stream_digest: Sha256Digest,
    provider_receipt_digest: Option<Sha256Digest>,
    metering_receipt_digest: Option<Sha256Digest>,
    settlement_intent_digest: Option<Sha256Digest>,
    terminal_status: ExecutionJournalTerminalStatusV1,
    finished_at: hyperintel_core::UtcTimestamp,
}

/// Reconcile one exact local execution lineage into an identity-only report.
///
/// # Errors
///
/// Returns without a report on any authentication, replay, journal, stream,
/// receipt, metering, settlement, authority, or identity mismatch.
pub fn reconcile_receipts_v1(
    input: &ReceiptReconciliationInputV1<'_, '_>,
) -> Result<ReceiptReconciliationReportV1, ReceiptReconciliationError> {
    verify_authenticated_execution_admission_v1(
        input.admission,
        input.authenticated,
        input.verification_keys,
    )?;
    let evaluation = evaluate_reliability_artifact_verified_route_v1(&input.authenticated.routing)?;
    let RouteEvaluationOutcomeV1::Selected(route) = evaluation.outcome else {
        return Err(ReceiptReconciliationError::NoSelectedRoute);
    };
    if evaluation.evaluation_digest != input.admission.route_evaluation_digest
        || route.route_digest != input.admission.route_decision_digest
        || route.selected_provider_id != input.admission.provider_id
    {
        return Err(ReceiptReconciliationError::ExecutionOutcomeMismatch);
    }

    let reconciled = reconcile_execution_outcome(input, &route)?;
    validate_replay_consumption(input, &reconciled.consumption)?;
    validate_journal_lineage(input, &reconciled)?;

    let mut report = ReceiptReconciliationReportV1 {
        schema_version: "hyperintel.receipt-reconciliation-report.v1".to_owned(),
        state_slice: RECEIPT_RECONCILIATION_STATE_SLICE_V1.to_owned(),
        status: reconciled.status,
        buyer_id: input.admission.buyer_id.clone(),
        provider_id: input.admission.provider_id.clone(),
        admission_digest: input.admission.admission_digest,
        request_policy_digest: input.admission.request_policy_digest,
        route_evaluation_digest: evaluation.evaluation_digest,
        route_decision_digest: route.route_digest,
        replay_transition_digest: input.authenticated.replay_transition.transition_digest,
        replay_consumption_digest: reconciled.consumption.consumption_digest,
        journal_start_digest: input.journal_start.start_digest,
        journal_terminal_digest: input.journal_terminal.terminal_digest,
        execution_outcome_digest: reconciled.outcome_digest,
        stream_digest: reconciled.stream_digest,
        provider_receipt_digest: reconciled.provider_receipt_digest,
        metering_receipt_digest: reconciled.metering_receipt_digest,
        settlement_intent_digest: reconciled.settlement_intent_digest,
        authority_granted: false,
        report_digest: Sha256Digest::from_bytes([0; 32]),
    };
    report.report_digest = report_digest(&report)?;
    Ok(report)
}

/// Reconstruct and compare one materialized reconciliation report.
///
/// # Errors
///
/// Returns when the report is invalid or differs from the exact input lineage.
pub fn verify_receipt_reconciliation_report_v1(
    report: &ReceiptReconciliationReportV1,
    input: &ReceiptReconciliationInputV1<'_, '_>,
) -> Result<(), ReceiptReconciliationError> {
    validate_report(report)?;
    let rebuilt = reconcile_receipts_v1(input)?;
    if rebuilt == *report {
        Ok(())
    } else {
        Err(ReceiptReconciliationError::ReportMismatch)
    }
}

fn reconcile_execution_outcome(
    input: &ReceiptReconciliationInputV1<'_, '_>,
    route: &RouteDecisionV1,
) -> Result<ReconciledOutcomeV1, ReceiptReconciliationError> {
    match input.execution {
        LocalExecutionOutcomeV1::Completed(completed) => {
            reconcile_completed(input, route, completed)
        }
        LocalExecutionOutcomeV1::Incomplete(incomplete) => reconcile_incomplete(input, incomplete),
    }
}

fn reconcile_completed(
    input: &ReceiptReconciliationInputV1<'_, '_>,
    route: &RouteDecisionV1,
    completed: &CompletedLocalExecutionV1,
) -> Result<ReconciledOutcomeV1, ReceiptReconciliationError> {
    if completed.schema_version != "hyperintel.completed-local-execution.v1"
        || completed.state_slice != LOCAL_EXECUTION_STATE_SLICE_V1
        || completed.authority_granted
        || completed.completion_digest.is_zero()
        || completed.digest()? != completed.completion_digest
    {
        return Err(ReceiptReconciliationError::ExecutionOutcomeMismatch);
    }
    let receipt = &completed.signed_receipt.receipt;
    let stream_digest = validate_stream(
        &completed.chunks,
        input.admission,
        input.authenticated.routing.service.model_version.as_str(),
        Some(
            receipt
                .started_at
                .to_datetime()
                .map_err(|_| ReceiptReconciliationError::StreamMismatch)?
                .unix_timestamp(),
        ),
        true,
    )?;
    if stream_digest != receipt.output_digest {
        return Err(ReceiptReconciliationError::StreamMismatch);
    }
    verify_signed_provider_execution_receipt_v1(
        &completed.signed_receipt,
        input.admission,
        input.authenticated,
        input.verification_keys,
    )?;
    let rebuilt_metering = build_metering_from_verified_execution_receipt_v1(
        &completed.signed_receipt,
        input.admission,
        input.authenticated,
        input.verification_keys,
        None,
    )?;
    if completed
        .metering_receipt
        .buyer_observation_digest
        .is_some()
        || rebuilt_metering != completed.metering_receipt
    {
        return Err(ReceiptReconciliationError::MeteringMismatch);
    }
    let provider = input
        .authenticated
        .routing
        .providers
        .get(&route.selected_provider_id)
        .ok_or(ReceiptReconciliationError::ExecutionOutcomeMismatch)?;
    let offer = input
        .authenticated
        .routing
        .offers
        .iter()
        .map(|signed| &signed.offer)
        .find(|offer| offer.digest().ok() == Some(route.selected_offer_digest))
        .ok_or(ReceiptReconciliationError::ExecutionOutcomeMismatch)?;
    validate_settlement_intent_v1(
        input.authenticated.routing.service,
        provider,
        offer,
        input.authenticated.routing.policy,
        route,
        &completed.metering_receipt,
        &completed.settlement_intent,
    )?;
    if completed.settlement_intent.expires_at != input.authenticated.routing.policy.valid_until {
        return Err(ReceiptReconciliationError::ExecutionOutcomeMismatch);
    }
    Ok(ReconciledOutcomeV1 {
        status: ReceiptReconciliationStatusV1::Completed,
        consumption: completed.replay_consumption.clone(),
        outcome_digest: completed.completion_digest,
        stream_digest,
        provider_receipt_digest: Some(receipt.receipt_digest),
        metering_receipt_digest: Some(completed.metering_receipt.receipt_digest),
        settlement_intent_digest: Some(completed.settlement_intent.intent_digest),
        terminal_status: ExecutionJournalTerminalStatusV1::Completed,
        finished_at: receipt.completed_at.clone(),
    })
}

fn reconcile_incomplete(
    input: &ReceiptReconciliationInputV1<'_, '_>,
    incomplete: &IncompleteLocalExecutionV1,
) -> Result<ReconciledOutcomeV1, ReceiptReconciliationError> {
    if incomplete.schema_version != "hyperintel.incomplete-local-execution.v1"
        || incomplete.state_slice != LOCAL_EXECUTION_STATE_SLICE_V1
        || incomplete.admission_digest != input.admission.admission_digest
        || incomplete.provider_id != input.admission.provider_id
        || incomplete.authority_granted
        || incomplete.attempt_digest.is_zero()
        || incomplete.digest()? != incomplete.attempt_digest
        || incomplete.finished_at < incomplete.replay_consumption.consumed_at
        || incomplete.finished_at > input.admission.deadline_at
    {
        return Err(ReceiptReconciliationError::ExecutionOutcomeMismatch);
    }
    let (status, completed_stream) = match incomplete.status {
        IncompleteExecutionStatusV1::Cancelled => {
            if incomplete.provider_error.is_some()
                || incomplete.finished_at <= incomplete.replay_consumption.consumed_at
            {
                return Err(ReceiptReconciliationError::ExecutionOutcomeMismatch);
            }
            (ReceiptReconciliationStatusV1::IncompleteCancelled, false)
        }
        IncompleteExecutionStatusV1::DeadlineExceeded => {
            if incomplete.provider_error.is_some()
                || incomplete.finished_at != input.admission.deadline_at
            {
                return Err(ReceiptReconciliationError::ExecutionOutcomeMismatch);
            }
            (
                ReceiptReconciliationStatusV1::IncompleteDeadlineExceeded,
                false,
            )
        }
        IncompleteExecutionStatusV1::ProviderFailed => {
            if incomplete.provider_error.is_none()
                || !incomplete.chunks.is_empty()
                || incomplete.finished_at != incomplete.replay_consumption.consumed_at
            {
                return Err(ReceiptReconciliationError::ExecutionOutcomeMismatch);
            }
            (
                ReceiptReconciliationStatusV1::IncompleteProviderFailed,
                false,
            )
        }
        IncompleteExecutionStatusV1::ReceiptSigningFailed => {
            if incomplete.provider_error.is_some() || incomplete.chunks.is_empty() {
                return Err(ReceiptReconciliationError::ExecutionOutcomeMismatch);
            }
            (
                ReceiptReconciliationStatusV1::IncompleteReceiptSigningFailed,
                true,
            )
        }
    };
    let stream_digest = validate_stream(
        &incomplete.chunks,
        input.admission,
        input.authenticated.routing.service.model_version.as_str(),
        None,
        completed_stream,
    )?;
    if stream_digest != incomplete.stream_digest {
        return Err(ReceiptReconciliationError::StreamMismatch);
    }
    validate_incomplete_stream_times(
        &incomplete.chunks,
        &incomplete.replay_consumption.consumed_at,
        &incomplete.finished_at,
        &input.admission.admitted_at,
        &input.admission.deadline_at,
    )?;
    Ok(ReconciledOutcomeV1 {
        status,
        consumption: incomplete.replay_consumption.clone(),
        outcome_digest: incomplete.attempt_digest,
        stream_digest,
        provider_receipt_digest: None,
        metering_receipt_digest: None,
        settlement_intent_digest: None,
        terminal_status: ExecutionJournalTerminalStatusV1::Incomplete,
        finished_at: incomplete.finished_at.clone(),
    })
}

fn validate_replay_consumption(
    input: &ReceiptReconciliationInputV1<'_, '_>,
    consumption: &AuthenticatedReplayConsumptionV1,
) -> Result<(), ReceiptReconciliationError> {
    let transition = input.authenticated.replay_transition;
    if consumption.schema_version != "hyperintel.authenticated-replay-consumption.v1"
        || consumption.state_slice != DURABLE_REPLAY_STATE_SLICE_V1
        || consumption.authority_granted
        || consumption.admission_digest != input.admission.admission_digest
        || consumption.transition_digest != transition.transition_digest
        || consumption.buyer_id != input.admission.buyer_id
        || consumption.buyer_id != transition.buyer_id
        || consumption.consumed_nonce != transition.consumed_nonce
        || consumption.before_state != transition.before_state
        || consumption.after_state != transition.after_state
        || consumption.consumed_at < input.admission.admitted_at
        || consumption.consumed_at > input.admission.deadline_at
        || consumption.consumption_digest.is_zero()
        || consumption.digest()? != consumption.consumption_digest
        || validate_buyer_replay_state_v1(&consumption.before_state).is_err()
        || validate_buyer_replay_state_v1(&consumption.after_state).is_err()
    {
        return Err(ReceiptReconciliationError::ReplayMismatch);
    }
    Ok(())
}

fn validate_journal_lineage(
    input: &ReceiptReconciliationInputV1<'_, '_>,
    outcome: &ReconciledOutcomeV1,
) -> Result<(), ReceiptReconciliationError> {
    let start = input.journal_start;
    let rebuilt_start = build_execution_journal_start_v1(ExecutionJournalStartDraftV1 {
        admission_digest: input.admission.admission_digest,
        buyer_id: input.admission.buyer_id.clone(),
        provider_id: input.admission.provider_id.clone(),
        prompt_digest: input.admission.prompt_digest,
        started_at: start.started_at.clone(),
    })?;
    if rebuilt_start != *start
        || start.state_slice != EXECUTION_JOURNAL_STATE_SLICE_V1
        || start.admission_digest != input.admission.admission_digest
        || start.buyer_id != input.admission.buyer_id
        || start.provider_id != input.admission.provider_id
        || start.prompt_digest != input.admission.prompt_digest
        || start.started_at != outcome.consumption.consumed_at
    {
        return Err(ReceiptReconciliationError::JournalMismatch);
    }
    let terminal = input.journal_terminal;
    if matches!(
        terminal.status,
        ExecutionJournalTerminalStatusV1::RejectedBeforeConsumption
            | ExecutionJournalTerminalStatusV1::FailedAfterConsumption
    ) {
        return Err(ReceiptReconciliationError::UnsupportedJournalTerminal);
    }
    let rebuilt_terminal = build_execution_journal_terminal_v1(
        start,
        ExecutionJournalTerminalDraftV1 {
            consumption_digest: Some(outcome.consumption.consumption_digest),
            outcome_digest: Some(outcome.outcome_digest),
            status: outcome.terminal_status,
            finished_at: outcome.finished_at.clone(),
        },
    )?;
    if rebuilt_terminal != *terminal
        || terminal.state_slice != EXECUTION_JOURNAL_STATE_SLICE_V1
        || terminal.admission_digest != input.admission.admission_digest
        || terminal.start_digest != start.start_digest
        || terminal.consumption_digest != Some(outcome.consumption.consumption_digest)
        || terminal.outcome_digest != Some(outcome.outcome_digest)
        || terminal.status != outcome.terminal_status
        || terminal.finished_at != outcome.finished_at
    {
        return Err(ReceiptReconciliationError::JournalMismatch);
    }
    Ok(())
}

fn validate_stream(
    chunks: &[ChatCompletionChunkV1],
    admission: &AuthenticatedExecutionAdmissionV1,
    expected_model: &str,
    expected_created: Option<i64>,
    completed: bool,
) -> Result<Sha256Digest, ReceiptReconciliationError> {
    if completed && chunks.is_empty() {
        return Err(ReceiptReconciliationError::StreamMismatch);
    }
    let expected_id = format!("chatcmpl-{}", &admission.admission_digest.to_hex()[..24]);
    let first_created = chunks.first().map(|chunk| chunk.created);
    let expected_created = expected_created
        .map(u64::try_from)
        .transpose()
        .map_err(|_| ReceiptReconciliationError::StreamMismatch)?;
    for (index, chunk) in chunks.iter().enumerate() {
        let is_last = index + 1 == chunks.len();
        let expected_finish = (completed && is_last).then_some(ChatFinishReasonV1::Stop);
        if chunk.id != expected_id
            || chunk.object != "chat.completion.chunk"
            || chunk.model != expected_model
            || Some(chunk.created) != first_created
            || expected_created.is_some_and(|value| value != chunk.created)
            || chunk.choices.len() != 1
        {
            return Err(ReceiptReconciliationError::StreamMismatch);
        }
        let choice = &chunk.choices[0];
        if choice.index != 0
            || choice.delta.role != (index == 0).then_some(ChatMessageRoleV1::Assistant)
            || choice.delta.content.as_deref().is_none_or(str::is_empty)
            || choice.finish_reason != expected_finish
        {
            return Err(ReceiptReconciliationError::StreamMismatch);
        }
    }
    Ok(domain_separated_digest(
        "hyperintel.openai-chat-stream.v1",
        &chunks,
    )?)
}

fn validate_incomplete_stream_times(
    chunks: &[ChatCompletionChunkV1],
    consumed_at: &UtcTimestamp,
    finished_at: &UtcTimestamp,
    admitted_at: &UtcTimestamp,
    deadline_at: &UtcTimestamp,
) -> Result<(), ReceiptReconciliationError> {
    let Some(created) = chunks.first().map(|chunk| chunk.created) else {
        return Ok(());
    };
    let created = i64::try_from(created).map_err(|_| ReceiptReconciliationError::StreamMismatch)?;
    let bound = |timestamp: &UtcTimestamp| {
        let value = timestamp
            .to_datetime()
            .map_err(|_| ReceiptReconciliationError::StreamMismatch)?;
        Ok::<i64, ReceiptReconciliationError>(value.unix_timestamp())
    };
    if created < bound(consumed_at)?
        || created > bound(finished_at)?
        || created < bound(admitted_at)?
        || created > bound(deadline_at)?
    {
        return Err(ReceiptReconciliationError::StreamMismatch);
    }
    Ok(())
}

fn validate_report(
    report: &ReceiptReconciliationReportV1,
) -> Result<(), ReceiptReconciliationError> {
    let completed = report.status == ReceiptReconciliationStatusV1::Completed;
    let completed_digests = [
        report.provider_receipt_digest,
        report.metering_receipt_digest,
        report.settlement_intent_digest,
    ];
    let shape_matches = if completed {
        completed_digests.iter().all(Option::is_some)
    } else {
        completed_digests.iter().all(Option::is_none)
    };
    let required = [
        report.admission_digest,
        report.request_policy_digest,
        report.route_evaluation_digest,
        report.route_decision_digest,
        report.replay_transition_digest,
        report.replay_consumption_digest,
        report.journal_start_digest,
        report.journal_terminal_digest,
        report.execution_outcome_digest,
        report.stream_digest,
        report.report_digest,
    ];
    if report.schema_version != "hyperintel.receipt-reconciliation-report.v1"
        || report.state_slice != RECEIPT_RECONCILIATION_STATE_SLICE_V1
        || report.authority_granted
        || !shape_matches
        || required.iter().any(Sha256Digest::is_zero)
        || completed_digests
            .iter()
            .flatten()
            .any(Sha256Digest::is_zero)
        || report.digest()? != report.report_digest
    {
        return Err(ReceiptReconciliationError::InvalidReport);
    }
    Ok(())
}

fn report_digest(
    report: &ReceiptReconciliationReportV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.receipt-reconciliation-report.v1",
        &ReceiptReconciliationReportIdentityV1 {
            schema_version: &report.schema_version,
            state_slice: &report.state_slice,
            status: report.status,
            buyer_id: &report.buyer_id,
            provider_id: &report.provider_id,
            admission_digest: report.admission_digest,
            request_policy_digest: report.request_policy_digest,
            route_evaluation_digest: report.route_evaluation_digest,
            route_decision_digest: report.route_decision_digest,
            replay_transition_digest: report.replay_transition_digest,
            replay_consumption_digest: report.replay_consumption_digest,
            journal_start_digest: report.journal_start_digest,
            journal_terminal_digest: report.journal_terminal_digest,
            execution_outcome_digest: report.execution_outcome_digest,
            stream_digest: report.stream_digest,
            provider_receipt_digest: report.provider_receipt_digest,
            metering_receipt_digest: report.metering_receipt_digest,
            settlement_intent_digest: report.settlement_intent_digest,
            authority_granted: report.authority_granted,
        },
    )
}
