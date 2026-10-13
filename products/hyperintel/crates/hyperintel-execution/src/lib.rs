//! Hermetic execution harness for one durably consumed authenticated admission.
//!
//! The crate exposes a deliberately narrow, text-only subset of the `OpenAI` Chat
//! Completions request and streaming-chunk shapes. Provider and receipt-signing
//! implementations are injected in process. This crate opens no socket, reads
//! no ambient credential, and submits no settlement or value movement.

mod reroute;

use hyperintel_assurance::VerificationKeyRegistry;
use hyperintel_core::{
    CanonicalizationError, ProtocolId, Sha256Digest, UtcTimestamp, domain_separated_digest,
};
use hyperintel_market::{
    AuthenticatedExecutionAdmissionV1, AuthenticatedExecutionContextV1, ExecutionEvidenceError,
    MeteringReceiptV1, ProviderExecutionReceiptDraftV1, ProviderExecutionReceiptV1,
    ProviderExecutionStatementV1, RouteEvaluationOutcomeV1, RoutingError, SettlementError,
    SettlementIntentV1, SignedProviderExecutionReceiptV1,
    build_metering_from_verified_execution_receipt_v1, build_provider_execution_receipt_v1,
    derive_settlement_intent_v1, evaluate_reliability_artifact_verified_route_v1,
    verify_signed_provider_execution_receipt_v1,
};
use hyperintel_persistence::{
    AuthenticatedReplayConsumptionV1, ConsumeReplayOutcomeV1, ReplayPersistenceError,
    SqliteReplayStoreV1,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use reroute::{
    AUTHENTICATED_REROUTE_STATE_SLICE_V1, AuthenticatedRerouteContextV1, AuthenticatedRerouteError,
    AuthenticatedRerouteExecutionV1, AuthenticatedRerouteRecordV1,
    execute_authenticated_reroute_v1,
};

/// State slice emitted by the local execution harness.
pub const LOCAL_EXECUTION_STATE_SLICE_V1: &str = "hyperintel-local-execution-harness-v1";

const CHAT_COMPLETION_CHUNK_OBJECT: &str = "chat.completion.chunk";

/// Supported text-message roles in the local compatibility subset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatMessageRoleV1 {
    /// Application instruction with precedence over user messages.
    Developer,
    /// Legacy system instruction.
    System,
    /// End-user input.
    User,
    /// Prior assistant text.
    Assistant,
}

/// One text-only chat message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatCompletionMessageV1 {
    /// Message role.
    pub role: ChatMessageRoleV1,
    /// Nonempty text content.
    pub content: String,
}

/// Text-only streaming Chat Completions request supported by the local harness.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatCompletionRequestV1 {
    /// Exact selected service model-version identifier.
    pub model: String,
    /// Ordered input messages.
    pub messages: Vec<ChatCompletionMessageV1>,
    /// Must be true for this streaming-only slice.
    pub stream: bool,
}

/// Assistant delta carried by one local streaming chunk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatCompletionDeltaV1 {
    /// Assistant role on the first chunk only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<ChatMessageRoleV1>,
    /// Text emitted by the provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Supported terminal reason for a completed local stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatFinishReasonV1 {
    /// The provider completed normally.
    Stop,
}

/// One choice within a local streaming chunk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatCompletionChunkChoiceV1 {
    /// Zero-based choice index. V1 supports exactly index zero.
    pub index: u32,
    /// Incremental assistant content.
    pub delta: ChatCompletionDeltaV1,
    /// Present only on the final chunk of a completed stream.
    pub finish_reason: Option<ChatFinishReasonV1>,
}

/// OpenAI-shaped text-stream chunk returned by the local harness.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatCompletionChunkV1 {
    /// Stable completion identifier shared by every chunk in the attempt.
    pub id: String,
    /// Fixed `chat.completion.chunk` object type.
    pub object: String,
    /// Unix seconds at provider start, shared by every chunk.
    pub created: u64,
    /// Exact selected service model-version identifier.
    pub model: String,
    /// V1 contains exactly one choice.
    pub choices: Vec<ChatCompletionChunkChoiceV1>,
}

/// One deterministic provider frame before compatibility-shape construction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderStreamFrameV1 {
    /// Explicit frame emission time.
    pub emitted_at: UtcTimestamp,
    /// Nonempty text delta.
    pub content: String,
    /// Nonzero provider-reported token increment for this frame.
    pub output_token_delta: u64,
}

/// Completed deterministic plan returned by an injected provider.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderExecutionPlanV1 {
    /// Provider that produced the plan.
    pub provider_id: ProtocolId,
    /// Provider execution start.
    pub started_at: UtcTimestamp,
    /// Ordered text frames.
    pub frames: Vec<ProviderStreamFrameV1>,
    /// Provider completion time.
    pub completed_at: UtcTimestamp,
    /// Provider-reported input token count.
    pub input_tokens: u64,
    /// Exact runtime commitment observed by the provider adapter.
    pub runtime_image_digest: Sha256Digest,
}

/// Closed provider-port failures exposed without arbitrary log text.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderAgentErrorV1 {
    /// The selected in-process provider is unavailable.
    #[error("selected provider is unavailable")]
    Unavailable,
    /// The deterministic provider script has no result remaining.
    #[error("scripted provider is exhausted")]
    ScriptExhausted,
}

/// Versioned, in-process provider execution port.
pub trait LocalProviderAgentV1 {
    /// Provider identity implemented by this adapter.
    fn provider_id(&self) -> &ProtocolId;

    /// Execute one already authenticated and durably consumed request.
    ///
    /// # Errors
    ///
    /// Returns a closed provider failure without a completion plan.
    fn execute(
        &mut self,
        request: &ChatCompletionRequestV1,
        admission: &AuthenticatedExecutionAdmissionV1,
    ) -> Result<ProviderExecutionPlanV1, ProviderAgentErrorV1>;
}

/// Closed receipt-signer failure exposed without signing-key material.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderReceiptSignerErrorV1 {
    /// The injected signer could not produce a statement.
    #[error("provider receipt signer is unavailable")]
    Unavailable,
}

/// Injected provider receipt-signing port.
pub trait ProviderReceiptSignerV1 {
    /// Provider identity implemented by this signer.
    fn provider_id(&self) -> &ProtocolId;

    /// Configured provider receipt-key identifier.
    fn key_id(&self) -> &ProtocolId;

    /// Sign one exact completed execution receipt.
    ///
    /// # Errors
    ///
    /// Returns when the configured signer is unavailable.
    fn sign(
        &self,
        receipt: &ProviderExecutionReceiptV1,
        admission: &AuthenticatedExecutionAdmissionV1,
    ) -> Result<ProviderExecutionStatementV1, ProviderReceiptSignerErrorV1>;
}

/// Explicit deterministic cancellation signal for one local attempt.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancellationSignalV1 {
    /// When present, frames at or after this time are not released.
    pub requested_at: Option<UtcTimestamp>,
}

/// Terminal state for a non-completed local attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IncompleteExecutionStatusV1 {
    /// The explicit deterministic cancellation time was reached.
    Cancelled,
    /// The authenticated admission deadline was exceeded.
    DeadlineExceeded,
    /// The injected provider failed before returning a plan.
    ProviderFailed,
    /// The injected receipt signer failed after provider completion.
    ReceiptSigningFailed,
}

/// Non-completed attempt after durable replay consumption.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncompleteLocalExecutionV1 {
    /// Fixed attempt schema.
    pub schema_version: String,
    /// Named local execution state slice.
    pub state_slice: String,
    /// Exact authenticated admission.
    pub admission_digest: Sha256Digest,
    /// Selected provider.
    pub provider_id: ProtocolId,
    /// Durable one-shot replay consumption.
    pub replay_consumption: AuthenticatedReplayConsumptionV1,
    /// Terminal non-completed status.
    pub status: IncompleteExecutionStatusV1,
    /// Closed provider failure when `status` is `PROVIDER_FAILED`.
    pub provider_error: Option<ProviderAgentErrorV1>,
    /// Frames released before the terminal boundary.
    pub chunks: Vec<ChatCompletionChunkV1>,
    /// Commitment to the released chunks, including an empty stream.
    pub stream_digest: Sha256Digest,
    /// Explicit terminal time.
    pub finished_at: UtcTimestamp,
    /// Local evidence grants no execution, settlement, or value authority.
    pub authority_granted: bool,
    /// Digest of every preceding semantic field.
    pub attempt_digest: Sha256Digest,
}

/// Completed local execution and its reconstructed evidence chain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedLocalExecutionV1 {
    /// Fixed completion schema.
    pub schema_version: String,
    /// Named local execution state slice.
    pub state_slice: String,
    /// Durable one-shot replay consumption.
    pub replay_consumption: AuthenticatedReplayConsumptionV1,
    /// Complete OpenAI-shaped text stream.
    pub chunks: Vec<ChatCompletionChunkV1>,
    /// Provider-signed completed execution receipt.
    pub signed_receipt: SignedProviderExecutionReceiptV1,
    /// Metering reconstructed from the verified provider receipt.
    pub metering_receipt: MeteringReceiptV1,
    /// Deterministic non-authoritative settlement instruction.
    pub settlement_intent: SettlementIntentV1,
    /// Local evidence grants no execution, settlement, or value authority.
    pub authority_granted: bool,
    /// Digest of every preceding semantic field.
    pub completion_digest: Sha256Digest,
}

impl IncompleteLocalExecutionV1 {
    /// Recompute the incomplete-attempt identity.
    ///
    /// # Errors
    ///
    /// Returns when canonical identity construction fails.
    pub fn digest(&self) -> Result<Sha256Digest, CanonicalizationError> {
        incomplete_digest(self)
    }
}

impl CompletedLocalExecutionV1 {
    /// Recompute the completed-execution identity.
    ///
    /// # Errors
    ///
    /// Returns when canonical identity construction fails.
    pub fn digest(&self) -> Result<Sha256Digest, CanonicalizationError> {
        completed_digest(self)
    }
}

/// Terminal result after a durable replay transition was newly consumed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "outcome",
    content = "record",
    rename_all = "SCREAMING_SNAKE_CASE"
)]
pub enum LocalExecutionOutcomeV1 {
    /// Provider completed and the full local evidence chain reconstructed.
    Completed(Box<CompletedLocalExecutionV1>),
    /// The attempt did not complete and has no metering or settlement intent.
    Incomplete(Box<IncompleteLocalExecutionV1>),
}

/// Borrowed dependencies and exact request for one local attempt.
pub struct LocalExecutionContextV1<'a> {
    /// Durable replay store.
    pub replay_store: &'a SqliteReplayStoreV1,
    /// Exact authenticated admission.
    pub admission: &'a AuthenticatedExecutionAdmissionV1,
    /// Inputs required to reconstruct the admission and route.
    pub authenticated: &'a AuthenticatedExecutionContextV1<'a>,
    /// Caller-controlled verification keys.
    pub verification_keys: &'a VerificationKeyRegistry,
    /// Text-only streaming request.
    pub request: &'a ChatCompletionRequestV1,
    /// Explicit replay-consumption time.
    pub consumed_at: &'a UtcTimestamp,
    /// Explicit deterministic cancellation signal.
    pub cancellation: &'a CancellationSignalV1,
}

/// Fail-closed local harness errors.
#[derive(Debug, Error)]
pub enum LocalExecutionError {
    /// The compatibility request is empty, non-streaming, or uses the wrong model.
    #[error("invalid text-only streaming chat request")]
    InvalidRequest,
    /// The exact request does not match the buyer prompt commitment.
    #[error("chat request commitment does not match authenticated admission")]
    PromptCommitmentMismatch,
    /// The provider adapter does not implement the selected provider.
    #[error("provider adapter does not match the selected provider")]
    ProviderBindingMismatch,
    /// The receipt signer does not match the selected provider and configured key.
    #[error("provider receipt signer does not match the selected provider key")]
    SignerBindingMismatch,
    /// Cancellation at or before consumption prevents nonce use and provider invocation.
    #[error("request was cancelled before durable replay consumption")]
    CancelledBeforeConsumption,
    /// The exact admission was already consumed and cannot execute twice.
    #[error("authenticated admission was already consumed")]
    AdmissionAlreadyConsumed,
    /// The provider plan has invalid binding, time ordering, content, or usage.
    #[error("provider execution plan is invalid")]
    InvalidProviderPlan,
    /// A timestamp cannot be represented as nonnegative Unix seconds.
    #[error("provider start time cannot be represented as Unix seconds")]
    InvalidUnixTimestamp,
    /// Durable replay verification or mutation failed.
    #[error("durable replay consumption failed: {0}")]
    Persistence(#[from] ReplayPersistenceError),
    /// Authenticated execution evidence failed reconstruction.
    #[error("execution evidence failed: {0}")]
    ExecutionEvidence(#[from] ExecutionEvidenceError),
    /// Route reconstruction failed.
    #[error("route reconstruction failed: {0}")]
    Routing(#[from] RoutingError),
    /// Settlement-intent reconstruction failed.
    #[error("settlement reconstruction failed: {0}")]
    Settlement(#[from] SettlementError),
    /// Canonical identity construction failed.
    #[error("local execution canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
}

/// Compute the exact request commitment required by the buyer policy.
///
/// # Errors
///
/// Returns when canonical JSON construction fails.
pub fn chat_completion_request_digest_v1(
    request: &ChatCompletionRequestV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest("hyperintel.openai-chat-request.v1", request)
}

/// Execute one newly consumed authenticated admission through injected local ports.
///
/// The provider is invoked only after a newly applied durable replay transition.
/// Existing-identical consumption is rejected so an exact retry cannot execute
/// twice. All returned records remain local, non-authoritative evidence.
///
/// # Errors
///
/// Returns before provider invocation on request, binding, authentication,
/// replay, or persistence failure. It also returns on invalid provider evidence
/// or receipt-to-settlement reconstruction failure.
pub async fn execute_local_chat_completion_v1<P, S>(
    context: LocalExecutionContextV1<'_>,
    provider: &mut P,
    signer: &S,
) -> Result<LocalExecutionOutcomeV1, LocalExecutionError>
where
    P: LocalProviderAgentV1,
    S: ProviderReceiptSignerV1,
{
    preflight(&context, provider, signer)?;
    let consumption = consume_once(&context).await?;
    let plan = match provider.execute(context.request, context.admission) {
        Ok(value) => value,
        Err(error) => {
            return Ok(LocalExecutionOutcomeV1::Incomplete(Box::new(
                build_incomplete(
                    &context,
                    consumption,
                    IncompleteExecutionStatusV1::ProviderFailed,
                    Some(error),
                    Vec::new(),
                    context.consumed_at.clone(),
                )?,
            )));
        }
    };
    validate_provider_plan(&context, &plan)?;
    if let Some(incomplete) = terminal_incomplete(&context, &plan, &consumption)? {
        return Ok(LocalExecutionOutcomeV1::Incomplete(Box::new(incomplete)));
    }
    complete_execution(&context, consumption, plan, signer)
}

fn preflight<P, S>(
    context: &LocalExecutionContextV1<'_>,
    provider: &P,
    signer: &S,
) -> Result<(), LocalExecutionError>
where
    P: LocalProviderAgentV1,
    S: ProviderReceiptSignerV1,
{
    let request = context.request;
    if !request.stream
        || request.messages.is_empty()
        || request
            .messages
            .iter()
            .any(|message| message.content.is_empty())
        || request.model != context.authenticated.routing.service.model_version.as_str()
    {
        return Err(LocalExecutionError::InvalidRequest);
    }
    if chat_completion_request_digest_v1(request)? != context.admission.prompt_digest {
        return Err(LocalExecutionError::PromptCommitmentMismatch);
    }
    if provider.provider_id() != &context.admission.provider_id {
        return Err(LocalExecutionError::ProviderBindingMismatch);
    }
    let selected = context
        .authenticated
        .routing
        .providers
        .get(&context.admission.provider_id)
        .ok_or(LocalExecutionError::ProviderBindingMismatch)?;
    if signer.provider_id() != &context.admission.provider_id
        || signer.key_id() != &selected.receipt_key_id
    {
        return Err(LocalExecutionError::SignerBindingMismatch);
    }
    if context
        .cancellation
        .requested_at
        .as_ref()
        .is_some_and(|requested_at| requested_at <= context.consumed_at)
    {
        return Err(LocalExecutionError::CancelledBeforeConsumption);
    }
    Ok(())
}

async fn consume_once(
    context: &LocalExecutionContextV1<'_>,
) -> Result<AuthenticatedReplayConsumptionV1, LocalExecutionError> {
    match context
        .replay_store
        .consume_authenticated_admission(
            context.admission,
            context.authenticated,
            context.verification_keys,
            context.consumed_at,
        )
        .await?
    {
        ConsumeReplayOutcomeV1::Applied(value) => Ok(value),
        ConsumeReplayOutcomeV1::ExistingIdentical(_) => {
            Err(LocalExecutionError::AdmissionAlreadyConsumed)
        }
    }
}

fn validate_provider_plan(
    context: &LocalExecutionContextV1<'_>,
    plan: &ProviderExecutionPlanV1,
) -> Result<(), LocalExecutionError> {
    if plan.provider_id != context.admission.provider_id
        || plan.runtime_image_digest != context.authenticated.routing.service.runtime_image_digest
        || plan.frames.is_empty()
        || plan.started_at < context.admission.admitted_at
        || plan.completed_at < plan.started_at
    {
        return Err(LocalExecutionError::InvalidProviderPlan);
    }
    let mut preceding = &plan.started_at;
    let mut output_tokens = 0_u64;
    for frame in &plan.frames {
        if frame.content.is_empty()
            || frame.output_token_delta == 0
            || frame.emitted_at <= *preceding
            || frame.emitted_at > plan.completed_at
        {
            return Err(LocalExecutionError::InvalidProviderPlan);
        }
        output_tokens = output_tokens
            .checked_add(frame.output_token_delta)
            .ok_or(LocalExecutionError::InvalidProviderPlan)?;
        preceding = &frame.emitted_at;
    }
    let routing = &context.authenticated.routing;
    if plan.input_tokens > routing.policy.max_input_tokens
        || plan.input_tokens > routing.service.max_input_tokens
        || output_tokens > routing.policy.max_output_tokens
        || output_tokens > routing.service.max_output_tokens
    {
        return Err(LocalExecutionError::InvalidProviderPlan);
    }
    Ok(())
}

fn terminal_incomplete(
    context: &LocalExecutionContextV1<'_>,
    plan: &ProviderExecutionPlanV1,
    consumption: &AuthenticatedReplayConsumptionV1,
) -> Result<Option<IncompleteLocalExecutionV1>, LocalExecutionError> {
    let cancellation = context.cancellation.requested_at.as_ref();
    let cancellation_wins = cancellation.is_some_and(|requested_at| {
        requested_at <= &plan.completed_at && requested_at <= &context.admission.deadline_at
    });
    if cancellation_wins {
        let requested_at = cancellation.ok_or(LocalExecutionError::InvalidProviderPlan)?;
        let frames = plan
            .frames
            .iter()
            .filter(|frame| frame.emitted_at < *requested_at)
            .cloned()
            .collect::<Vec<_>>();
        let chunks = build_chunks(context, plan, &frames, false)?;
        return Ok(Some(build_incomplete(
            context,
            consumption.clone(),
            IncompleteExecutionStatusV1::Cancelled,
            None,
            chunks,
            requested_at.clone(),
        )?));
    }
    if plan.completed_at > context.admission.deadline_at {
        let frames = plan
            .frames
            .iter()
            .filter(|frame| frame.emitted_at <= context.admission.deadline_at)
            .cloned()
            .collect::<Vec<_>>();
        let chunks = build_chunks(context, plan, &frames, false)?;
        return Ok(Some(build_incomplete(
            context,
            consumption.clone(),
            IncompleteExecutionStatusV1::DeadlineExceeded,
            None,
            chunks,
            context.admission.deadline_at.clone(),
        )?));
    }
    Ok(None)
}

fn complete_execution<S>(
    context: &LocalExecutionContextV1<'_>,
    consumption: AuthenticatedReplayConsumptionV1,
    plan: ProviderExecutionPlanV1,
    signer: &S,
) -> Result<LocalExecutionOutcomeV1, LocalExecutionError>
where
    S: ProviderReceiptSignerV1,
{
    let chunks = build_chunks(context, &plan, &plan.frames, true)?;
    let output_digest = stream_digest(&chunks)?;
    let output_tokens = plan.frames.iter().try_fold(0_u64, |total, frame| {
        total.checked_add(frame.output_token_delta)
    });
    let receipt = build_provider_execution_receipt_v1(
        context.admission,
        &context.authenticated.routing,
        ProviderExecutionReceiptDraftV1 {
            started_at: plan.started_at,
            first_token_at: plan
                .frames
                .first()
                .ok_or(LocalExecutionError::InvalidProviderPlan)?
                .emitted_at
                .clone(),
            completed_at: plan.completed_at,
            input_tokens: plan.input_tokens,
            output_tokens: output_tokens.ok_or(LocalExecutionError::InvalidProviderPlan)?,
            output_digest,
            runtime_image_digest: plan.runtime_image_digest,
        },
    )?;
    let Ok(statement) = signer.sign(&receipt, context.admission) else {
        return Ok(LocalExecutionOutcomeV1::Incomplete(Box::new(
            build_incomplete(
                context,
                consumption,
                IncompleteExecutionStatusV1::ReceiptSigningFailed,
                None,
                chunks,
                receipt.completed_at,
            )?,
        )));
    };
    let signed_receipt = SignedProviderExecutionReceiptV1 {
        schema_version: "hyperintel.signed-provider-execution-receipt.v1".to_owned(),
        receipt,
        statement,
    };
    verify_signed_provider_execution_receipt_v1(
        &signed_receipt,
        context.admission,
        context.authenticated,
        context.verification_keys,
    )?;
    build_completed(context, consumption, chunks, signed_receipt)
}

fn build_completed(
    context: &LocalExecutionContextV1<'_>,
    consumption: AuthenticatedReplayConsumptionV1,
    chunks: Vec<ChatCompletionChunkV1>,
    signed_receipt: SignedProviderExecutionReceiptV1,
) -> Result<LocalExecutionOutcomeV1, LocalExecutionError> {
    let metering_receipt = build_metering_from_verified_execution_receipt_v1(
        &signed_receipt,
        context.admission,
        context.authenticated,
        context.verification_keys,
        None,
    )?;
    let evaluation =
        evaluate_reliability_artifact_verified_route_v1(&context.authenticated.routing)?;
    let RouteEvaluationOutcomeV1::Selected(route) = evaluation.outcome else {
        return Err(LocalExecutionError::ProviderBindingMismatch);
    };
    let routing = &context.authenticated.routing;
    let provider = routing
        .providers
        .get(&route.selected_provider_id)
        .ok_or(LocalExecutionError::ProviderBindingMismatch)?;
    let offer = routing
        .offers
        .iter()
        .map(|signed| &signed.offer)
        .find(|offer| offer.digest().ok() == Some(route.selected_offer_digest))
        .ok_or(LocalExecutionError::ProviderBindingMismatch)?;
    let settlement_intent = derive_settlement_intent_v1(
        routing.service,
        provider,
        offer,
        routing.policy,
        &route,
        &metering_receipt,
        routing.policy.valid_until.clone(),
    )?;
    let mut completed = CompletedLocalExecutionV1 {
        schema_version: "hyperintel.completed-local-execution.v1".to_owned(),
        state_slice: LOCAL_EXECUTION_STATE_SLICE_V1.to_owned(),
        replay_consumption: consumption,
        chunks,
        signed_receipt,
        metering_receipt,
        settlement_intent,
        authority_granted: false,
        completion_digest: Sha256Digest::from_bytes([0; 32]),
    };
    completed.completion_digest = completed_digest(&completed)?;
    Ok(LocalExecutionOutcomeV1::Completed(Box::new(completed)))
}

fn build_chunks(
    context: &LocalExecutionContextV1<'_>,
    plan: &ProviderExecutionPlanV1,
    frames: &[ProviderStreamFrameV1],
    completed: bool,
) -> Result<Vec<ChatCompletionChunkV1>, LocalExecutionError> {
    let created = u64::try_from(
        plan.started_at
            .to_datetime()
            .map_err(|_| LocalExecutionError::InvalidUnixTimestamp)?
            .unix_timestamp(),
    )
    .map_err(|_| LocalExecutionError::InvalidUnixTimestamp)?;
    let id = format!(
        "chatcmpl-{}",
        &context.admission.admission_digest.to_hex()[..24]
    );
    Ok(frames
        .iter()
        .enumerate()
        .map(|(index, frame)| ChatCompletionChunkV1 {
            id: id.clone(),
            object: CHAT_COMPLETION_CHUNK_OBJECT.to_owned(),
            created,
            model: context.request.model.clone(),
            choices: vec![ChatCompletionChunkChoiceV1 {
                index: 0,
                delta: ChatCompletionDeltaV1 {
                    role: (index == 0).then_some(ChatMessageRoleV1::Assistant),
                    content: Some(frame.content.clone()),
                },
                finish_reason: (completed && index + 1 == frames.len())
                    .then_some(ChatFinishReasonV1::Stop),
            }],
        })
        .collect())
}

fn build_incomplete(
    context: &LocalExecutionContextV1<'_>,
    consumption: AuthenticatedReplayConsumptionV1,
    status: IncompleteExecutionStatusV1,
    provider_error: Option<ProviderAgentErrorV1>,
    chunks: Vec<ChatCompletionChunkV1>,
    finished_at: UtcTimestamp,
) -> Result<IncompleteLocalExecutionV1, CanonicalizationError> {
    let stream_digest = stream_digest(&chunks)?;
    let mut attempt = IncompleteLocalExecutionV1 {
        schema_version: "hyperintel.incomplete-local-execution.v1".to_owned(),
        state_slice: LOCAL_EXECUTION_STATE_SLICE_V1.to_owned(),
        admission_digest: context.admission.admission_digest,
        provider_id: context.admission.provider_id.clone(),
        replay_consumption: consumption,
        status,
        provider_error,
        chunks,
        stream_digest,
        finished_at,
        authority_granted: false,
        attempt_digest: Sha256Digest::from_bytes([0; 32]),
    };
    attempt.attempt_digest = incomplete_digest(&attempt)?;
    Ok(attempt)
}

fn stream_digest(chunks: &[ChatCompletionChunkV1]) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest("hyperintel.openai-chat-stream.v1", &chunks)
}

fn incomplete_digest(
    attempt: &IncompleteLocalExecutionV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.incomplete-local-execution.v1",
        &(
            &attempt.schema_version,
            &attempt.state_slice,
            attempt.admission_digest,
            &attempt.provider_id,
            &attempt.replay_consumption,
            attempt.status,
            attempt.provider_error,
            &attempt.chunks,
            attempt.stream_digest,
            &attempt.finished_at,
            attempt.authority_granted,
        ),
    )
}

fn completed_digest(
    completed: &CompletedLocalExecutionV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.completed-local-execution.v1",
        &(
            &completed.schema_version,
            &completed.state_slice,
            &completed.replay_consumption,
            &completed.chunks,
            &completed.signed_receipt,
            &completed.metering_receipt,
            &completed.settlement_intent,
            completed.authority_granted,
        ),
    )
}
