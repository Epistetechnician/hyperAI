use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    AtomicAmount, BasisPoints, CanonicalizationError, PartsPerMillion, ProtocolId, Sha256Digest,
    UtcTimestamp, domain_separated_digest,
};

/// Evidence maturity. Ordering is deliberate and must remain conservative.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssuranceMaturity {
    /// Attributable declaration only.
    Declared,
    /// Checked locally against hermetic rules.
    LocallyChecked,
    /// Bound to an attestation trust root.
    Attested,
    /// A scoped property was machine checked.
    MachineChecked,
    /// The scoped evidence was independently reproduced.
    IndependentlyReproduced,
}

/// Non-compensable assurance properties.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssuranceProperty {
    /// Provider declares authorization to supply the service.
    ProviderAuthorization,
    /// Evidence binds the exact service manifest.
    ServiceManifestBinding,
    /// Evidence binds a measured runtime.
    RuntimeMeasurement,
    /// Evidence supports the declared execution region.
    RegionConfinement,
    /// Provider declares that prompt/response content is not retained.
    NoContentRetentionDeclared,
    /// Evidence supports the metering procedure.
    MeteringIntegrity,
    /// Evidence supports a scoped policy decision.
    PolicyCompliance,
}

/// Inference features that must be matched exactly or by inclusion.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InferenceCapability {
    /// Token streaming.
    Streaming,
    /// Tool or function calls.
    ToolCalls,
    /// Schema-constrained JSON output.
    StructuredOutput,
}

/// Plaintext exposure profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrivacyMode {
    /// Marketplace edge terminates TLS.
    Standard,
    /// Buyer establishes a direct authenticated provider channel.
    DirectSealed,
    /// Termination occurs inside an attested confidential boundary.
    AttestedConfidential,
}

/// Declared provider retention behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RetentionMode {
    /// Prompt and response content may be retained.
    ContentRetentionDeclared,
    /// Only operational metadata is declared retained.
    MetadataOnlyDeclared,
    /// No prompt or response content retention is declared.
    NoContentRetentionDeclared,
}

/// Usage measurement method.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MeteringMethod {
    /// Token counts are produced using the committed tokenizer.
    TokenizerDigest,
}

/// Provider lifecycle state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderStatus {
    /// Provider may be considered when all other gates pass.
    Active,
    /// Provider is temporarily ineligible.
    Suspended,
    /// Provider is permanently ineligible under this identity.
    Revoked,
}

/// Canonical inference service specification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceSpecV1 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Stable service identifier.
    pub service_id: ProtocolId,
    /// Model family.
    pub model_family: ProtocolId,
    /// Model release or checkpoint label.
    pub model_version: ProtocolId,
    /// Open-weight artifact or provider service commitment.
    pub model_artifact_digest: Sha256Digest,
    /// Tokenizer commitment.
    pub tokenizer_digest: Sha256Digest,
    /// Runtime image commitment.
    pub runtime_image_digest: Sha256Digest,
    /// Execution region.
    pub region: ProtocolId,
    /// Required privacy channel.
    pub privacy_mode: PrivacyMode,
    /// Declared content-retention behavior.
    pub retention_mode: RetentionMode,
    /// Supported inference features.
    pub capabilities: BTreeSet<InferenceCapability>,
    /// Maximum input tokens.
    pub max_input_tokens: u64,
    /// Maximum output tokens.
    pub max_output_tokens: u64,
    /// Usage measurement method.
    pub metering_method: MeteringMethod,
}

impl ServiceSpecV1 {
    /// Compute the canonical service-spec digest.
    ///
    /// # Errors
    ///
    /// Returns when canonicalization fails.
    pub fn digest(&self) -> Result<Sha256Digest, CanonicalizationError> {
        domain_separated_digest("hyperintel.service-spec.v1", self)
    }
}

/// Provider identity and declared authorization boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderIdentityV1 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Stable provider identifier.
    pub provider_id: ProtocolId,
    /// Current provider status.
    pub status: ProviderStatus,
    /// Identity activation time.
    pub valid_from: UtcTimestamp,
    /// Identity expiry time.
    pub valid_until: UtcTimestamp,
    /// Key identifier used for offers.
    pub offer_key_id: ProtocolId,
    /// Key identifier used for execution receipts.
    pub receipt_key_id: ProtocolId,
    /// Declared settlement payout account.
    pub payout_account: ProtocolId,
    /// Services this identity declares authorization to provide.
    pub authorized_service_digests: BTreeSet<Sha256Digest>,
}

impl ProviderIdentityV1 {
    /// Compute the canonical provider-identity digest.
    ///
    /// # Errors
    ///
    /// Returns when canonicalization fails.
    pub fn digest(&self) -> Result<Sha256Digest, CanonicalizationError> {
        domain_separated_digest("hyperintel.provider-identity.v1", self)
    }
}

/// Signed capacity and price declaration. Signature transport is separate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityOfferV1 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Stable offer identifier.
    pub offer_id: ProtocolId,
    /// Provider owning the offer.
    pub provider_id: ProtocolId,
    /// Exact service specification.
    pub service_spec_digest: Sha256Digest,
    /// Offer activation time.
    pub valid_from: UtcTimestamp,
    /// Offer expiry time.
    pub valid_until: UtcTimestamp,
    /// Available request units.
    pub available_request_units: u64,
    /// Maximum input tokens per request.
    pub max_input_tokens: u64,
    /// Maximum output tokens per request.
    pub max_output_tokens: u64,
    /// Fixed provider charge.
    pub fixed_request_fee: AtomicAmount,
    /// Provider price per one million input tokens.
    pub input_price_per_million: AtomicAmount,
    /// Provider price per one million output tokens.
    pub output_price_per_million: AtomicAmount,
    /// Declared/evidence-qualified success probability.
    pub success_probability: PartsPerMillion,
    /// Declared p95 completion latency.
    pub p95_latency_ms: u64,
    /// Settlement asset identifier.
    pub settlement_asset: ProtocolId,
    /// Execution region.
    pub region: ProtocolId,
    /// Reliability evidence reference.
    pub reliability_evidence_digest: Sha256Digest,
}

impl CapacityOfferV1 {
    /// Compute the canonical offer digest.
    ///
    /// # Errors
    ///
    /// Returns when canonicalization fails.
    pub fn digest(&self) -> Result<Sha256Digest, CanonicalizationError> {
        domain_separated_digest("hyperintel.capacity-offer.v1", self)
    }
}

/// Buyer-owned policy and economic authorization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InferenceRequestPolicyV1 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Stable request identifier.
    pub request_id: ProtocolId,
    /// Buyer identity.
    pub buyer_id: ProtocolId,
    /// Buyer refund account.
    pub buyer_refund_account: ProtocolId,
    /// Buyer-controlled replay nonce.
    pub nonce: u64,
    /// Policy activation time.
    pub valid_from: UtcTimestamp,
    /// Policy expiry time.
    pub valid_until: UtcTimestamp,
    /// Required service specification.
    pub required_service_spec_digest: Sha256Digest,
    /// Maximum input tokens authorized.
    pub max_input_tokens: u64,
    /// Maximum output tokens authorized.
    pub max_output_tokens: u64,
    /// Maximum debit authorized by the buyer.
    pub max_total_cost: AtomicAmount,
    /// Required settlement asset.
    pub settlement_asset: ProtocolId,
    /// Allowed regions.
    pub allowed_regions: BTreeSet<ProtocolId>,
    /// Optional provider allow-list. Empty means no positive restriction.
    pub allowed_providers: BTreeSet<ProtocolId>,
    /// Explicit provider deny-list.
    pub denied_providers: BTreeSet<ProtocolId>,
    /// Required inference features.
    pub required_capabilities: BTreeSet<InferenceCapability>,
    /// Minimum privacy mode.
    pub minimum_privacy_mode: PrivacyMode,
    /// Minimum assurance maturity.
    pub minimum_assurance_maturity: AssuranceMaturity,
    /// Required non-compensable assurance properties.
    pub required_assurance_properties: BTreeSet<AssuranceProperty>,
    /// Minimum accepted success probability.
    pub minimum_success_probability: PartsPerMillion,
    /// Maximum accepted p95 latency.
    pub max_p95_latency_ms: u64,
    /// Transparent platform fee.
    pub protocol_fee: BasisPoints,
    /// Platform fee recipient.
    pub protocol_fee_recipient: ProtocolId,
    /// Salted or keyed prompt commitment.
    pub prompt_digest: Sha256Digest,
    /// Replay-state commitment supplied by the buyer gate.
    pub replay_snapshot_digest: Sha256Digest,
}

impl InferenceRequestPolicyV1 {
    /// Compute the canonical request-policy digest.
    ///
    /// # Errors
    ///
    /// Returns when canonicalization fails.
    pub fn digest(&self) -> Result<Sha256Digest, CanonicalizationError> {
        domain_separated_digest("hyperintel.request-policy.v1", self)
    }
}
