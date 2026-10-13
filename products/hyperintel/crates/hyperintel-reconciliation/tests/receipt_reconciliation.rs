//! Contract coverage for pure-data receipt reconciliation.

use std::{cell::Cell, collections::BTreeMap};

use data_encoding::HEXLOWER;
use ed25519_dalek::{Signer as _, SigningKey};
use hyperintel_assurance::{
    AssuranceRootClassV1, RegisteredVerificationKeyV1, VerificationKeyRegistry,
};
use hyperintel_core::{
    InferenceRequestPolicyV1, ProtocolId, Sha256Digest, UtcTimestamp, domain_separated_digest,
};
use hyperintel_execution::{
    CancellationSignalV1, ChatCompletionMessageV1, ChatCompletionRequestV1, ChatMessageRoleV1,
    LocalExecutionContextV1, LocalExecutionOutcomeV1, LocalProviderAgentV1, ProviderAgentErrorV1,
    ProviderExecutionPlanV1, ProviderReceiptSignerErrorV1, ProviderReceiptSignerV1,
    ProviderStreamFrameV1, chat_completion_request_digest_v1,
};
use hyperintel_execution_journal::{
    ExecutionJournalStartV1, ExecutionJournalTerminalStatusV1, ExecutionJournalTerminalV1,
    SqliteExecutionJournalV1, execute_journaled_local_chat_completion_v1,
};
use hyperintel_market::{
    AuthenticatedExecutionAdmissionV1, AuthenticatedExecutionContextV1, BuyerPolicyStatementV1,
    ProviderExecutionReceiptV1, ProviderExecutionStatementV1,
    ReliabilityArtifactVerifiedRoutingContextV1, ReplayProtectedMarketEvidenceBundleV4,
    ReplayStateTransitionV1, RouteEvaluationOutcomeV1, SignedInferenceRequestPolicyV1,
    SignedReliabilityArtifactV1, build_authenticated_execution_admission_v1,
    build_metering_from_verified_execution_receipt_v1, derive_replay_state_transition_v1,
    derive_settlement_intent_v1, evaluate_reliability_artifact_verified_route_v1,
    verify_replay_protected_market_evidence_bundle_v4,
};
use hyperintel_persistence::{InitializeReplayStateOutcomeV1, SqliteReplayStoreV1};
use hyperintel_reconciliation::{
    ReceiptReconciliationInputV1, ReceiptReconciliationReportV1, ReceiptReconciliationStatusV1,
    reconcile_receipts_v1, verify_receipt_reconciliation_report_v1,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct V4Fixture {
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
    bundle: ReplayProtectedMarketEvidenceBundleV4,
    reliability_artifacts: BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrustedKeysFixture {
    trusted_verification_keys: Vec<RegisteredVerificationKeyV1>,
}

struct ExecutionCase {
    policy: InferenceRequestPolicyV1,
    signed_policy: SignedInferenceRequestPolicyV1,
    replay_transition: ReplayStateTransitionV1,
    admission: AuthenticatedExecutionAdmissionV1,
}

struct ReconciliationCase {
    fixture: V4Fixture,
    registry: VerificationKeyRegistry,
    execution_case: ExecutionCase,
    start: ExecutionJournalStartV1,
    terminal: ExecutionJournalTerminalV1,
    execution: LocalExecutionOutcomeV1,
}

#[derive(Clone, Copy)]
enum CaseMode {
    Completed,
    Cancelled,
    DeadlineExceeded,
    ProviderFailed,
    ReceiptSigningFailed,
}

struct ScriptedProvider {
    provider_id: ProtocolId,
    plan: Option<Result<ProviderExecutionPlanV1, ProviderAgentErrorV1>>,
}

impl LocalProviderAgentV1 for ScriptedProvider {
    fn provider_id(&self) -> &ProtocolId {
        &self.provider_id
    }

    fn execute(
        &mut self,
        _request: &ChatCompletionRequestV1,
        _admission: &AuthenticatedExecutionAdmissionV1,
    ) -> Result<ProviderExecutionPlanV1, ProviderAgentErrorV1> {
        self.plan
            .take()
            .unwrap_or(Err(ProviderAgentErrorV1::ScriptExhausted))
    }
}

struct TestReceiptSigner {
    provider_id: ProtocolId,
    key_id: ProtocolId,
    signing_key: SigningKey,
    calls: Cell<u64>,
    unavailable: bool,
}

impl ProviderReceiptSignerV1 for TestReceiptSigner {
    fn provider_id(&self) -> &ProtocolId {
        &self.provider_id
    }

    fn key_id(&self) -> &ProtocolId {
        &self.key_id
    }

    fn sign(
        &self,
        receipt: &ProviderExecutionReceiptV1,
        admission: &AuthenticatedExecutionAdmissionV1,
    ) -> Result<ProviderExecutionStatementV1, ProviderReceiptSignerErrorV1> {
        self.calls.set(self.calls.get() + 1);
        if self.unavailable {
            return Err(ProviderReceiptSignerErrorV1::Unavailable);
        }
        let mut statement = ProviderExecutionStatementV1 {
            schema_version: "hyperintel.provider-execution-statement.v1".to_owned(),
            receipt_digest: receipt.receipt_digest,
            admission_digest: admission.admission_digest,
            issued_at: receipt.completed_at.clone(),
            expires_at: admission.deadline_at.clone(),
            sequence: 1,
            signer_id: self.provider_id.clone(),
            key_id: self.key_id.clone(),
            algorithm: "ed25519".to_owned(),
            signature_hex: String::new(),
        };
        let message = statement
            .signing_message()
            .map_err(|_| ProviderReceiptSignerErrorV1::Unavailable)?;
        statement.signature_hex = HEXLOWER.encode(&self.signing_key.sign(&message).to_bytes());
        Ok(statement)
    }
}

#[tokio::test]
async fn contract_completed_alpha_beta_reports_are_stable_reconstructable_and_content_free()
-> Result<(), Box<dyn std::error::Error>> {
    let expected = [
        (
            "provider:alpha",
            "202aff32e350c6f605ad4c298bf59ae28b2db696b3ff8adadcfd60ed90d98f4b",
        ),
        (
            "provider:beta",
            "971d90ce87924cf91b28ad974f19d1a4560183ac2cb9b63a1344dcd1c384eeeb",
        ),
    ];
    for (provider, expected_digest) in expected {
        let case = build_case(provider, CaseMode::Completed).await?;
        let report = reconcile_case(&case)?;
        assert_eq!(report.status, ReceiptReconciliationStatusV1::Completed);
        assert_eq!(report.report_digest.to_hex(), expected_digest);
        verify_case_report(&case, &report)?;
        let encoded = serde_json::to_string(&report)?;
        let decoded: ReceiptReconciliationReportV1 = serde_json::from_str(&encoded)?;
        assert_eq!(decoded, report);
        assert_content_free(&encoded);
    }
    Ok(())
}

#[tokio::test]
async fn contract_all_incomplete_statuses_reconcile_without_completed_economics()
-> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        (
            CaseMode::Cancelled,
            ReceiptReconciliationStatusV1::IncompleteCancelled,
        ),
        (
            CaseMode::DeadlineExceeded,
            ReceiptReconciliationStatusV1::IncompleteDeadlineExceeded,
        ),
        (
            CaseMode::ProviderFailed,
            ReceiptReconciliationStatusV1::IncompleteProviderFailed,
        ),
        (
            CaseMode::ReceiptSigningFailed,
            ReceiptReconciliationStatusV1::IncompleteReceiptSigningFailed,
        ),
    ];
    for (mode, expected_status) in cases {
        let case = build_case("provider:beta", mode).await?;
        let report = reconcile_case(&case)?;
        assert_eq!(report.status, expected_status);
        assert_eq!(report.provider_receipt_digest, None);
        assert_eq!(report.metering_receipt_digest, None);
        assert_eq!(report.settlement_intent_digest, None);
        verify_case_report(&case, &report)?;
        assert_content_free(&serde_json::to_string(&report)?);
    }
    Ok(())
}

#[tokio::test]
async fn contract_admission_replay_journal_chunk_receipt_and_authority_tamper_fail_closed()
-> Result<(), Box<dyn std::error::Error>> {
    let case = build_case("provider:alpha", CaseMode::Completed).await?;
    assert_admission_tamper_rejected(&case)?;
    assert_replay_tamper_rejected(&case)?;
    assert_journal_tamper_rejected(&case)?;
    assert_chunk_tamper_rejected(&case)?;
    assert_receipt_tamper_rejected(&case)?;
    assert_authority_tamper_rejected(&case)?;
    Ok(())
}

#[tokio::test]
async fn contract_metering_settlement_status_and_report_tamper_fail_closed()
-> Result<(), Box<dyn std::error::Error>> {
    let completed = build_case("provider:alpha", CaseMode::Completed).await?;
    assert_metering_tamper_rejected(&completed)?;
    assert_self_consistent_buyer_observation_rejected(&completed)?;
    assert_settlement_tamper_rejected(&completed)?;
    assert_report_tamper_rejected(&completed)?;
    assert_completed_terminal_swap_rejected(&completed)?;

    let incomplete = build_case("provider:beta", CaseMode::ProviderFailed).await?;
    assert_incomplete_terminal_swaps_rejected(&incomplete)?;
    Ok(())
}

#[tokio::test]
async fn contract_self_consistent_incomplete_time_impossibilities_fail_closed()
-> Result<(), Box<dyn std::error::Error>> {
    let cancelled = build_case("provider:beta", CaseMode::Cancelled).await?;
    assert_cancelled_at_consumption_rejected(&cancelled)?;
    assert_incomplete_chunk_before_consumption_rejected(&cancelled)?;
    Ok(())
}

fn assert_admission_tamper_rejected(
    case: &ReconciliationCase,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut admission = case.execution_case.admission.clone();
    admission.provider_id = "provider:tampered".parse()?;
    admission.admission_digest = Sha256Digest::from_bytes([0; 32]);
    admission.admission_digest = admission.digest()?;
    assert!(
        reconcile_parts(
            case,
            &admission,
            &case.start,
            &case.terminal,
            &case.execution
        )
        .is_err()
    );
    Ok(())
}

fn assert_replay_tamper_rejected(
    case: &ReconciliationCase,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut execution = case.execution.clone();
    let completed = completed_mut(&mut execution)?;
    completed.replay_consumption.authority_granted = true;
    completed.replay_consumption.consumption_digest = Sha256Digest::from_bytes([0; 32]);
    completed.replay_consumption.consumption_digest = completed.replay_consumption.digest()?;
    refresh_completed_and_terminal(case, execution, |terminal, outcome| {
        terminal.consumption_digest = Some(outcome_consumption_digest(outcome));
    })?;
    Ok(())
}

fn assert_journal_tamper_rejected(
    case: &ReconciliationCase,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut start = case.start.clone();
    start.prompt_digest = Sha256Digest::from_bytes([101; 32]);
    start.start_digest = Sha256Digest::from_bytes([0; 32]);
    start.start_digest = start.digest()?;
    let mut terminal = case.terminal.clone();
    terminal.start_digest = start.start_digest;
    refresh_terminal(&mut terminal)?;
    assert!(
        reconcile_parts(
            case,
            &case.execution_case.admission,
            &start,
            &terminal,
            &case.execution
        )
        .is_err()
    );
    Ok(())
}

fn assert_chunk_tamper_rejected(
    case: &ReconciliationCase,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut execution = case.execution.clone();
    let completed = completed_mut(&mut execution)?;
    let content = completed
        .chunks
        .first_mut()
        .and_then(|chunk| chunk.choices.first_mut())
        .and_then(|choice| choice.delta.content.as_mut())
        .ok_or("missing completed chunk content")?;
    content.push_str(" tampered");
    refresh_completed_and_terminal(case, execution, |_terminal, _outcome| {})?;
    Ok(())
}

fn assert_receipt_tamper_rejected(
    case: &ReconciliationCase,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut execution = case.execution.clone();
    completed_mut(&mut execution)?
        .signed_receipt
        .statement
        .signature_hex
        .push_str("00");
    refresh_completed_and_terminal(case, execution, |_terminal, _outcome| {})?;
    Ok(())
}

fn assert_authority_tamper_rejected(
    case: &ReconciliationCase,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut execution = case.execution.clone();
    completed_mut(&mut execution)?.authority_granted = true;
    refresh_completed_and_terminal(case, execution, |_terminal, _outcome| {})?;
    Ok(())
}

fn assert_metering_tamper_rejected(
    case: &ReconciliationCase,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut execution = case.execution.clone();
    completed_mut(&mut execution)?.metering_receipt.input_tokens += 1;
    refresh_completed_and_terminal(case, execution, |_terminal, _outcome| {})?;
    Ok(())
}

fn assert_self_consistent_buyer_observation_rejected(
    case: &ReconciliationCase,
) -> Result<(), Box<dyn std::error::Error>> {
    let authenticated = case_authenticated_context(case);
    let evaluation = evaluate_reliability_artifact_verified_route_v1(&authenticated.routing)?;
    let RouteEvaluationOutcomeV1::Selected(route) = evaluation.outcome else {
        return Err("expected selected route".into());
    };
    let provider = authenticated
        .routing
        .providers
        .get(&route.selected_provider_id)
        .ok_or("missing selected provider")?;
    let offer = authenticated
        .routing
        .offers
        .iter()
        .map(|signed| &signed.offer)
        .find(|offer| offer.digest().ok() == Some(route.selected_offer_digest))
        .ok_or("missing selected offer")?;
    let mut execution = case.execution.clone();
    let completed = completed_mut(&mut execution)?;
    completed.metering_receipt = build_metering_from_verified_execution_receipt_v1(
        &completed.signed_receipt,
        &case.execution_case.admission,
        &authenticated,
        &case.registry,
        Some(Sha256Digest::from_bytes([77; 32])),
    )?;
    completed.settlement_intent = derive_settlement_intent_v1(
        authenticated.routing.service,
        provider,
        offer,
        authenticated.routing.policy,
        &route,
        &completed.metering_receipt,
        authenticated.routing.policy.valid_until.clone(),
    )?;
    completed.completion_digest = Sha256Digest::from_bytes([0; 32]);
    completed.completion_digest = completed.digest()?;
    let mut terminal = case.terminal.clone();
    terminal.outcome_digest = Some(completed.completion_digest);
    refresh_terminal(&mut terminal)?;
    assert!(
        reconcile_parts(
            case,
            &case.execution_case.admission,
            &case.start,
            &terminal,
            &execution,
        )
        .is_err()
    );
    Ok(())
}

fn assert_settlement_tamper_rejected(
    case: &ReconciliationCase,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut execution = case.execution.clone();
    completed_mut(&mut execution)?
        .settlement_intent
        .intent_id
        .push_str("-tampered");
    refresh_completed_and_terminal(case, execution, |_terminal, _outcome| {})?;
    Ok(())
}

fn assert_report_tamper_rejected(
    case: &ReconciliationCase,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut report = reconcile_case(case)?;
    report.provider_id = "provider:tampered".parse()?;
    report.report_digest = Sha256Digest::from_bytes([0; 32]);
    report.report_digest = report.digest()?;
    assert!(verify_case_report(case, &report).is_err());
    Ok(())
}

fn assert_completed_terminal_swap_rejected(
    case: &ReconciliationCase,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = case.terminal.clone();
    terminal.status = ExecutionJournalTerminalStatusV1::Incomplete;
    refresh_terminal(&mut terminal)?;
    assert!(
        reconcile_parts(
            case,
            &case.execution_case.admission,
            &case.start,
            &terminal,
            &case.execution
        )
        .is_err()
    );
    Ok(())
}

fn assert_incomplete_terminal_swaps_rejected(
    case: &ReconciliationCase,
) -> Result<(), Box<dyn std::error::Error>> {
    for status in [
        ExecutionJournalTerminalStatusV1::Completed,
        ExecutionJournalTerminalStatusV1::RejectedBeforeConsumption,
        ExecutionJournalTerminalStatusV1::FailedAfterConsumption,
    ] {
        let mut terminal = case.terminal.clone();
        terminal.status = status;
        if status == ExecutionJournalTerminalStatusV1::RejectedBeforeConsumption {
            terminal.consumption_digest = None;
            terminal.outcome_digest = None;
        } else if status == ExecutionJournalTerminalStatusV1::FailedAfterConsumption {
            terminal.outcome_digest = None;
        }
        refresh_terminal(&mut terminal)?;
        assert!(
            reconcile_parts(
                case,
                &case.execution_case.admission,
                &case.start,
                &terminal,
                &case.execution
            )
            .is_err()
        );
    }
    Ok(())
}

fn assert_cancelled_at_consumption_rejected(
    case: &ReconciliationCase,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut execution = case.execution.clone();
    let incomplete = incomplete_mut(&mut execution)?;
    incomplete.finished_at = incomplete.replay_consumption.consumed_at.clone();
    refresh_incomplete_and_terminal(case, execution)
}

fn assert_incomplete_chunk_before_consumption_rejected(
    case: &ReconciliationCase,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut execution = case.execution.clone();
    let incomplete = incomplete_mut(&mut execution)?;
    incomplete
        .chunks
        .first_mut()
        .ok_or("expected nonempty cancelled stream")?
        .created = 0;
    incomplete.stream_digest =
        domain_separated_digest("hyperintel.openai-chat-stream.v1", &incomplete.chunks)?;
    refresh_incomplete_and_terminal(case, execution)
}

fn refresh_completed_and_terminal<F>(
    case: &ReconciliationCase,
    mut execution: LocalExecutionOutcomeV1,
    adjust_terminal: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(&mut ExecutionJournalTerminalV1, &LocalExecutionOutcomeV1),
{
    let completed = completed_mut(&mut execution)?;
    completed.completion_digest = Sha256Digest::from_bytes([0; 32]);
    completed.completion_digest = completed.digest()?;
    let mut terminal = case.terminal.clone();
    terminal.outcome_digest = Some(completed.completion_digest);
    adjust_terminal(&mut terminal, &execution);
    refresh_terminal(&mut terminal)?;
    assert!(
        reconcile_parts(
            case,
            &case.execution_case.admission,
            &case.start,
            &terminal,
            &execution
        )
        .is_err()
    );
    Ok(())
}

fn completed_mut(
    execution: &mut LocalExecutionOutcomeV1,
) -> Result<&mut hyperintel_execution::CompletedLocalExecutionV1, Box<dyn std::error::Error>> {
    match execution {
        LocalExecutionOutcomeV1::Completed(value) => Ok(value),
        LocalExecutionOutcomeV1::Incomplete(_) => Err("expected completed execution".into()),
    }
}

fn incomplete_mut(
    execution: &mut LocalExecutionOutcomeV1,
) -> Result<&mut hyperintel_execution::IncompleteLocalExecutionV1, Box<dyn std::error::Error>> {
    match execution {
        LocalExecutionOutcomeV1::Incomplete(value) => Ok(value),
        LocalExecutionOutcomeV1::Completed(_) => Err("expected incomplete execution".into()),
    }
}

fn refresh_incomplete_and_terminal(
    case: &ReconciliationCase,
    mut execution: LocalExecutionOutcomeV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let (outcome_digest, finished_at) = {
        let incomplete = incomplete_mut(&mut execution)?;
        incomplete.attempt_digest = Sha256Digest::from_bytes([0; 32]);
        incomplete.attempt_digest = incomplete.digest()?;
        (incomplete.attempt_digest, incomplete.finished_at.clone())
    };
    let mut terminal = case.terminal.clone();
    terminal.outcome_digest = Some(outcome_digest);
    terminal.finished_at = finished_at;
    refresh_terminal(&mut terminal)?;
    assert!(
        reconcile_parts(
            case,
            &case.execution_case.admission,
            &case.start,
            &terminal,
            &execution,
        )
        .is_err()
    );
    Ok(())
}

fn outcome_consumption_digest(execution: &LocalExecutionOutcomeV1) -> Sha256Digest {
    match execution {
        LocalExecutionOutcomeV1::Completed(value) => value.replay_consumption.consumption_digest,
        LocalExecutionOutcomeV1::Incomplete(value) => value.replay_consumption.consumption_digest,
    }
}

fn refresh_terminal(
    terminal: &mut ExecutionJournalTerminalV1,
) -> Result<(), Box<dyn std::error::Error>> {
    terminal.terminal_digest = Sha256Digest::from_bytes([0; 32]);
    terminal.terminal_digest = terminal.digest()?;
    Ok(())
}

fn reconcile_case(
    case: &ReconciliationCase,
) -> Result<ReceiptReconciliationReportV1, Box<dyn std::error::Error>> {
    reconcile_parts(
        case,
        &case.execution_case.admission,
        &case.start,
        &case.terminal,
        &case.execution,
    )
}

fn reconcile_parts(
    case: &ReconciliationCase,
    admission: &AuthenticatedExecutionAdmissionV1,
    start: &ExecutionJournalStartV1,
    terminal: &ExecutionJournalTerminalV1,
    execution: &LocalExecutionOutcomeV1,
) -> Result<ReceiptReconciliationReportV1, Box<dyn std::error::Error>> {
    let authenticated = case_authenticated_context(case);
    Ok(reconcile_receipts_v1(&ReceiptReconciliationInputV1 {
        admission,
        authenticated: &authenticated,
        verification_keys: &case.registry,
        journal_start: start,
        journal_terminal: terminal,
        execution,
    })?)
}

fn verify_case_report(
    case: &ReconciliationCase,
    report: &ReceiptReconciliationReportV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let authenticated = case_authenticated_context(case);
    Ok(verify_receipt_reconciliation_report_v1(
        report,
        &ReceiptReconciliationInputV1 {
            admission: &case.execution_case.admission,
            authenticated: &authenticated,
            verification_keys: &case.registry,
            journal_start: &case.start,
            journal_terminal: &case.terminal,
            execution: &case.execution,
        },
    )?)
}

fn assert_content_free(encoded: &str) {
    for forbidden in [
        "\"chunks\"",
        "\"messages\"",
        "\"content\"",
        "\"signature_hex\"",
        "\"key_id\"",
        "\"public_key_hex\"",
        "\"credential\"",
        "\"error\"",
    ] {
        assert!(!encoded.contains(forbidden));
    }
}

async fn build_case(
    provider: &str,
    mode: CaseMode,
) -> Result<ReconciliationCase, Box<dyn std::error::Error>> {
    let (fixture, registry, buyer_key) = execution_environment()?;
    let request = request(&fixture, provider);
    let execution_case = execution_case(&fixture, &registry, &buyer_key, &request, provider)?;
    let replay_store = initialized_store(&fixture).await?;
    let journal = SqliteExecutionJournalV1::in_memory().await?;
    journal.migrate().await?;
    let (mut agent, signer, cancellation) = execution_ports(provider, mode, &fixture)?;
    let journaled = {
        let authenticated = authenticated_context(
            &fixture,
            &execution_case.policy,
            &execution_case.signed_policy,
            &execution_case.replay_transition,
            &registry,
        );
        execute_journaled_local_chat_completion_v1(
            &journal,
            local_context(
                &replay_store,
                &execution_case,
                &authenticated,
                &registry,
                &request,
                &cancellation,
            ),
            &mut agent,
            &signer,
        )
        .await?
    };
    if matches!(mode, CaseMode::ReceiptSigningFailed) {
        assert_eq!(signer.calls.get(), 1);
    }
    journal.checkpoint_and_close().await?;
    replay_store.checkpoint_and_close().await?;
    Ok(ReconciliationCase {
        fixture,
        registry,
        execution_case,
        start: journaled.start,
        terminal: journaled.terminal,
        execution: journaled.execution,
    })
}

fn execution_ports(
    provider: &str,
    mode: CaseMode,
    fixture: &V4Fixture,
) -> Result<(ScriptedProvider, TestReceiptSigner, CancellationSignalV1), Box<dyn std::error::Error>>
{
    let mut plan = receipt_plan(provider, fixture)?;
    let mut cancellation = CancellationSignalV1::default();
    let provider_plan = match mode {
        CaseMode::ProviderFailed => Err(ProviderAgentErrorV1::Unavailable),
        CaseMode::DeadlineExceeded => {
            plan.frames.push(ProviderStreamFrameV1 {
                emitted_at: timestamp("2026-07-20T12:05:01Z")?,
                content: " late".to_owned(),
                output_token_delta: 1,
            });
            plan.completed_at = timestamp("2026-07-20T12:06:00Z")?;
            Ok(plan)
        }
        CaseMode::Cancelled => {
            cancellation.requested_at = Some(timestamp("2026-07-20T12:00:03Z")?);
            Ok(plan)
        }
        CaseMode::Completed | CaseMode::ReceiptSigningFailed => Ok(plan),
    };
    let mut signer = receipt_signer(provider)?;
    signer.unavailable = matches!(mode, CaseMode::ReceiptSigningFailed);
    Ok((
        ScriptedProvider {
            provider_id: provider.parse()?,
            plan: Some(provider_plan),
        },
        signer,
        cancellation,
    ))
}

fn case_authenticated_context(case: &ReconciliationCase) -> AuthenticatedExecutionContextV1<'_> {
    authenticated_context(
        &case.fixture,
        &case.execution_case.policy,
        &case.execution_case.signed_policy,
        &case.execution_case.replay_transition,
        &case.registry,
    )
}

fn execution_environment()
-> Result<(V4Fixture, VerificationKeyRegistry, SigningKey), Box<dyn std::error::Error>> {
    let fixture: V4Fixture = serde_json::from_slice(include_bytes!(
        "../../hyperintel-cli/tests/fixtures/evidence-v4.json"
    ))?;
    let trusted: TrustedKeysFixture = serde_json::from_slice(include_bytes!(
        "../../hyperintel-cli/tests/fixtures/trusted-keys-v4.json"
    ))?;
    let buyer_key = SigningKey::from_bytes(&[80; 32]);
    let mut registry = VerificationKeyRegistry::default();
    for key in trusted.trusted_verification_keys {
        registry.register(key)?;
    }
    register_key(
        &mut registry,
        "buyer:demo",
        "key:buyer-policy-1",
        AssuranceRootClassV1::BuyerPolicy,
        &buyer_key,
    )?;
    register_key(
        &mut registry,
        "provider:alpha",
        "key:alpha-receipt",
        AssuranceRootClassV1::ProviderKey,
        &SigningKey::from_bytes(&[81; 32]),
    )?;
    register_key(
        &mut registry,
        "provider:beta",
        "key:beta-receipt",
        AssuranceRootClassV1::ProviderKey,
        &SigningKey::from_bytes(&[82; 32]),
    )?;
    verify_replay_protected_market_evidence_bundle_v4(
        &fixture.bundle,
        &fixture.assurance_blobs,
        &fixture.reliability_artifacts,
        &registry,
    )?;
    Ok((fixture, registry, buyer_key))
}

fn execution_case(
    fixture: &V4Fixture,
    registry: &VerificationKeyRegistry,
    buyer_key: &SigningKey,
    request: &ChatCompletionRequestV1,
    provider: &str,
) -> Result<ExecutionCase, Box<dyn std::error::Error>> {
    let mut policy = fixture.bundle.policy.clone();
    policy.request_id = format!("request:reconcile-{}", provider.replace(':', "-")).parse()?;
    policy.allowed_providers.clear();
    policy.allowed_providers.insert(provider.parse()?);
    policy.prompt_digest = chat_completion_request_digest_v1(request)?;
    let routing = routing_context(fixture, &policy, registry);
    let evaluation = evaluate_reliability_artifact_verified_route_v1(&routing)?;
    let replay_transition = derive_replay_state_transition_v1(
        &fixture.bundle.replay_transition.before_state,
        &policy,
        evaluation.evaluation_digest,
        &fixture.bundle.evaluated_at,
    )?;
    let signed_policy = signed_buyer_policy(&policy, buyer_key)?;
    let authenticated = authenticated_context(
        fixture,
        &policy,
        &signed_policy,
        &replay_transition,
        registry,
    );
    let admission = build_authenticated_execution_admission_v1(
        &authenticated,
        registry,
        timestamp("2026-07-20T12:00:00Z")?,
        timestamp("2026-07-20T12:05:00Z")?,
    )?;
    Ok(ExecutionCase {
        policy,
        signed_policy,
        replay_transition,
        admission,
    })
}

fn authenticated_context<'a>(
    fixture: &'a V4Fixture,
    policy: &'a InferenceRequestPolicyV1,
    signed_policy: &'a SignedInferenceRequestPolicyV1,
    transition: &'a ReplayStateTransitionV1,
    registry: &'a VerificationKeyRegistry,
) -> AuthenticatedExecutionContextV1<'a> {
    AuthenticatedExecutionContextV1 {
        routing: routing_context(fixture, policy, registry),
        replay_transition: transition,
        signed_policy,
    }
}

fn routing_context<'a>(
    fixture: &'a V4Fixture,
    policy: &'a InferenceRequestPolicyV1,
    registry: &'a VerificationKeyRegistry,
) -> ReliabilityArtifactVerifiedRoutingContextV1<'a> {
    ReliabilityArtifactVerifiedRoutingContextV1 {
        evaluated_at: &fixture.bundle.evaluated_at,
        service: &fixture.bundle.service,
        providers: &fixture.bundle.providers,
        offers: &fixture.bundle.offers,
        policy,
        assurance_sets: &fixture.bundle.assurance_sets,
        assurance_blobs: &fixture.assurance_blobs,
        reliability_artifacts: &fixture.reliability_artifacts,
        verification_keys: registry,
    }
}

fn local_context<'a>(
    store: &'a SqliteReplayStoreV1,
    execution_case: &'a ExecutionCase,
    authenticated: &'a AuthenticatedExecutionContextV1<'a>,
    registry: &'a VerificationKeyRegistry,
    request: &'a ChatCompletionRequestV1,
    cancellation: &'a CancellationSignalV1,
) -> LocalExecutionContextV1<'a> {
    LocalExecutionContextV1 {
        replay_store: store,
        admission: &execution_case.admission,
        authenticated,
        verification_keys: registry,
        request,
        consumed_at: &execution_case.admission.admitted_at,
        cancellation,
    }
}

fn request(fixture: &V4Fixture, provider: &str) -> ChatCompletionRequestV1 {
    ChatCompletionRequestV1 {
        model: fixture.bundle.service.model_version.as_str().to_owned(),
        messages: vec![
            ChatCompletionMessageV1 {
                role: ChatMessageRoleV1::Developer,
                content: "Reconcile one local execution.".to_owned(),
            },
            ChatCompletionMessageV1 {
                role: ChatMessageRoleV1::User,
                content: format!("Exercise {provider}."),
            },
        ],
        stream: true,
    }
}

async fn initialized_store(
    fixture: &V4Fixture,
) -> Result<SqliteReplayStoreV1, Box<dyn std::error::Error>> {
    let store = SqliteReplayStoreV1::in_memory().await?;
    store.migrate().await?;
    assert_eq!(
        store
            .initialize_buyer_state(
                &fixture.bundle.replay_transition.before_state,
                &fixture.bundle.evaluated_at,
            )
            .await?,
        InitializeReplayStateOutcomeV1::Inserted
    );
    Ok(store)
}

fn receipt_plan(
    provider: &str,
    fixture: &V4Fixture,
) -> Result<ProviderExecutionPlanV1, Box<dyn std::error::Error>> {
    Ok(ProviderExecutionPlanV1 {
        provider_id: provider.parse()?,
        started_at: timestamp("2026-07-20T12:00:01Z")?,
        frames: vec![
            ProviderStreamFrameV1 {
                emitted_at: timestamp("2026-07-20T12:00:02Z")?,
                content: "local ".to_owned(),
                output_token_delta: 1,
            },
            ProviderStreamFrameV1 {
                emitted_at: timestamp("2026-07-20T12:00:03Z")?,
                content: provider.to_owned(),
                output_token_delta: 1,
            },
        ],
        completed_at: timestamp("2026-07-20T12:00:04Z")?,
        input_tokens: 10,
        runtime_image_digest: fixture.bundle.service.runtime_image_digest,
    })
}

fn receipt_signer(provider: &str) -> Result<TestReceiptSigner, Box<dyn std::error::Error>> {
    let (key_id, seed) = match provider {
        "provider:alpha" => ("key:alpha-receipt", 81_u8),
        "provider:beta" => ("key:beta-receipt", 82_u8),
        _ => return Err("unsupported test provider".into()),
    };
    Ok(TestReceiptSigner {
        provider_id: provider.parse()?,
        key_id: key_id.parse()?,
        signing_key: SigningKey::from_bytes(&[seed; 32]),
        calls: Cell::new(0),
        unavailable: false,
    })
}

fn signed_buyer_policy(
    policy: &InferenceRequestPolicyV1,
    signing_key: &SigningKey,
) -> Result<SignedInferenceRequestPolicyV1, Box<dyn std::error::Error>> {
    let mut statement = BuyerPolicyStatementV1 {
        schema_version: "hyperintel.buyer-policy-statement.v1".to_owned(),
        request_policy_digest: policy.digest()?,
        replay_snapshot_digest: policy.replay_snapshot_digest,
        issued_at: policy.valid_from.clone(),
        expires_at: policy.valid_until.clone(),
        sequence: policy.nonce,
        signer_id: policy.buyer_id.clone(),
        key_id: "key:buyer-policy-1".parse()?,
        algorithm: "ed25519".to_owned(),
        signature_hex: String::new(),
    };
    statement.signature_hex =
        HEXLOWER.encode(&signing_key.sign(&statement.signing_message()?).to_bytes());
    Ok(SignedInferenceRequestPolicyV1 {
        schema_version: "hyperintel.signed-inference-request-policy.v1".to_owned(),
        policy: policy.clone(),
        statement,
    })
}

fn register_key(
    registry: &mut VerificationKeyRegistry,
    signer_id: &str,
    key_id: &str,
    class: AssuranceRootClassV1,
    key: &SigningKey,
) -> Result<(), Box<dyn std::error::Error>> {
    registry.register(RegisteredVerificationKeyV1 {
        signer_id: signer_id.parse()?,
        key_id: key_id.parse()?,
        trust_root_class: class,
        public_key_hex: HEXLOWER.encode(key.verifying_key().as_bytes()),
        valid_from: timestamp("2026-07-20T10:00:00Z")?,
        valid_until: timestamp("2026-07-20T14:00:00Z")?,
        revoked_at: None,
    })?;
    Ok(())
}

fn timestamp(value: &str) -> Result<UtcTimestamp, Box<dyn std::error::Error>> {
    Ok(value.parse()?)
}
