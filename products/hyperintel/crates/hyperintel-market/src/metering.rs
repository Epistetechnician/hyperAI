use hyperintel_core::{
    AtomicAmount, CanonicalizationError, CapacityOfferV1, ExactArithmeticError,
    InferenceRequestPolicyV1, MeteringMethod, ProtocolId, ProviderIdentityV1, ServiceSpecV1,
    Sha256Digest, UtcTimestamp, domain_separated_digest,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{MARKET_STATE_SLICE_V1, RouteDecisionV1};

/// V1 supports only completed, single-attempt inference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionStatusV1 {
    /// Provider reports one completed attempt.
    Completed,
    /// Explicitly unsupported until failure economics are specified.
    Failed,
    /// Explicitly unsupported until partial-output economics are specified.
    Partial,
    /// Explicitly unsupported until retry receipt graphs are specified.
    Retried,
}

/// Caller-supplied observation before identity derivation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeteringReceiptDraftV1 {
    /// Execution start.
    pub started_at: UtcTimestamp,
    /// First token time.
    pub first_token_at: UtcTimestamp,
    /// Completion time.
    pub completed_at: UtcTimestamp,
    /// Actual input tokens.
    pub input_tokens: u64,
    /// Actual output tokens.
    pub output_tokens: u64,
    /// Execution status.
    pub execution_status: ExecutionStatusV1,
    /// Prompt commitment observed by the meter.
    pub prompt_digest: Sha256Digest,
    /// Output-stream commitment.
    pub output_digest: Sha256Digest,
    /// Provider metering evidence commitment.
    pub provider_metering_evidence_digest: Sha256Digest,
    /// Optional buyer observation commitment.
    pub buyer_observation_digest: Option<Sha256Digest>,
}

/// Validated metering receipt and actual provider charge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeteringReceiptV1 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Named state slice.
    pub state_slice: String,
    /// Buyer request policy.
    pub request_policy_digest: Sha256Digest,
    /// Route decision.
    pub route_decision_digest: Sha256Digest,
    /// Service specification.
    pub service_spec_digest: Sha256Digest,
    /// Provider identity.
    pub provider_identity_digest: Sha256Digest,
    /// Selected offer.
    pub offer_digest: Sha256Digest,
    /// Provider identifier.
    pub provider_id: ProtocolId,
    /// Start timestamp.
    pub started_at: UtcTimestamp,
    /// First token timestamp.
    pub first_token_at: UtcTimestamp,
    /// Completion timestamp.
    pub completed_at: UtcTimestamp,
    /// Actual input tokens.
    pub input_tokens: u64,
    /// Actual output tokens.
    pub output_tokens: u64,
    /// Execution status.
    pub execution_status: ExecutionStatusV1,
    /// Metering method bound by the service.
    pub metering_method: MeteringMethod,
    /// Prompt commitment.
    pub prompt_digest: Sha256Digest,
    /// Output-stream commitment.
    pub output_digest: Sha256Digest,
    /// Provider evidence commitment.
    pub provider_metering_evidence_digest: Sha256Digest,
    /// Optional buyer observation.
    pub buyer_observation_digest: Option<Sha256Digest>,
    /// Actual provider charge.
    pub actual_provider_charge: AtomicAmount,
    /// Pure-data records grant no execution authority.
    pub authority_granted: bool,
    /// Digest of every preceding field.
    pub receipt_digest: Sha256Digest,
}

#[derive(Serialize)]
struct MeteringIdentity<'a> {
    schema_version: &'a str,
    state_slice: &'a str,
    request_policy_digest: Sha256Digest,
    route_decision_digest: Sha256Digest,
    service_spec_digest: Sha256Digest,
    provider_identity_digest: Sha256Digest,
    offer_digest: Sha256Digest,
    provider_id: &'a ProtocolId,
    started_at: &'a UtcTimestamp,
    first_token_at: &'a UtcTimestamp,
    completed_at: &'a UtcTimestamp,
    input_tokens: u64,
    output_tokens: u64,
    execution_status: ExecutionStatusV1,
    metering_method: &'a MeteringMethod,
    prompt_digest: Sha256Digest,
    output_digest: Sha256Digest,
    provider_metering_evidence_digest: Sha256Digest,
    buyer_observation_digest: Option<Sha256Digest>,
    actual_provider_charge: AtomicAmount,
    authority_granted: bool,
}

/// Metering validation failures.
#[derive(Debug, Error)]
pub enum MeteringError {
    /// V1 accepts only completed single-attempt execution.
    #[error("execution status is unsupported in v1")]
    UnsupportedExecutionStatus,
    /// Timestamp order is invalid.
    #[error("metering timestamps are not ordered")]
    TimestampOrderInvalid,
    /// Execution falls outside route, policy, offer, or provider authorization.
    #[error("execution is outside its authorization window")]
    ExecutionOutsideAuthorization,
    /// Prompt commitment differs from buyer policy.
    #[error("prompt digest mismatch")]
    PromptDigestMismatch,
    /// Output or metering evidence is absent.
    #[error("required metering evidence digest is missing")]
    MissingEvidence,
    /// Route, service, provider, or offer binding differs.
    #[error("metering binding mismatch")]
    BindingMismatch,
    /// Actual usage exceeds an authorized bound.
    #[error("metered token count exceeds an authorized maximum")]
    TokenLimitExceeded,
    /// Actual provider charge exceeds the routed maximum.
    #[error("actual charge exceeds routed quote")]
    ChargeExceedsQuote,
    /// Exact arithmetic failed.
    #[error("metering arithmetic failed: {0}")]
    Arithmetic(#[from] ExactArithmeticError),
    /// Canonicalization failed.
    #[error("metering canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
}

/// Build and validate a metering receipt for one completed attempt.
///
/// # Errors
///
/// Returns without a receipt on any status, binding, time, token, evidence,
/// arithmetic, or quote failure.
pub fn build_metering_receipt_v1(
    service: &ServiceSpecV1,
    provider: &ProviderIdentityV1,
    offer: &CapacityOfferV1,
    policy: &InferenceRequestPolicyV1,
    route: &RouteDecisionV1,
    draft: MeteringReceiptDraftV1,
) -> Result<MeteringReceiptV1, MeteringError> {
    if draft.execution_status != ExecutionStatusV1::Completed {
        return Err(MeteringError::UnsupportedExecutionStatus);
    }
    if draft.started_at > draft.first_token_at || draft.first_token_at > draft.completed_at {
        return Err(MeteringError::TimestampOrderInvalid);
    }
    if draft.started_at < route.evaluated_at
        || draft.started_at < policy.valid_from
        || draft.started_at < offer.valid_from
        || draft.started_at < provider.valid_from
        || draft.completed_at > policy.valid_until
        || draft.completed_at > offer.valid_until
        || draft.completed_at > provider.valid_until
    {
        return Err(MeteringError::ExecutionOutsideAuthorization);
    }
    if draft.prompt_digest != policy.prompt_digest {
        return Err(MeteringError::PromptDigestMismatch);
    }
    if draft.output_digest.is_zero() || draft.provider_metering_evidence_digest.is_zero() {
        return Err(MeteringError::MissingEvidence);
    }
    let service_digest = service.digest()?;
    let provider_digest = provider.digest()?;
    let offer_digest = offer.digest()?;
    let policy_digest = policy.digest()?;
    if route.request_policy_digest != policy_digest
        || route.service_spec_digest != service_digest
        || route.selected_provider_identity_digest != provider_digest
        || route.selected_offer_digest != offer_digest
        || route.selected_provider_id != provider.provider_id
        || offer.provider_id != provider.provider_id
    {
        return Err(MeteringError::BindingMismatch);
    }
    if draft.input_tokens > policy.max_input_tokens
        || draft.input_tokens > service.max_input_tokens
        || draft.input_tokens > offer.max_input_tokens
        || draft.output_tokens > policy.max_output_tokens
        || draft.output_tokens > service.max_output_tokens
        || draft.output_tokens > offer.max_output_tokens
    {
        return Err(MeteringError::TokenLimitExceeded);
    }
    let actual_provider_charge = actual_provider_charge(offer, &draft)?;
    if actual_provider_charge > route.selected_cost.provider_quote_max {
        return Err(MeteringError::ChargeExceedsQuote);
    }
    let mut receipt = MeteringReceiptV1 {
        schema_version: "hyperintel.metering-receipt.v1".to_owned(),
        state_slice: MARKET_STATE_SLICE_V1.to_owned(),
        request_policy_digest: policy_digest,
        route_decision_digest: route.route_digest,
        service_spec_digest: service_digest,
        provider_identity_digest: provider_digest,
        offer_digest,
        provider_id: provider.provider_id.clone(),
        started_at: draft.started_at,
        first_token_at: draft.first_token_at,
        completed_at: draft.completed_at,
        input_tokens: draft.input_tokens,
        output_tokens: draft.output_tokens,
        execution_status: draft.execution_status,
        metering_method: service.metering_method.clone(),
        prompt_digest: draft.prompt_digest,
        output_digest: draft.output_digest,
        provider_metering_evidence_digest: draft.provider_metering_evidence_digest,
        buyer_observation_digest: draft.buyer_observation_digest,
        actual_provider_charge,
        authority_granted: false,
        receipt_digest: Sha256Digest::from_bytes([0; 32]),
    };
    receipt.receipt_digest = metering_digest(&receipt)?;
    Ok(receipt)
}

/// Revalidate an already materialized receipt by rebuilding it.
///
/// # Errors
///
/// Returns on any semantic or identity mismatch.
pub fn validate_metering_receipt_v1(
    service: &ServiceSpecV1,
    provider: &ProviderIdentityV1,
    offer: &CapacityOfferV1,
    policy: &InferenceRequestPolicyV1,
    route: &RouteDecisionV1,
    receipt: &MeteringReceiptV1,
) -> Result<(), MeteringError> {
    let rebuilt = build_metering_receipt_v1(
        service,
        provider,
        offer,
        policy,
        route,
        MeteringReceiptDraftV1 {
            started_at: receipt.started_at.clone(),
            first_token_at: receipt.first_token_at.clone(),
            completed_at: receipt.completed_at.clone(),
            input_tokens: receipt.input_tokens,
            output_tokens: receipt.output_tokens,
            execution_status: receipt.execution_status,
            prompt_digest: receipt.prompt_digest,
            output_digest: receipt.output_digest,
            provider_metering_evidence_digest: receipt.provider_metering_evidence_digest,
            buyer_observation_digest: receipt.buyer_observation_digest,
        },
    )?;
    if &rebuilt == receipt {
        Ok(())
    } else {
        Err(MeteringError::BindingMismatch)
    }
}

fn actual_provider_charge(
    offer: &CapacityOfferV1,
    draft: &MeteringReceiptDraftV1,
) -> Result<AtomicAmount, ExactArithmeticError> {
    let input = offer
        .input_price_per_million
        .checked_ceil_mul_div(draft.input_tokens, 1_000_000)?;
    let output = offer
        .output_price_per_million
        .checked_ceil_mul_div(draft.output_tokens, 1_000_000)?;
    offer
        .fixed_request_fee
        .checked_add(input)?
        .checked_add(output)
}

fn metering_digest(receipt: &MeteringReceiptV1) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.metering-receipt.v1",
        &MeteringIdentity {
            schema_version: &receipt.schema_version,
            state_slice: &receipt.state_slice,
            request_policy_digest: receipt.request_policy_digest,
            route_decision_digest: receipt.route_decision_digest,
            service_spec_digest: receipt.service_spec_digest,
            provider_identity_digest: receipt.provider_identity_digest,
            offer_digest: receipt.offer_digest,
            provider_id: &receipt.provider_id,
            started_at: &receipt.started_at,
            first_token_at: &receipt.first_token_at,
            completed_at: &receipt.completed_at,
            input_tokens: receipt.input_tokens,
            output_tokens: receipt.output_tokens,
            execution_status: receipt.execution_status,
            metering_method: &receipt.metering_method,
            prompt_digest: receipt.prompt_digest,
            output_digest: receipt.output_digest,
            provider_metering_evidence_digest: receipt.provider_metering_evidence_digest,
            buyer_observation_digest: receipt.buyer_observation_digest,
            actual_provider_charge: receipt.actual_provider_charge,
            authority_granted: receipt.authority_granted,
        },
    )
}
