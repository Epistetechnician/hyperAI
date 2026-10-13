use hyperintel_core::{
    AtomicAmount, CanonicalizationError, CapacityOfferV1, ExactArithmeticError,
    InferenceRequestPolicyV1, ProtocolId, ProviderIdentityV1, ServiceSpecV1, Sha256Digest,
    UtcTimestamp, domain_separated_digest,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    MARKET_STATE_SLICE_V1, MeteringError, MeteringReceiptV1, RouteDecisionV1,
    validate_metering_receipt_v1,
};

/// Pure-data instruction for a future settlement adapter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementIntentV1 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Named state slice.
    pub state_slice: String,
    /// Full deterministic intent identity.
    pub intent_id: String,
    /// Buyer request policy.
    pub request_policy_digest: Sha256Digest,
    /// Route decision.
    pub route_decision_digest: Sha256Digest,
    /// Metering receipt.
    pub metering_receipt_digest: Sha256Digest,
    /// Settlement asset.
    pub settlement_asset: ProtocolId,
    /// Buyer-authorized escrow amount.
    pub escrow_amount: AtomicAmount,
    /// Provider payout account.
    pub provider_payout_account: ProtocolId,
    /// Actual provider payout.
    pub provider_payout_amount: AtomicAmount,
    /// Platform fee recipient.
    pub protocol_fee_recipient: ProtocolId,
    /// Actual platform fee.
    pub protocol_fee_amount: AtomicAmount,
    /// Buyer refund account.
    pub buyer_refund_account: ProtocolId,
    /// Unused authorization returned to the buyer.
    pub buyer_refund_amount: AtomicAmount,
    /// Intent expiry.
    pub expires_at: UtcTimestamp,
    /// Full idempotency digest.
    pub idempotency_digest: Sha256Digest,
    /// Pure-data records grant no settlement authority.
    pub authority_granted: bool,
    /// Digest of every preceding semantic field.
    pub intent_digest: Sha256Digest,
}

#[derive(Serialize)]
struct SettlementIdentity<'a> {
    schema_version: &'a str,
    state_slice: &'a str,
    request_policy_digest: Sha256Digest,
    route_decision_digest: Sha256Digest,
    metering_receipt_digest: Sha256Digest,
    settlement_asset: &'a ProtocolId,
    escrow_amount: AtomicAmount,
    provider_payout_account: &'a ProtocolId,
    provider_payout_amount: AtomicAmount,
    protocol_fee_recipient: &'a ProtocolId,
    protocol_fee_amount: AtomicAmount,
    buyer_refund_account: &'a ProtocolId,
    buyer_refund_amount: AtomicAmount,
    expires_at: &'a UtcTimestamp,
    authority_granted: bool,
}

/// Settlement-intent derivation failures.
#[derive(Debug, Error)]
pub enum SettlementError {
    /// Input objects do not bind the same request and route.
    #[error("settlement binding mismatch")]
    BindingMismatch,
    /// Buyer authorization cannot cover payout and fee.
    #[error("buyer authorization is insufficient")]
    EscrowInsufficient,
    /// Conservation does not hold.
    #[error("settlement conservation failure")]
    ConservationFailure,
    /// Intent expiry is not after execution or exceeds buyer authorization.
    #[error("settlement expiry is outside the authorized window")]
    InvalidExpiry,
    /// Metering receipt does not reconstruct from its bound inputs.
    #[error("settlement metering receipt is invalid: {0}")]
    Metering(#[from] MeteringError),
    /// Exact arithmetic failed.
    #[error("settlement arithmetic failed: {0}")]
    Arithmetic(#[from] ExactArithmeticError),
    /// Canonicalization failed.
    #[error("settlement canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
}

/// Derive a deterministic, non-authoritative settlement instruction.
///
/// # Errors
///
/// Returns without an intent on binding, authorization, arithmetic, or
/// conservation failure.
pub fn derive_settlement_intent_v1(
    service: &ServiceSpecV1,
    provider: &ProviderIdentityV1,
    offer: &CapacityOfferV1,
    policy: &InferenceRequestPolicyV1,
    route: &RouteDecisionV1,
    metering: &MeteringReceiptV1,
    expires_at: UtcTimestamp,
) -> Result<SettlementIntentV1, SettlementError> {
    validate_metering_receipt_v1(service, provider, offer, policy, route, metering)?;
    let policy_digest = policy.digest()?;
    let provider_digest = provider.digest()?;
    let offer_digest = offer.digest()?;
    if route.request_policy_digest != policy_digest
        || route.selected_provider_identity_digest != provider_digest
        || route.selected_offer_digest != offer_digest
        || metering.request_policy_digest != policy_digest
        || metering.route_decision_digest != route.route_digest
        || metering.provider_identity_digest != provider_digest
        || metering.offer_digest != offer_digest
        || metering.provider_id != provider.provider_id
        || offer.provider_id != provider.provider_id
        || offer.settlement_asset != policy.settlement_asset
    {
        return Err(SettlementError::BindingMismatch);
    }
    if expires_at <= metering.completed_at || expires_at > policy.valid_until {
        return Err(SettlementError::InvalidExpiry);
    }
    let payout = metering.actual_provider_charge;
    let fee = payout.checked_ceil_mul_div(u64::from(policy.protocol_fee.get()), 10_000)?;
    let debit = payout.checked_add(fee)?;
    if debit > policy.max_total_cost || debit > route.selected_cost.buyer_quote_max {
        return Err(SettlementError::EscrowInsufficient);
    }
    let refund = policy.max_total_cost.checked_sub(debit)?;
    if payout.checked_add(fee)?.checked_add(refund)? != policy.max_total_cost {
        return Err(SettlementError::ConservationFailure);
    }
    let identity = SettlementIdentity {
        schema_version: "hyperintel.settlement-intent.v1",
        state_slice: MARKET_STATE_SLICE_V1,
        request_policy_digest: policy_digest,
        route_decision_digest: route.route_digest,
        metering_receipt_digest: metering.receipt_digest,
        settlement_asset: &policy.settlement_asset,
        escrow_amount: policy.max_total_cost,
        provider_payout_account: &provider.payout_account,
        provider_payout_amount: payout,
        protocol_fee_recipient: &policy.protocol_fee_recipient,
        protocol_fee_amount: fee,
        buyer_refund_account: &policy.buyer_refund_account,
        buyer_refund_amount: refund,
        expires_at: &expires_at,
        authority_granted: false,
    };
    let idempotency_digest =
        domain_separated_digest("hyperintel.settlement-idempotency.v1", &identity)?;
    let intent_digest = domain_separated_digest("hyperintel.settlement-intent.v1", &identity)?;
    Ok(SettlementIntentV1 {
        schema_version: "hyperintel.settlement-intent.v1".to_owned(),
        state_slice: MARKET_STATE_SLICE_V1.to_owned(),
        intent_id: format!("intent_{}", idempotency_digest.to_hex()),
        request_policy_digest: policy_digest,
        route_decision_digest: route.route_digest,
        metering_receipt_digest: metering.receipt_digest,
        settlement_asset: policy.settlement_asset.clone(),
        escrow_amount: policy.max_total_cost,
        provider_payout_account: provider.payout_account.clone(),
        provider_payout_amount: payout,
        protocol_fee_recipient: policy.protocol_fee_recipient.clone(),
        protocol_fee_amount: fee,
        buyer_refund_account: policy.buyer_refund_account.clone(),
        buyer_refund_amount: refund,
        expires_at,
        idempotency_digest,
        authority_granted: false,
        intent_digest,
    })
}

/// Reconstruct and compare a materialized settlement intent.
///
/// # Errors
///
/// Returns on any semantic or identity mismatch.
pub fn validate_settlement_intent_v1(
    service: &ServiceSpecV1,
    provider: &ProviderIdentityV1,
    offer: &CapacityOfferV1,
    policy: &InferenceRequestPolicyV1,
    route: &RouteDecisionV1,
    metering: &MeteringReceiptV1,
    intent: &SettlementIntentV1,
) -> Result<(), SettlementError> {
    let rebuilt = derive_settlement_intent_v1(
        service,
        provider,
        offer,
        policy,
        route,
        metering,
        intent.expires_at.clone(),
    )?;
    if &rebuilt == intent {
        Ok(())
    } else {
        Err(SettlementError::BindingMismatch)
    }
}
