use std::collections::{BTreeMap, BTreeSet};

use hyperintel_assurance::{
    AssuranceEvidenceError, AssuranceEvidenceSetV1, RegisteredVerificationKeyV1,
    VerificationKeyRegistry, verify_assurance_evidence_set_v1,
};
use hyperintel_core::{
    CanonicalizationError, CapacityOfferV1, InferenceRequestPolicyV1, ProtocolId,
    ProviderIdentityV1, ServiceSpecV1, Sha256Digest, UtcTimestamp, domain_separated_digest,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    MARKET_STATE_SLICE_V1, MeteringError, MeteringReceiptV1,
    ReliabilityArtifactVerifiedRoutingContextV1, ReplayError, ReplayStateTransitionV1,
    RouteDecisionV1, RouteEvaluationOutcomeV1, RouteEvaluationRecordV1, RoutingContextV1,
    RoutingError, SettlementError, SettlementIntentV1, SignedCapacityOfferV1,
    SignedReliabilityArtifactV1, SignedRoutingContextV1,
    evaluate_reliability_artifact_verified_route_v1, select_reliability_artifact_verified_route_v1,
    select_route_v1, select_signed_route_v1, validate_metering_receipt_v1,
    validate_route_evaluation_against_offers_v1, validate_settlement_intent_v1,
    verify_replay_state_transition_v1, verify_signed_reliability_artifact_v1,
};

const REQUIRED_NONCLAIMS_V1: [&str; 11] = [
    "no-inference-execution",
    "no-capacity-offer-signature-verification",
    "no-consumed-nonce-or-replay-state-enforcement",
    "no-reliability-evidence-verification",
    "no-provider-authentication-beyond-supplied-evidence",
    "no-model-or-output-correctness",
    "no-confidential-execution-claim",
    "no-settlement-or-value-movement",
    "no-hyperevm-contract-claim",
    "no-independent-reproduction",
    "no-production-readiness",
];

const REQUIRED_NONCLAIMS_V2: [&str; 10] = [
    "no-inference-execution",
    "no-consumed-nonce-or-replay-state-enforcement",
    "no-reliability-evidence-verification",
    "no-provider-authentication-beyond-supplied-evidence",
    "no-model-or-output-correctness",
    "no-confidential-execution-claim",
    "no-settlement-or-value-movement",
    "no-hyperevm-contract-claim",
    "no-independent-reproduction",
    "no-production-readiness",
];

const REQUIRED_NONCLAIMS_V3: [&str; 10] = [
    "no-inference-execution",
    "no-consumed-nonce-or-replay-state-enforcement",
    "no-observation-source-verification-or-future-reliability-accuracy",
    "no-provider-authentication-beyond-supplied-evidence",
    "no-model-or-output-correctness",
    "no-confidential-execution-claim",
    "no-settlement-or-value-movement",
    "no-hyperevm-contract-claim",
    "no-independent-reproduction",
    "no-production-readiness",
];

const REQUIRED_NONCLAIMS_V4: [&str; 10] = [
    "no-inference-execution",
    "no-buyer-policy-authentication-or-durable-atomic-replay-state-enforcement",
    "no-observation-source-verification-or-future-reliability-accuracy",
    "no-provider-authentication-beyond-supplied-evidence",
    "no-model-or-output-correctness",
    "no-confidential-execution-claim",
    "no-settlement-or-value-movement",
    "no-hyperevm-contract-claim",
    "no-independent-reproduction",
    "no-production-readiness",
];

/// Portable reconstruction manifest for one complete local market slice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketEvidenceBundleV1 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Named state slice.
    pub state_slice: String,
    /// Explicit reconstruction time.
    pub evaluated_at: UtcTimestamp,
    /// Service specification.
    pub service: ServiceSpecV1,
    /// Provider identities keyed in canonical order.
    pub providers: BTreeMap<ProtocolId, ProviderIdentityV1>,
    /// Candidate offers.
    pub offers: Vec<CapacityOfferV1>,
    /// Buyer policy.
    pub policy: InferenceRequestPolicyV1,
    /// Generic assurance sets keyed by their self-digests.
    pub assurance_sets: BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    /// Public verification keys required by the bundle.
    pub verification_keys: Vec<RegisteredVerificationKeyV1>,
    /// Derived route decision.
    pub route: RouteDecisionV1,
    /// Derived metering receipt.
    pub metering: MeteringReceiptV1,
    /// Derived settlement instruction.
    pub settlement: SettlementIntentV1,
    /// Explicit model and evidence assumptions.
    pub assumptions: Vec<String>,
    /// Required negative claim boundary.
    pub explicit_nonclaims: Vec<String>,
    /// Digest of every preceding field.
    pub bundle_digest: Sha256Digest,
}

/// Portable reconstruction manifest whose capacity offers are provider-authenticated.
///
/// V2 is additive. V1 remains decodable and verifiable under its explicit
/// `no-capacity-offer-signature-verification` compatibility boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedMarketEvidenceBundleV2 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Named state slice.
    pub state_slice: String,
    /// Explicit reconstruction time.
    pub evaluated_at: UtcTimestamp,
    /// Service specification.
    pub service: ServiceSpecV1,
    /// Provider identities keyed in canonical order.
    pub providers: BTreeMap<ProtocolId, ProviderIdentityV1>,
    /// Candidate offers authenticated by their configured provider offer keys.
    pub offers: Vec<SignedCapacityOfferV1>,
    /// Buyer policy.
    pub policy: InferenceRequestPolicyV1,
    /// Generic assurance sets keyed by their self-digests.
    pub assurance_sets: BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    /// Public verification keys required by the bundle.
    pub verification_keys: Vec<RegisteredVerificationKeyV1>,
    /// Derived route decision.
    pub route: RouteDecisionV1,
    /// Derived metering receipt.
    pub metering: MeteringReceiptV1,
    /// Derived settlement instruction.
    pub settlement: SettlementIntentV1,
    /// Explicit model and evidence assumptions.
    pub assumptions: Vec<String>,
    /// Required negative claim boundary.
    pub explicit_nonclaims: Vec<String>,
    /// Digest of every preceding field.
    pub bundle_digest: Sha256Digest,
}

/// Additive signed-offer bundle whose route verifies typed reliability artifacts.
///
/// V3 does not embed reliability artifacts. Each signed offer commits the required artifact
/// digest; the artifact objects and caller trust remain separate verifier inputs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReliabilityArtifactVerifiedMarketEvidenceBundleV3 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Named state slice.
    pub state_slice: String,
    /// Explicit reconstruction time.
    pub evaluated_at: UtcTimestamp,
    /// Service specification.
    pub service: ServiceSpecV1,
    /// Provider identities keyed in canonical order.
    pub providers: BTreeMap<ProtocolId, ProviderIdentityV1>,
    /// Candidate offers authenticated by their configured provider offer keys.
    pub offers: Vec<SignedCapacityOfferV1>,
    /// Buyer policy.
    pub policy: InferenceRequestPolicyV1,
    /// Generic assurance sets keyed by their self-digests.
    pub assurance_sets: BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    /// Public assurance, provider, and reliability verification keys required by the bundle.
    pub verification_keys: Vec<RegisteredVerificationKeyV1>,
    /// Derived reliability-checked route decision.
    pub route: RouteDecisionV1,
    /// Derived metering receipt.
    pub metering: MeteringReceiptV1,
    /// Derived settlement instruction.
    pub settlement: SettlementIntentV1,
    /// Explicit model and evidence assumptions.
    pub assumptions: Vec<String>,
    /// Required negative claim boundary.
    pub explicit_nonclaims: Vec<String>,
    /// Digest of every preceding field.
    pub bundle_digest: Sha256Digest,
}

/// Additive V4 bundle with full route reports and a pure-data buyer replay transition.
///
/// V4 can represent either a selected route with metering and settlement records or a
/// deterministic all-rejected evaluation without them. The replay transition advances only the
/// caller-supplied state embedded in the bundle and grants no durable or external authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayProtectedMarketEvidenceBundleV4 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Named state slice.
    pub state_slice: String,
    /// Explicit reconstruction time.
    pub evaluated_at: UtcTimestamp,
    /// Service specification.
    pub service: ServiceSpecV1,
    /// Provider identities keyed in canonical order.
    pub providers: BTreeMap<ProtocolId, ProviderIdentityV1>,
    /// Candidate offers authenticated by their configured provider offer keys.
    pub offers: Vec<SignedCapacityOfferV1>,
    /// Buyer policy.
    pub policy: InferenceRequestPolicyV1,
    /// Generic assurance sets keyed by their self-digests.
    pub assurance_sets: BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    /// Public assurance, provider, and reliability verification keys required by the bundle.
    pub verification_keys: Vec<RegisteredVerificationKeyV1>,
    /// Full reliability-checked selected-or-denied route evaluation.
    pub route_evaluation: RouteEvaluationRecordV1,
    /// Pure-data transition of the exact buyer replay snapshot committed by the policy.
    pub replay_transition: ReplayStateTransitionV1,
    /// Metering record, present exactly when a route was selected.
    pub metering: Option<MeteringReceiptV1>,
    /// Settlement intent, present exactly when a route was selected.
    pub settlement: Option<SettlementIntentV1>,
    /// Explicit model and evidence assumptions.
    pub assumptions: Vec<String>,
    /// Required negative claim boundary.
    pub explicit_nonclaims: Vec<String>,
    /// Digest of every preceding field.
    pub bundle_digest: Sha256Digest,
}

#[derive(Serialize)]
struct BundleIdentity<'a> {
    schema_version: &'a str,
    state_slice: &'a str,
    evaluated_at: &'a UtcTimestamp,
    service: &'a ServiceSpecV1,
    providers: &'a BTreeMap<ProtocolId, ProviderIdentityV1>,
    offers: &'a [CapacityOfferV1],
    policy: &'a InferenceRequestPolicyV1,
    assurance_sets: &'a BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    verification_keys: &'a [RegisteredVerificationKeyV1],
    route: &'a RouteDecisionV1,
    metering: &'a MeteringReceiptV1,
    settlement: &'a SettlementIntentV1,
    assumptions: &'a [String],
    explicit_nonclaims: &'a [String],
}

#[derive(Serialize)]
struct SignedBundleIdentity<'a> {
    schema_version: &'a str,
    state_slice: &'a str,
    evaluated_at: &'a UtcTimestamp,
    service: &'a ServiceSpecV1,
    providers: &'a BTreeMap<ProtocolId, ProviderIdentityV1>,
    offers: &'a [SignedCapacityOfferV1],
    policy: &'a InferenceRequestPolicyV1,
    assurance_sets: &'a BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    verification_keys: &'a [RegisteredVerificationKeyV1],
    route: &'a RouteDecisionV1,
    metering: &'a MeteringReceiptV1,
    settlement: &'a SettlementIntentV1,
    assumptions: &'a [String],
    explicit_nonclaims: &'a [String],
}

#[derive(Serialize)]
struct ReplayProtectedBundleIdentity<'a> {
    schema_version: &'a str,
    state_slice: &'a str,
    evaluated_at: &'a UtcTimestamp,
    service: &'a ServiceSpecV1,
    providers: &'a BTreeMap<ProtocolId, ProviderIdentityV1>,
    offers: &'a [SignedCapacityOfferV1],
    policy: &'a InferenceRequestPolicyV1,
    assurance_sets: &'a BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    verification_keys: &'a [RegisteredVerificationKeyV1],
    route_evaluation: &'a RouteEvaluationRecordV1,
    replay_transition: &'a ReplayStateTransitionV1,
    metering: &'a Option<MeteringReceiptV1>,
    settlement: &'a Option<SettlementIntentV1>,
    assumptions: &'a [String],
    explicit_nonclaims: &'a [String],
}

/// Market bundle construction and reconstruction failures.
#[derive(Debug, Error)]
pub enum MarketEvidenceError {
    /// Bundle schema or state slice is unsupported.
    #[error("unsupported market evidence bundle")]
    UnsupportedSchema,
    /// A required negative claim is absent.
    #[error("required market nonclaim is absent")]
    MissingRequiredNonclaim,
    /// A bundle-disclosed key is absent or different in the caller's registry.
    #[error("bundle verification key is not an exact caller-trusted key")]
    UntrustedVerificationKey,
    /// Assurance map key differs from the embedded self-digest.
    #[error("assurance evidence map key does not match its self-digest")]
    AssuranceMapKeyMismatch,
    /// Disclosed keys are not exactly the keys used by signed statements.
    #[error("bundle key disclosure does not exactly match statement-used keys")]
    KeyDisclosureMismatch,
    /// A disclosed assurance set is invalid.
    #[error("bundle assurance evidence is invalid: {0}")]
    Assurance(#[from] AssuranceEvidenceError),
    /// Route reconstruction failed.
    #[error("route reconstruction failed: {0}")]
    Routing(#[from] RoutingError),
    /// Route bytes differ from reconstruction.
    #[error("route reconstruction mismatch")]
    RouteMismatch,
    /// Full route evaluation differs from deterministic reconstruction.
    #[error("route evaluation reconstruction mismatch")]
    RouteEvaluationMismatch,
    /// Replay-state validation or reconstruction failed.
    #[error("replay-state reconstruction failed: {0}")]
    Replay(#[from] ReplayError),
    /// Selected and denied outcomes carry inconsistent metering or settlement records.
    #[error("route outcome does not match metering and settlement presence")]
    OutcomeArtifactMismatch,
    /// Metering reconstruction failed.
    #[error("metering reconstruction failed: {0}")]
    Metering(#[from] MeteringError),
    /// Settlement reconstruction failed.
    #[error("settlement reconstruction failed: {0}")]
    Settlement(#[from] SettlementError),
    /// Selected provider or offer is absent.
    #[error("bundle selected provider or offer is absent")]
    SelectedSubjectMissing,
    /// Bundle self-digest differs.
    #[error("market evidence bundle digest mismatch")]
    BundleDigestMismatch,
    /// Canonicalization failed.
    #[error("market evidence canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
}

/// Assemble one market evidence manifest after route, metering, and settlement.
///
/// # Errors
///
/// Returns if required negative claims are absent or identity cannot be built.
#[allow(clippy::too_many_arguments)]
pub fn build_market_evidence_bundle_v1(
    evaluated_at: UtcTimestamp,
    service: ServiceSpecV1,
    providers: BTreeMap<ProtocolId, ProviderIdentityV1>,
    mut offers: Vec<CapacityOfferV1>,
    policy: InferenceRequestPolicyV1,
    assurance_sets: BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    mut verification_keys: Vec<RegisteredVerificationKeyV1>,
    route: RouteDecisionV1,
    metering: MeteringReceiptV1,
    settlement: SettlementIntentV1,
    mut assumptions: Vec<String>,
) -> Result<MarketEvidenceBundleV1, MarketEvidenceError> {
    offers.sort_by_key(|offer| offer.digest().ok());
    verification_keys.sort_by_key(|key| (key.signer_id.clone(), key.key_id.clone()));
    assumptions.sort();
    assumptions.dedup();
    let mut bundle = MarketEvidenceBundleV1 {
        schema_version: "hyperintel.market-evidence-bundle.v1".to_owned(),
        state_slice: MARKET_STATE_SLICE_V1.to_owned(),
        evaluated_at,
        service,
        providers,
        offers,
        policy,
        assurance_sets,
        verification_keys,
        route,
        metering,
        settlement,
        assumptions,
        explicit_nonclaims: REQUIRED_NONCLAIMS_V1
            .iter()
            .map(ToString::to_string)
            .collect(),
        bundle_digest: Sha256Digest::from_bytes([0; 32]),
    };
    bundle.bundle_digest = bundle_digest(&bundle)?;
    Ok(bundle)
}

/// Reconstruct every derived record and the final bundle identity.
///
/// Content-addressed assurance blobs and trusted keys are caller-supplied so
/// restricted evidence need not be embedded and bundle-disclosed keys cannot
/// authorize themselves.
///
/// # Errors
///
/// Returns on any schema, nonclaim, key, assurance, route, metering,
/// settlement, selected-subject, or digest mismatch.
pub fn verify_market_evidence_bundle_v1(
    bundle: &MarketEvidenceBundleV1,
    assurance_blobs: &BTreeMap<Sha256Digest, Vec<u8>>,
    trusted_keys: &VerificationKeyRegistry,
) -> Result<(), MarketEvidenceError> {
    if bundle.schema_version != "hyperintel.market-evidence-bundle.v1"
        || bundle.state_slice != MARKET_STATE_SLICE_V1
    {
        return Err(MarketEvidenceError::UnsupportedSchema);
    }
    for required in REQUIRED_NONCLAIMS_V1 {
        if !bundle
            .explicit_nonclaims
            .iter()
            .any(|value| value == required)
        {
            return Err(MarketEvidenceError::MissingRequiredNonclaim);
        }
    }
    for key in &bundle.verification_keys {
        if !trusted_keys.contains_exact(key) {
            return Err(MarketEvidenceError::UntrustedVerificationKey);
        }
    }
    let disclosed_key_ids = bundle
        .verification_keys
        .iter()
        .map(|key| (key.signer_id.clone(), key.key_id.clone()))
        .collect::<BTreeSet<_>>();
    if disclosed_key_ids.len() != bundle.verification_keys.len() {
        return Err(MarketEvidenceError::KeyDisclosureMismatch);
    }
    let mut used_key_ids = BTreeSet::new();
    for (map_digest, set) in &bundle.assurance_sets {
        if *map_digest != set.evidence_set_digest {
            return Err(MarketEvidenceError::AssuranceMapKeyMismatch);
        }
        verify_assurance_evidence_set_v1(set, assurance_blobs, trusted_keys, &bundle.evaluated_at)?;
        used_key_ids.extend(
            set.signed_statements
                .iter()
                .map(|statement| (statement.signer_id.clone(), statement.key_id.clone())),
        );
    }
    if used_key_ids != disclosed_key_ids {
        return Err(MarketEvidenceError::KeyDisclosureMismatch);
    }
    let context = RoutingContextV1 {
        evaluated_at: &bundle.evaluated_at,
        service: &bundle.service,
        providers: &bundle.providers,
        offers: &bundle.offers,
        policy: &bundle.policy,
        assurance_sets: &bundle.assurance_sets,
        assurance_blobs,
        verification_keys: trusted_keys,
    };
    let rebuilt_route = select_route_v1(&context)?;
    if rebuilt_route != bundle.route {
        return Err(MarketEvidenceError::RouteMismatch);
    }
    let provider = bundle
        .providers
        .get(&bundle.route.selected_provider_id)
        .ok_or(MarketEvidenceError::SelectedSubjectMissing)?;
    let offer = bundle
        .offers
        .iter()
        .find(|offer| offer.digest().ok() == Some(bundle.route.selected_offer_digest))
        .ok_or(MarketEvidenceError::SelectedSubjectMissing)?;
    validate_metering_receipt_v1(
        &bundle.service,
        provider,
        offer,
        &bundle.policy,
        &bundle.route,
        &bundle.metering,
    )?;
    validate_settlement_intent_v1(
        &bundle.service,
        provider,
        offer,
        &bundle.policy,
        &bundle.route,
        &bundle.metering,
        &bundle.settlement,
    )?;
    if bundle_digest(bundle)? != bundle.bundle_digest {
        return Err(MarketEvidenceError::BundleDigestMismatch);
    }
    Ok(())
}

/// Assemble a signed-offer market evidence manifest without changing V1 compatibility.
///
/// # Errors
///
/// Returns if identity construction fails. Full signature and reconstruction checks are
/// performed by [`verify_signed_market_evidence_bundle_v2`].
#[allow(clippy::too_many_arguments)]
pub fn build_signed_market_evidence_bundle_v2(
    evaluated_at: UtcTimestamp,
    service: ServiceSpecV1,
    providers: BTreeMap<ProtocolId, ProviderIdentityV1>,
    mut offers: Vec<SignedCapacityOfferV1>,
    policy: InferenceRequestPolicyV1,
    assurance_sets: BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    mut verification_keys: Vec<RegisteredVerificationKeyV1>,
    route: RouteDecisionV1,
    metering: MeteringReceiptV1,
    settlement: SettlementIntentV1,
    mut assumptions: Vec<String>,
) -> Result<SignedMarketEvidenceBundleV2, MarketEvidenceError> {
    offers.sort_by_key(|signed_offer| signed_offer.offer.digest().ok());
    verification_keys.sort_by_key(|key| (key.signer_id.clone(), key.key_id.clone()));
    assumptions.sort();
    assumptions.dedup();
    let mut bundle = SignedMarketEvidenceBundleV2 {
        schema_version: "hyperintel.market-evidence-bundle.v2".to_owned(),
        state_slice: MARKET_STATE_SLICE_V1.to_owned(),
        evaluated_at,
        service,
        providers,
        offers,
        policy,
        assurance_sets,
        verification_keys,
        route,
        metering,
        settlement,
        assumptions,
        explicit_nonclaims: REQUIRED_NONCLAIMS_V2
            .iter()
            .map(ToString::to_string)
            .collect(),
        bundle_digest: Sha256Digest::from_bytes([0; 32]),
    };
    bundle.bundle_digest = signed_bundle_digest(&bundle)?;
    Ok(bundle)
}

/// Reconstruct signed-offer admission, routing, metering, settlement, and V2 identity.
///
/// Content-addressed assurance blobs and trusted keys remain separate caller inputs.
/// Bundle-disclosed keys cannot authorize themselves.
///
/// # Errors
///
/// Returns on schema, nonclaim, caller trust, assurance, signed-offer admission, route,
/// metering, settlement, selected-subject, or digest mismatch.
pub fn verify_signed_market_evidence_bundle_v2(
    bundle: &SignedMarketEvidenceBundleV2,
    assurance_blobs: &BTreeMap<Sha256Digest, Vec<u8>>,
    trusted_keys: &VerificationKeyRegistry,
) -> Result<(), MarketEvidenceError> {
    if bundle.schema_version != "hyperintel.market-evidence-bundle.v2"
        || bundle.state_slice != MARKET_STATE_SLICE_V1
    {
        return Err(MarketEvidenceError::UnsupportedSchema);
    }
    for required in REQUIRED_NONCLAIMS_V2 {
        if !bundle
            .explicit_nonclaims
            .iter()
            .any(|value| value == required)
        {
            return Err(MarketEvidenceError::MissingRequiredNonclaim);
        }
    }
    for key in &bundle.verification_keys {
        if !trusted_keys.contains_exact(key) {
            return Err(MarketEvidenceError::UntrustedVerificationKey);
        }
    }
    let disclosed_key_ids = bundle
        .verification_keys
        .iter()
        .map(|key| (key.signer_id.clone(), key.key_id.clone()))
        .collect::<BTreeSet<_>>();
    if disclosed_key_ids.len() != bundle.verification_keys.len() {
        return Err(MarketEvidenceError::KeyDisclosureMismatch);
    }
    // A successfully admitted offer must use the same provider offer key that
    // already authenticated its provider-authorization assurance set. Do not
    // promote attacker-claimed signer/key fields from rejected offer wrappers
    // into caller-trusted key disclosures.
    let mut used_key_ids = BTreeSet::new();
    for (map_digest, set) in &bundle.assurance_sets {
        if *map_digest != set.evidence_set_digest {
            return Err(MarketEvidenceError::AssuranceMapKeyMismatch);
        }
        verify_assurance_evidence_set_v1(set, assurance_blobs, trusted_keys, &bundle.evaluated_at)?;
        used_key_ids.extend(
            set.signed_statements
                .iter()
                .map(|statement| (statement.signer_id.clone(), statement.key_id.clone())),
        );
    }
    if used_key_ids != disclosed_key_ids {
        return Err(MarketEvidenceError::KeyDisclosureMismatch);
    }
    let context = SignedRoutingContextV1 {
        evaluated_at: &bundle.evaluated_at,
        service: &bundle.service,
        providers: &bundle.providers,
        offers: &bundle.offers,
        policy: &bundle.policy,
        assurance_sets: &bundle.assurance_sets,
        assurance_blobs,
        verification_keys: trusted_keys,
    };
    let rebuilt_route = select_signed_route_v1(&context)?;
    if rebuilt_route != bundle.route {
        return Err(MarketEvidenceError::RouteMismatch);
    }
    let provider = bundle
        .providers
        .get(&bundle.route.selected_provider_id)
        .ok_or(MarketEvidenceError::SelectedSubjectMissing)?;
    let offer = bundle
        .offers
        .iter()
        .map(|signed_offer| &signed_offer.offer)
        .find(|offer| offer.digest().ok() == Some(bundle.route.selected_offer_digest))
        .ok_or(MarketEvidenceError::SelectedSubjectMissing)?;
    validate_metering_receipt_v1(
        &bundle.service,
        provider,
        offer,
        &bundle.policy,
        &bundle.route,
        &bundle.metering,
    )?;
    validate_settlement_intent_v1(
        &bundle.service,
        provider,
        offer,
        &bundle.policy,
        &bundle.route,
        &bundle.metering,
        &bundle.settlement,
    )?;
    if signed_bundle_digest(bundle)? != bundle.bundle_digest {
        return Err(MarketEvidenceError::BundleDigestMismatch);
    }
    Ok(())
}

/// Assemble an additive V3 bundle for signed offers and typed reliability verification.
///
/// # Errors
///
/// Returns if canonical identity construction fails. Full trust, signature, artifact,
/// reconstruction, metering, and settlement checks are performed by
/// [`verify_reliability_artifact_verified_market_evidence_bundle_v3`].
#[allow(clippy::too_many_arguments)]
pub fn build_reliability_artifact_verified_market_evidence_bundle_v3(
    evaluated_at: UtcTimestamp,
    service: ServiceSpecV1,
    providers: BTreeMap<ProtocolId, ProviderIdentityV1>,
    mut offers: Vec<SignedCapacityOfferV1>,
    policy: InferenceRequestPolicyV1,
    assurance_sets: BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    mut verification_keys: Vec<RegisteredVerificationKeyV1>,
    route: RouteDecisionV1,
    metering: MeteringReceiptV1,
    settlement: SettlementIntentV1,
    mut assumptions: Vec<String>,
) -> Result<ReliabilityArtifactVerifiedMarketEvidenceBundleV3, MarketEvidenceError> {
    offers.sort_by_key(|signed_offer| signed_offer.offer.digest().ok());
    verification_keys.sort_by_key(|key| (key.signer_id.clone(), key.key_id.clone()));
    assumptions.sort();
    assumptions.dedup();
    let mut bundle = ReliabilityArtifactVerifiedMarketEvidenceBundleV3 {
        schema_version: "hyperintel.market-evidence-bundle.v3".to_owned(),
        state_slice: MARKET_STATE_SLICE_V1.to_owned(),
        evaluated_at,
        service,
        providers,
        offers,
        policy,
        assurance_sets,
        verification_keys,
        route,
        metering,
        settlement,
        assumptions,
        explicit_nonclaims: REQUIRED_NONCLAIMS_V3
            .iter()
            .map(ToString::to_string)
            .collect(),
        bundle_digest: Sha256Digest::from_bytes([0; 32]),
    };
    bundle.bundle_digest = reliability_artifact_verified_bundle_digest(&bundle)?;
    Ok(bundle)
}

/// Reconstruct signed-offer admission, typed reliability gates, routing, metering, settlement,
/// and V3 bundle identity.
///
/// Reliability artifacts, assurance bytes, and caller trust are separate inputs. Invalid
/// reliability evidence is candidate-scoped: it rejects that offer without poisoning a valid
/// fallback or promoting attacker-claimed keys into the disclosed trust set.
///
/// # Errors
///
/// Returns on schema, nonclaim, caller trust, assurance, route, metering, settlement,
/// selected-subject, or digest mismatch.
pub fn verify_reliability_artifact_verified_market_evidence_bundle_v3(
    bundle: &ReliabilityArtifactVerifiedMarketEvidenceBundleV3,
    assurance_blobs: &BTreeMap<Sha256Digest, Vec<u8>>,
    reliability_artifacts: &BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
    trusted_keys: &VerificationKeyRegistry,
) -> Result<(), MarketEvidenceError> {
    if bundle.schema_version != "hyperintel.market-evidence-bundle.v3"
        || bundle.state_slice != MARKET_STATE_SLICE_V1
    {
        return Err(MarketEvidenceError::UnsupportedSchema);
    }
    for required in REQUIRED_NONCLAIMS_V3 {
        if !bundle
            .explicit_nonclaims
            .iter()
            .any(|value| value == required)
        {
            return Err(MarketEvidenceError::MissingRequiredNonclaim);
        }
    }
    verify_v3_key_disclosure(bundle, assurance_blobs, reliability_artifacts, trusted_keys)?;
    let context = ReliabilityArtifactVerifiedRoutingContextV1 {
        evaluated_at: &bundle.evaluated_at,
        service: &bundle.service,
        providers: &bundle.providers,
        offers: &bundle.offers,
        policy: &bundle.policy,
        assurance_sets: &bundle.assurance_sets,
        assurance_blobs,
        reliability_artifacts,
        verification_keys: trusted_keys,
    };
    let rebuilt_route = select_reliability_artifact_verified_route_v1(&context)?;
    if rebuilt_route != bundle.route {
        return Err(MarketEvidenceError::RouteMismatch);
    }
    let provider = bundle
        .providers
        .get(&bundle.route.selected_provider_id)
        .ok_or(MarketEvidenceError::SelectedSubjectMissing)?;
    let offer = bundle
        .offers
        .iter()
        .map(|signed_offer| &signed_offer.offer)
        .find(|offer| offer.digest().ok() == Some(bundle.route.selected_offer_digest))
        .ok_or(MarketEvidenceError::SelectedSubjectMissing)?;
    validate_metering_receipt_v1(
        &bundle.service,
        provider,
        offer,
        &bundle.policy,
        &bundle.route,
        &bundle.metering,
    )?;
    validate_settlement_intent_v1(
        &bundle.service,
        provider,
        offer,
        &bundle.policy,
        &bundle.route,
        &bundle.metering,
        &bundle.settlement,
    )?;
    if reliability_artifact_verified_bundle_digest(bundle)? != bundle.bundle_digest {
        return Err(MarketEvidenceError::BundleDigestMismatch);
    }
    Ok(())
}

/// Assemble an additive V4 bundle with full route reports and a replay transition.
///
/// The builder validates all internal route, replay, metering, settlement, and context links.
/// Caller trust, assurance bytes, and reliability artifacts remain separate verifier inputs.
///
/// # Errors
///
/// Returns on canonicalization, route-evaluation, replay, outcome-artifact, metering, settlement,
/// selected-subject, or context-binding failure.
#[allow(clippy::too_many_arguments)]
pub fn build_replay_protected_market_evidence_bundle_v4(
    evaluated_at: UtcTimestamp,
    service: ServiceSpecV1,
    providers: BTreeMap<ProtocolId, ProviderIdentityV1>,
    mut offers: Vec<SignedCapacityOfferV1>,
    policy: InferenceRequestPolicyV1,
    assurance_sets: BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    mut verification_keys: Vec<RegisteredVerificationKeyV1>,
    route_evaluation: RouteEvaluationRecordV1,
    replay_transition: ReplayStateTransitionV1,
    metering: Option<MeteringReceiptV1>,
    settlement: Option<SettlementIntentV1>,
    mut assumptions: Vec<String>,
) -> Result<ReplayProtectedMarketEvidenceBundleV4, MarketEvidenceError> {
    offers.sort_by_key(|signed_offer| signed_offer.offer.digest().ok());
    verification_keys.sort_by_key(|key| (key.signer_id.clone(), key.key_id.clone()));
    assumptions.sort();
    assumptions.dedup();
    let mut bundle = ReplayProtectedMarketEvidenceBundleV4 {
        schema_version: "hyperintel.market-evidence-bundle.v4".to_owned(),
        state_slice: MARKET_STATE_SLICE_V1.to_owned(),
        evaluated_at,
        service,
        providers,
        offers,
        policy,
        assurance_sets,
        verification_keys,
        route_evaluation,
        replay_transition,
        metering,
        settlement,
        assumptions,
        explicit_nonclaims: REQUIRED_NONCLAIMS_V4
            .iter()
            .map(ToString::to_string)
            .collect(),
        bundle_digest: Sha256Digest::from_bytes([0; 32]),
    };
    validate_v4_internal_links(&bundle)?;
    bundle.bundle_digest = replay_protected_bundle_digest(&bundle)?;
    Ok(bundle)
}

/// Reconstruct V4 trust, routing, replay, optional metering and settlement, and identity.
///
/// A selected evaluation requires both metering and settlement records. An all-rejected
/// evaluation requires both to be absent. The replay transition is checked against the exact
/// caller-supplied before state embedded in the transition; no durable state is read or written.
///
/// # Errors
///
/// Returns on schema, nonclaim, caller trust, assurance, route evaluation, replay, optional
/// outcome artifact, selected-subject, or digest mismatch.
pub fn verify_replay_protected_market_evidence_bundle_v4(
    bundle: &ReplayProtectedMarketEvidenceBundleV4,
    assurance_blobs: &BTreeMap<Sha256Digest, Vec<u8>>,
    reliability_artifacts: &BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
    trusted_keys: &VerificationKeyRegistry,
) -> Result<(), MarketEvidenceError> {
    if bundle.schema_version != "hyperintel.market-evidence-bundle.v4"
        || bundle.state_slice != MARKET_STATE_SLICE_V1
    {
        return Err(MarketEvidenceError::UnsupportedSchema);
    }
    for required in REQUIRED_NONCLAIMS_V4 {
        if !bundle
            .explicit_nonclaims
            .iter()
            .any(|value| value == required)
        {
            return Err(MarketEvidenceError::MissingRequiredNonclaim);
        }
    }
    verify_reliability_key_disclosure(
        &bundle.evaluated_at,
        &bundle.verification_keys,
        &bundle.assurance_sets,
        &bundle.offers,
        assurance_blobs,
        reliability_artifacts,
        trusted_keys,
    )?;
    let context = ReliabilityArtifactVerifiedRoutingContextV1 {
        evaluated_at: &bundle.evaluated_at,
        service: &bundle.service,
        providers: &bundle.providers,
        offers: &bundle.offers,
        policy: &bundle.policy,
        assurance_sets: &bundle.assurance_sets,
        assurance_blobs,
        reliability_artifacts,
        verification_keys: trusted_keys,
    };
    let rebuilt_evaluation = evaluate_reliability_artifact_verified_route_v1(&context)?;
    if rebuilt_evaluation != bundle.route_evaluation {
        return Err(MarketEvidenceError::RouteEvaluationMismatch);
    }
    validate_v4_internal_links(bundle)?;
    if replay_protected_bundle_digest(bundle)? != bundle.bundle_digest {
        return Err(MarketEvidenceError::BundleDigestMismatch);
    }
    Ok(())
}

fn validate_v4_internal_links(
    bundle: &ReplayProtectedMarketEvidenceBundleV4,
) -> Result<(), MarketEvidenceError> {
    let offers = bundle
        .offers
        .iter()
        .map(|signed_offer| signed_offer.offer.clone())
        .collect::<Vec<_>>();
    validate_route_evaluation_against_offers_v1(&bundle.route_evaluation, &offers)?;
    if bundle.service.digest()? != bundle.route_evaluation.service_spec_digest
        || bundle.policy.digest()? != bundle.route_evaluation.request_policy_digest
        || bundle.policy.replay_snapshot_digest != bundle.route_evaluation.replay_snapshot_digest
        || bundle.evaluated_at != bundle.route_evaluation.evaluated_at
    {
        return Err(MarketEvidenceError::RouteEvaluationMismatch);
    }
    verify_replay_state_transition_v1(
        &bundle.replay_transition,
        &bundle.policy,
        bundle.route_evaluation.evaluation_digest,
        &bundle.evaluated_at,
    )?;

    match &bundle.route_evaluation.outcome {
        RouteEvaluationOutcomeV1::Selected(route) => {
            let (Some(metering), Some(settlement)) = (&bundle.metering, &bundle.settlement) else {
                return Err(MarketEvidenceError::OutcomeArtifactMismatch);
            };
            let provider = bundle
                .providers
                .get(&route.selected_provider_id)
                .ok_or(MarketEvidenceError::SelectedSubjectMissing)?;
            let offer = bundle
                .offers
                .iter()
                .map(|signed_offer| &signed_offer.offer)
                .find(|offer| offer.digest().ok() == Some(route.selected_offer_digest))
                .ok_or(MarketEvidenceError::SelectedSubjectMissing)?;
            validate_metering_receipt_v1(
                &bundle.service,
                provider,
                offer,
                &bundle.policy,
                route,
                metering,
            )?;
            validate_settlement_intent_v1(
                &bundle.service,
                provider,
                offer,
                &bundle.policy,
                route,
                metering,
                settlement,
            )?;
        }
        RouteEvaluationOutcomeV1::NoEligibleOffer => {
            if bundle.metering.is_some() || bundle.settlement.is_some() {
                return Err(MarketEvidenceError::OutcomeArtifactMismatch);
            }
        }
    }
    Ok(())
}

fn verify_v3_key_disclosure(
    bundle: &ReliabilityArtifactVerifiedMarketEvidenceBundleV3,
    assurance_blobs: &BTreeMap<Sha256Digest, Vec<u8>>,
    reliability_artifacts: &BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
    trusted_keys: &VerificationKeyRegistry,
) -> Result<(), MarketEvidenceError> {
    verify_reliability_key_disclosure(
        &bundle.evaluated_at,
        &bundle.verification_keys,
        &bundle.assurance_sets,
        &bundle.offers,
        assurance_blobs,
        reliability_artifacts,
        trusted_keys,
    )
}

fn verify_reliability_key_disclosure(
    evaluated_at: &UtcTimestamp,
    verification_keys: &[RegisteredVerificationKeyV1],
    assurance_sets: &BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    offers: &[SignedCapacityOfferV1],
    assurance_blobs: &BTreeMap<Sha256Digest, Vec<u8>>,
    reliability_artifacts: &BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
    trusted_keys: &VerificationKeyRegistry,
) -> Result<(), MarketEvidenceError> {
    for key in verification_keys {
        if !trusted_keys.contains_exact(key) {
            return Err(MarketEvidenceError::UntrustedVerificationKey);
        }
    }
    let disclosed_key_ids = verification_keys
        .iter()
        .map(|key| (key.signer_id.clone(), key.key_id.clone()))
        .collect::<BTreeSet<_>>();
    if disclosed_key_ids.len() != verification_keys.len() {
        return Err(MarketEvidenceError::KeyDisclosureMismatch);
    }
    let mut used_key_ids = BTreeSet::new();
    for (map_digest, set) in assurance_sets {
        if *map_digest != set.evidence_set_digest {
            return Err(MarketEvidenceError::AssuranceMapKeyMismatch);
        }
        verify_assurance_evidence_set_v1(set, assurance_blobs, trusted_keys, evaluated_at)?;
        used_key_ids.extend(
            set.signed_statements
                .iter()
                .map(|statement| (statement.signer_id.clone(), statement.key_id.clone())),
        );
    }
    for signed_offer in offers {
        let offer = &signed_offer.offer;
        if let Some(artifact) = reliability_artifacts.get(&offer.reliability_evidence_digest)
            && verify_signed_reliability_artifact_v1(
                artifact,
                offer.reliability_evidence_digest,
                offer,
                evaluated_at,
                trusted_keys,
            )
            .is_ok()
        {
            used_key_ids.insert((
                artifact.statement.signer_id.clone(),
                artifact.statement.key_id.clone(),
            ));
        }
    }
    if used_key_ids != disclosed_key_ids {
        return Err(MarketEvidenceError::KeyDisclosureMismatch);
    }
    Ok(())
}

fn bundle_digest(bundle: &MarketEvidenceBundleV1) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.market-evidence-bundle.v1",
        &BundleIdentity {
            schema_version: &bundle.schema_version,
            state_slice: &bundle.state_slice,
            evaluated_at: &bundle.evaluated_at,
            service: &bundle.service,
            providers: &bundle.providers,
            offers: &bundle.offers,
            policy: &bundle.policy,
            assurance_sets: &bundle.assurance_sets,
            verification_keys: &bundle.verification_keys,
            route: &bundle.route,
            metering: &bundle.metering,
            settlement: &bundle.settlement,
            assumptions: &bundle.assumptions,
            explicit_nonclaims: &bundle.explicit_nonclaims,
        },
    )
}

fn signed_bundle_digest(
    bundle: &SignedMarketEvidenceBundleV2,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.market-evidence-bundle.v2",
        &SignedBundleIdentity {
            schema_version: &bundle.schema_version,
            state_slice: &bundle.state_slice,
            evaluated_at: &bundle.evaluated_at,
            service: &bundle.service,
            providers: &bundle.providers,
            offers: &bundle.offers,
            policy: &bundle.policy,
            assurance_sets: &bundle.assurance_sets,
            verification_keys: &bundle.verification_keys,
            route: &bundle.route,
            metering: &bundle.metering,
            settlement: &bundle.settlement,
            assumptions: &bundle.assumptions,
            explicit_nonclaims: &bundle.explicit_nonclaims,
        },
    )
}

fn reliability_artifact_verified_bundle_digest(
    bundle: &ReliabilityArtifactVerifiedMarketEvidenceBundleV3,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.market-evidence-bundle.v3",
        &SignedBundleIdentity {
            schema_version: &bundle.schema_version,
            state_slice: &bundle.state_slice,
            evaluated_at: &bundle.evaluated_at,
            service: &bundle.service,
            providers: &bundle.providers,
            offers: &bundle.offers,
            policy: &bundle.policy,
            assurance_sets: &bundle.assurance_sets,
            verification_keys: &bundle.verification_keys,
            route: &bundle.route,
            metering: &bundle.metering,
            settlement: &bundle.settlement,
            assumptions: &bundle.assumptions,
            explicit_nonclaims: &bundle.explicit_nonclaims,
        },
    )
}

fn replay_protected_bundle_digest(
    bundle: &ReplayProtectedMarketEvidenceBundleV4,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.market-evidence-bundle.v4",
        &ReplayProtectedBundleIdentity {
            schema_version: &bundle.schema_version,
            state_slice: &bundle.state_slice,
            evaluated_at: &bundle.evaluated_at,
            service: &bundle.service,
            providers: &bundle.providers,
            offers: &bundle.offers,
            policy: &bundle.policy,
            assurance_sets: &bundle.assurance_sets,
            verification_keys: &bundle.verification_keys,
            route_evaluation: &bundle.route_evaluation,
            replay_transition: &bundle.replay_transition,
            metering: &bundle.metering,
            settlement: &bundle.settlement,
            assumptions: &bundle.assumptions,
            explicit_nonclaims: &bundle.explicit_nonclaims,
        },
    )
}
