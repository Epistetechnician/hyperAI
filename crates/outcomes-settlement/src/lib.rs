//! Backend-neutral settlement capabilities, intents, receipts, and reconciliation.

use std::collections::BTreeSet;

use outcomes_domain::{
    ClaimSpecV1, ExternalSideV1, MarketBindingV1, ResolutionRecordV1, SettlementHandoffV1,
    SettlementNetwork, Sha256Digest, UnsignedSettlementIntentV1, UtcTimestamp, canonical_json,
    domain_separated_digest,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Separately detectable backend capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackendCapability {
    /// Read active market metadata.
    ReadActiveMetadata,
    /// Read a retained settled outcome.
    ReadSettledOutcome,
    /// Submit a mock settlement effect.
    MockSubmission,
    /// Create or register an external market.
    CreateMarket,
    /// Publish an external resolution.
    PublishResolution,
}

/// Dated backend capability snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityMatrix {
    /// Backend identifier.
    pub backend: String,
    /// Network scope.
    pub network: SettlementNetwork,
    /// Capabilities supported by current primary sources or mock contract.
    pub supported: BTreeSet<BackendCapability>,
    /// Exact unavailable capability reasons.
    pub blockers: Vec<String>,
    /// Snapshot timestamp.
    pub verified_at: UtcTimestamp,
}

/// Normalized external market used only for explicit binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalMarket {
    /// Backend identifier.
    pub backend: String,
    /// Network-scoped identity.
    pub network: SettlementNetwork,
    /// External outcome/market ID.
    pub market_id: String,
    /// Optional external question ID.
    pub question_id: Option<String>,
    /// Ordered exact side specifications.
    pub sides: Vec<ExternalSideV1>,
    /// Per-outcome quote token when documented.
    pub quote_token: Option<String>,
    /// Digest of retained raw metadata bytes.
    pub metadata_digest: Sha256Digest,
    /// Raw payload content digest.
    pub raw_payload_digest: Sha256Digest,
    /// Expected external resolution authority, if known.
    pub settlement_authority: Option<String>,
    /// Whether external metadata says the outcome is already settled.
    pub settled: bool,
}

/// Successful settlement readiness preflight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementPreflight {
    /// Exact binding digest.
    pub binding_digest: Sha256Digest,
    /// Exact final resolution digest.
    pub resolution_digest: Sha256Digest,
    /// Whether an effect may be built for this backend.
    pub ready: bool,
    /// Exact non-readiness reasons.
    pub blockers: Vec<String>,
}

/// Backend-neutral settlement errors.
#[derive(Debug, Error)]
pub enum SettlementError {
    /// Network does not match the selected backend mode.
    #[error("wrong settlement network")]
    WrongNetwork,
    /// External side list or mapping is ambiguous.
    #[error("invalid or ambiguous external side mapping")]
    InvalidSideMapping,
    /// Claim and market/binding digests do not match.
    #[error("claim or binding digest mismatch")]
    DigestMismatch,
    /// External resolution authority is absent.
    #[error("external settlement authority is undocumented or absent")]
    MissingAuthority,
    /// Market is already settled.
    #[error("external market is already settled")]
    AlreadySettled,
    /// Final immutable resolution is required.
    #[error("settlement requires a final resolution")]
    NotFinal,
    /// Backend capability is absent.
    #[error("backend capability is unsupported: {0}")]
    UnsupportedCapability(String),
    /// Mainnet mutation is structurally disabled.
    #[error("mainnet mutation is structurally disabled")]
    MainnetMutationDisabled,
    /// Binding metadata is stale.
    #[error("market binding metadata is stale")]
    StaleBinding,
    /// Payload cannot be canonicalized.
    #[error("settlement payload is not canonicalizable: {0}")]
    Canonicalization(#[from] outcomes_domain::CanonicalizationError),
    /// JSON encoding failed.
    #[error("settlement JSON encoding failed: {0}")]
    Json(#[from] serde_json::Error),
}

/// Validate and build an explicit digest-bound market binding.
///
/// # Errors
///
/// Returns network, side, authority, settled-market, claim, or canonicalization
/// failures. Display names are never used as the binding key.
#[allow(clippy::too_many_arguments)]
pub fn bind_market(
    claim: &ClaimSpecV1,
    market: &ExternalMarket,
    true_side_index: u8,
    false_side_index: u8,
    capability_snapshot_digest: Sha256Digest,
    created_by: &str,
    created_at: UtcTimestamp,
    binding_signature: String,
) -> Result<MarketBindingV1, SettlementError> {
    if true_side_index == false_side_index
        || !market
            .sides
            .iter()
            .any(|side| side.side_index == true_side_index)
        || !market
            .sides
            .iter()
            .any(|side| side.side_index == false_side_index)
    {
        return Err(SettlementError::InvalidSideMapping);
    }
    if market.settled {
        return Err(SettlementError::AlreadySettled);
    }
    if market.settlement_authority.is_none() {
        return Err(SettlementError::MissingAuthority);
    }
    let claim_digest = claim
        .claim_spec_digest()
        .map_err(|_| SettlementError::DigestMismatch)?;
    let identity = (
        &market.backend,
        market.network,
        &market.market_id,
        &market.question_id,
        market.metadata_digest,
        &claim.claim_id,
        claim_digest,
        true_side_index,
        false_side_index,
        capability_snapshot_digest,
    );
    let binding_digest = domain_separated_digest("hsai.market-binding.v1", &identity)?;
    Ok(MarketBindingV1 {
        schema_version: "hsai.market-binding.v1".to_owned(),
        backend: market.backend.clone(),
        network: market.network,
        external_market_id: market.market_id.clone(),
        external_question_id: market.question_id.clone(),
        sides: market.sides.clone(),
        true_side_index,
        false_side_index,
        true_settlement_fraction: claim.outcome_mapping.true_settlement_fraction.clone(),
        false_settlement_fraction: claim.outcome_mapping.false_settlement_fraction.clone(),
        quote_token: market.quote_token.clone(),
        external_metadata_digest: market.metadata_digest,
        raw_payload_digest: market.raw_payload_digest,
        claim_id: claim.claim_id.clone(),
        claim_spec_digest: claim_digest,
        external_claim_commitment: None,
        expected_settlement_authority: market.settlement_authority.clone(),
        capability_snapshot_digest,
        capability_version: "hsai.backend-capability.v1".to_owned(),
        created_by: created_by.to_owned(),
        created_at,
        binding_signature,
        binding_digest,
        invalidation_reason: None,
    })
}

/// Check finality, exact binding, metadata freshness, and publish capability.
#[must_use]
pub fn preflight(
    binding: &MarketBindingV1,
    resolution: &ResolutionRecordV1,
    current_metadata_digest: Sha256Digest,
    capabilities: &CapabilityMatrix,
) -> SettlementPreflight {
    let mut blockers = Vec::new();
    if resolution.finality != outcomes_domain::ResolutionFinality::Final {
        blockers.push("FINAL_RESOLUTION_REQUIRED".to_owned());
    }
    if resolution.claim_id != binding.claim_id {
        blockers.push("CLAIM_BINDING_MISMATCH".to_owned());
    }
    if binding.external_metadata_digest != current_metadata_digest {
        blockers.push("STALE_BINDING".to_owned());
    }
    if binding.invalidation_reason.is_some() {
        blockers.push("BINDING_INVALIDATED".to_owned());
    }
    if !capabilities
        .supported
        .contains(&BackendCapability::PublishResolution)
        && !capabilities
            .supported
            .contains(&BackendCapability::MockSubmission)
    {
        blockers.push("PUBLISH_RESOLUTION_UNSUPPORTED".to_owned());
    }
    SettlementPreflight {
        binding_digest: binding.binding_digest,
        resolution_digest: resolution.record_digest,
        ready: blockers.is_empty(),
        blockers,
    }
}

/// Build a deterministic unsigned intent after successful preflight.
///
/// # Errors
///
/// Returns not-final, mainnet, unsupported-preflight, or canonicalization errors.
pub fn build_intent(
    binding: &MarketBindingV1,
    resolution: &ResolutionRecordV1,
    preflight: &SettlementPreflight,
    expires_at: UtcTimestamp,
) -> Result<UnsignedSettlementIntentV1, SettlementError> {
    if binding.network == SettlementNetwork::HyperliquidMainnetReadonly {
        return Err(SettlementError::MainnetMutationDisabled);
    }
    if resolution.finality != outcomes_domain::ResolutionFinality::Final {
        return Err(SettlementError::NotFinal);
    }
    if !preflight.ready {
        return Err(SettlementError::UnsupportedCapability(
            preflight.blockers.join(","),
        ));
    }
    let payload_value = serde_json::json!({
        "backend": binding.backend,
        "network": binding.network,
        "external_market_id": binding.external_market_id,
        "binding_digest": binding.binding_digest,
        "final_resolution_digest": resolution.record_digest,
        "truth": resolution.truth,
        "true_settlement_fraction": binding.true_settlement_fraction,
        "false_settlement_fraction": binding.false_settlement_fraction,
    });
    let canonical_payload = String::from_utf8(canonical_json(&payload_value)?)
        .map_err(|_| SettlementError::DigestMismatch)?;
    let payload_digest =
        domain_separated_digest("hsai.unsigned-settlement-payload.v1", &payload_value)?;
    let idempotency_digest = domain_separated_digest(
        "hsai.settlement-idempotency.v1",
        &(
            &binding.backend,
            binding.network,
            binding.binding_digest,
            resolution.record_digest,
            "ATTESTED_RESOLUTION",
            payload_digest,
        ),
    )?;
    Ok(UnsignedSettlementIntentV1 {
        schema_version: "hsai.unsigned-settlement-intent.v1".to_owned(),
        intent_id: format!("intent_{}", &idempotency_digest.to_hex()[..32]),
        backend: binding.backend.clone(),
        network: binding.network,
        binding_digest: binding.binding_digest,
        final_resolution_digest: resolution.record_digest,
        action_type: "ATTESTED_RESOLUTION".to_owned(),
        canonical_payload,
        payload_digest,
        idempotency_key: format!("settlement_{}", idempotency_digest.to_hex()),
        expires_at,
    })
}

/// Build a signed manual handoff record from an unsigned intent.
///
/// The caller supplies a signature produced through the HSAI signer interface;
/// this function never receives private key bytes.
///
/// # Errors
///
/// Returns a canonicalization error if the mapping cannot be committed.
#[allow(clippy::too_many_arguments)]
pub fn build_manual_handoff(
    binding: &MarketBindingV1,
    resolution: &ResolutionRecordV1,
    intent: &UnsignedSettlementIntentV1,
    evidence_bundle_uri: String,
    evidence_bundle_digest: Sha256Digest,
    required_authority_role: String,
    hsai_handoff_signature: String,
) -> Result<SettlementHandoffV1, SettlementError> {
    Ok(SettlementHandoffV1 {
        schema_version: "hsai.settlement-handoff.v1".to_owned(),
        backend: binding.backend.clone(),
        network: binding.network,
        binding_digest: binding.binding_digest,
        resolution_digest: resolution.record_digest,
        expected_settlement_mapping: [
            ("TRUE".to_owned(), binding.true_settlement_fraction.clone()),
            (
                "FALSE".to_owned(),
                binding.false_settlement_fraction.clone(),
            ),
        ]
        .into_iter()
        .collect(),
        unsigned_intent_digest: domain_separated_digest(
            "hsai.unsigned-settlement-intent.v1",
            intent,
        )?,
        evidence_bundle_uri,
        evidence_bundle_digest,
        required_authority_role,
        operator_preflight_checklist: vec![
            "verify final HSAI evidence bundle offline".to_owned(),
            "re-read network-scoped external metadata".to_owned(),
            "confirm authorized external resolution role".to_owned(),
            "confirm intent expiry and replay protection".to_owned(),
        ],
        expires_at: intent.expires_at.clone(),
        replay_protection: intent.idempotency_key.clone(),
        hsai_handoff_signature,
    })
}
