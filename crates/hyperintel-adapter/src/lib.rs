//! Offline `HyperIntel` evidence import for the `HyperAI` interface.
//!
//! Verification delegates to the source-owned `HyperIntel` contracts. This
//! adapter does not route or execute inference, resolve keys, or persist input.

use std::collections::BTreeMap;

use hyperai_foundation::parse_json_without_duplicate_keys;
use hyperintel_assurance::{
    AssuranceEvidenceSetV1, RegisteredVerificationKeyV1, VerificationKeyRegistry,
};
use hyperintel_core::{
    InferenceRequestPolicyV1, ProtocolId, ProviderIdentityV1, ServiceSpecV1, Sha256Digest,
    UtcTimestamp,
};
use hyperintel_execution::LocalExecutionOutcomeV1;
use hyperintel_execution_journal::{ExecutionJournalStartV1, ExecutionJournalTerminalV1};
use hyperintel_market::{
    AuthenticatedExecutionAdmissionV1, AuthenticatedExecutionContextV1, MarketEvidenceBundleV1,
    ReliabilityArtifactVerifiedMarketEvidenceBundleV3, ReliabilityArtifactVerifiedRoutingContextV1,
    ReplayProtectedMarketEvidenceBundleV4, ReplayStateTransitionV1, SignedCapacityOfferV1,
    SignedInferenceRequestPolicyV1, SignedMarketEvidenceBundleV2, SignedReliabilityArtifactV1,
    verify_market_evidence_bundle_v1,
    verify_reliability_artifact_verified_market_evidence_bundle_v3,
    verify_replay_protected_market_evidence_bundle_v4, verify_signed_market_evidence_bundle_v2,
};
use hyperintel_reconciliation::{
    ReceiptReconciliationInputV1, ReceiptReconciliationReportV1, reconcile_receipts_v1,
    verify_receipt_reconciliation_report_v1,
};
use serde::Deserialize;

/// Maximum combined UTF-8 bytes across the two caller-selected JSON documents.
pub const MAX_INPUT_BYTES: usize = 150_000;

/// Caller-selected evidence and public-key inputs for one offline verification.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HyperIntelBundleInputV1 {
    /// `HyperIntel` CLI-compatible bundle input, including separate evidence blobs.
    pub evidence_input: String,
    /// Caller-controlled public verification-key registry.
    pub trusted_keys_input: String,
}

/// Compact local result from a source-owned `HyperIntel` bundle verifier.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct HyperIntelBundleVerificationV1 {
    /// `HyperAI` interface result schema.
    pub schema_version: &'static str,
    /// Local verification result under caller-supplied trust.
    pub status: &'static str,
    /// Owner of the verified bundle contract.
    pub source_product: &'static str,
    /// Exact `HyperIntel` bundle schema handled by the owner verifier.
    pub bundle_schema_version: String,
    /// State slice carried by the verified `HyperIntel` bundle.
    pub state_slice: String,
    /// Bundle reconstruction time; this is not a freshness judgment.
    pub evaluated_at: String,
    /// Digest checked by the owner verifier.
    pub bundle_digest: String,
    /// Candidate offers represented by the bundle.
    pub candidate_count: usize,
    /// Caller-supplied keys loaded into the verification registry.
    pub trusted_key_count: usize,
    /// Content-addressed assurance blobs supplied to the verifier.
    pub assurance_blob_count: usize,
    /// Typed reliability artifacts supplied to V3 and V4 verifiers.
    pub reliability_artifact_count: usize,
    /// Assumptions retained from the verified bundle.
    pub assumptions: Vec<String>,
    /// Required and bundle-declared nonclaims retained from the verified bundle.
    pub explicit_nonclaims: Vec<String>,
    /// Fixed limit on the meaning of this interface result.
    pub claim_ceiling: &'static str,
    /// The source bundle grants no authority.
    pub authority_granted: bool,
    /// The adapter retains no input or output artifact.
    pub inputs_persisted: bool,
}

/// Caller-selected serialized lineage and public-key trust for offline
/// reconciliation. The execution JSON may contain stream text; callers must
/// use synthetic or otherwise explicitly authorized non-sensitive data only.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HyperIntelReconciliationInputV1 {
    /// Complete source-owned execution, replay, journal, and routing lineage.
    pub execution_input: String,
    /// Caller-controlled public verification-key registry.
    pub trusted_keys_input: String,
}

/// Compact report from the pinned source-owned execution reconciliation.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct HyperIntelReconciliationV1 {
    /// `HyperAI` interface result schema.
    pub schema_version: &'static str,
    /// Local source-contract result; this is not external acceptance.
    pub status: &'static str,
    /// Owner of the reconciled execution contracts.
    pub source_product: &'static str,
    /// Identity-only report emitted by the source-owned reconciler.
    pub report: ReceiptReconciliationReportV1,
    /// Caller-supplied keys loaded into the verification registry.
    pub trusted_key_count: usize,
    /// Signed capacity offers in the supplied route snapshot.
    pub signed_offer_count: usize,
    /// Content-addressed assurance blobs in the supplied route snapshot.
    pub assurance_blob_count: usize,
    /// Reliability artifacts in the supplied route snapshot.
    pub reliability_artifact_count: usize,
    /// The source report grants no execution or settlement authority.
    pub authority_granted: bool,
    /// The adapter retains no input or output artifact.
    pub inputs_persisted: bool,
    /// Fixed limit on the meaning of this interface result.
    pub claim_ceiling: &'static str,
}

/// Fail-closed input and verification failures with non-sensitive messages.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum HyperIntelBundleError {
    /// The bundle input is malformed or violates a versioned contract.
    #[error("evidence input is invalid")]
    InvalidEvidence,
    /// The bundle version has no verifier in the pinned adapter.
    #[error("unsupported HyperIntel evidence schema")]
    UnsupportedSchema,
    /// The caller-supplied public-key registry is malformed or inconsistent.
    #[error("trusted-key input is invalid")]
    InvalidTrust,
    /// The combined caller-selected inputs exceed the local interface limit.
    #[error("selected inputs exceed the local verification size limit")]
    InputTooLarge,
    /// An owner verifier rejected trust, signatures, digests, or reconstruction.
    #[error("HyperIntel evidence verification failed")]
    VerificationFailed,
}

/// Fail-closed parsing, trust, and source reconciliation errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum HyperIntelReconciliationError {
    /// The execution lineage is malformed or violates its versioned contract.
    #[error("execution reconciliation input is invalid")]
    InvalidEvidence,
    /// The lineage schema has no verifier in the pinned adapter.
    #[error("unsupported HyperIntel reconciliation input schema")]
    UnsupportedSchema,
    /// The caller-supplied public-key registry is malformed or inconsistent.
    #[error("trusted-key input is invalid")]
    InvalidTrust,
    /// The combined caller-selected inputs exceed the local interface limit.
    #[error("selected inputs exceed the local reconciliation size limit")]
    InputTooLarge,
    /// A source-owned reconciliation or report reconstruction check rejected.
    #[error("HyperIntel execution reconciliation failed")]
    ReconciliationFailed,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationInputV1 {
    bundle: MarketEvidenceBundleV1,
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationInputV2 {
    bundle: SignedMarketEvidenceBundleV2,
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationInputV3 {
    bundle: ReliabilityArtifactVerifiedMarketEvidenceBundleV3,
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
    reliability_artifacts: BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationInputV4 {
    bundle: ReplayProtectedMarketEvidenceBundleV4,
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
    reliability_artifacts: BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
}

#[derive(Deserialize)]
struct VerificationInputProbeV1 {
    bundle: BundleSchemaProbeV1,
}

#[derive(Deserialize)]
struct BundleSchemaProbeV1 {
    schema_version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrustInputV1 {
    trusted_verification_keys: Vec<RegisteredVerificationKeyV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReconciliationDocumentV1 {
    schema_version: String,
    admission: AuthenticatedExecutionAdmissionV1,
    routing: ReconciliationRoutingV1,
    replay_transition: ReplayStateTransitionV1,
    signed_policy: SignedInferenceRequestPolicyV1,
    journal_start: ExecutionJournalStartV1,
    journal_terminal: ExecutionJournalTerminalV1,
    execution: LocalExecutionOutcomeV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReconciliationRoutingV1 {
    evaluated_at: UtcTimestamp,
    service: ServiceSpecV1,
    providers: BTreeMap<ProtocolId, ProviderIdentityV1>,
    offers: Vec<SignedCapacityOfferV1>,
    policy: InferenceRequestPolicyV1,
    assurance_sets: BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
    reliability_artifacts: BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
}

struct SummaryDetailsV1 {
    bundle_schema_version: String,
    state_slice: String,
    evaluated_at: String,
    bundle_digest: String,
    candidate_count: usize,
    assurance_blob_count: usize,
    reliability_artifact_count: usize,
    assumptions: Vec<String>,
    explicit_nonclaims: Vec<String>,
}

/// Verify one `HyperIntel` V1–V4 market-evidence input under caller-supplied keys.
///
/// Input must match the pinned `HyperIntel` CLI's JSON contracts. Verification
/// runs in memory and returns only a compact, non-authorizing summary.
///
/// # Errors
///
/// Returns a closed error category for malformed input, unsupported schemas,
/// invalid trust, or any source-owned verifier rejection.
pub fn verify_market_evidence_input_v1(
    input: &HyperIntelBundleInputV1,
) -> Result<HyperIntelBundleVerificationV1, HyperIntelBundleError> {
    if input
        .evidence_input
        .len()
        .saturating_add(input.trusted_keys_input.len())
        > MAX_INPUT_BYTES
    {
        return Err(HyperIntelBundleError::InputTooLarge);
    }

    validate_unique_json(&input.evidence_input)
        .map_err(|_| HyperIntelBundleError::InvalidEvidence)?;
    validate_unique_json(&input.trusted_keys_input)
        .map_err(|_| HyperIntelBundleError::InvalidTrust)?;

    let probe: VerificationInputProbeV1 = serde_json::from_str(&input.evidence_input)
        .map_err(|_| HyperIntelBundleError::InvalidEvidence)?;
    if !matches!(
        probe.bundle.schema_version.as_str(),
        "hyperintel.market-evidence-bundle.v1"
            | "hyperintel.market-evidence-bundle.v2"
            | "hyperintel.market-evidence-bundle.v3"
            | "hyperintel.market-evidence-bundle.v4"
    ) {
        return Err(HyperIntelBundleError::UnsupportedSchema);
    }
    let trust: TrustInputV1 = serde_json::from_str(&input.trusted_keys_input)
        .map_err(|_| HyperIntelBundleError::InvalidTrust)?;
    let trusted_key_count = trust.trusted_verification_keys.len();
    let mut trusted_keys = VerificationKeyRegistry::default();
    for key in trust.trusted_verification_keys {
        trusted_keys
            .register(key)
            .map_err(|_| HyperIntelBundleError::InvalidTrust)?;
    }

    let details = match probe.bundle.schema_version.as_str() {
        "hyperintel.market-evidence-bundle.v1" => verify_v1(&input.evidence_input, &trusted_keys)?,
        "hyperintel.market-evidence-bundle.v2" => verify_v2(&input.evidence_input, &trusted_keys)?,
        "hyperintel.market-evidence-bundle.v3" => verify_v3(&input.evidence_input, &trusted_keys)?,
        "hyperintel.market-evidence-bundle.v4" => verify_v4(&input.evidence_input, &trusted_keys)?,
        _ => return Err(HyperIntelBundleError::UnsupportedSchema),
    };
    Ok(summary(details, trusted_key_count))
}

/// Reconcile one caller-selected `HyperIntel` execution lineage under
/// caller-supplied verification keys. Parsing, reconciliation, and report
/// re-verification all run in memory; stream payloads are never returned.
///
/// # Errors
///
/// Returns a closed error category for malformed or oversized input, invalid
/// trust, unsupported schema, or any source-owned reconciliation rejection.
pub fn reconcile_execution_input_v1(
    input: &HyperIntelReconciliationInputV1,
) -> Result<HyperIntelReconciliationV1, HyperIntelReconciliationError> {
    if input
        .execution_input
        .len()
        .saturating_add(input.trusted_keys_input.len())
        > MAX_INPUT_BYTES
    {
        return Err(HyperIntelReconciliationError::InputTooLarge);
    }

    validate_unique_json(&input.execution_input)
        .map_err(|_| HyperIntelReconciliationError::InvalidEvidence)?;
    validate_unique_json(&input.trusted_keys_input)
        .map_err(|_| HyperIntelReconciliationError::InvalidTrust)?;

    let document: ReconciliationDocumentV1 = serde_json::from_str(&input.execution_input)
        .map_err(|_| HyperIntelReconciliationError::InvalidEvidence)?;
    if document.schema_version != "hyperintel.receipt-reconciliation-input.v1" {
        return Err(HyperIntelReconciliationError::UnsupportedSchema);
    }
    let trust: TrustInputV1 = serde_json::from_str(&input.trusted_keys_input)
        .map_err(|_| HyperIntelReconciliationError::InvalidTrust)?;
    let trusted_key_count = trust.trusted_verification_keys.len();
    let mut verification_keys = VerificationKeyRegistry::default();
    for key in trust.trusted_verification_keys {
        verification_keys
            .register(key)
            .map_err(|_| HyperIntelReconciliationError::InvalidTrust)?;
    }

    let route = &document.routing;
    let authenticated = AuthenticatedExecutionContextV1 {
        routing: ReliabilityArtifactVerifiedRoutingContextV1 {
            evaluated_at: &route.evaluated_at,
            service: &route.service,
            providers: &route.providers,
            offers: &route.offers,
            policy: &route.policy,
            assurance_sets: &route.assurance_sets,
            assurance_blobs: &route.assurance_blobs,
            reliability_artifacts: &route.reliability_artifacts,
            verification_keys: &verification_keys,
        },
        replay_transition: &document.replay_transition,
        signed_policy: &document.signed_policy,
    };
    let reconciliation_input = ReceiptReconciliationInputV1 {
        admission: &document.admission,
        authenticated: &authenticated,
        verification_keys: &verification_keys,
        journal_start: &document.journal_start,
        journal_terminal: &document.journal_terminal,
        execution: &document.execution,
    };
    let report = reconcile_receipts_v1(&reconciliation_input)
        .map_err(|_| HyperIntelReconciliationError::ReconciliationFailed)?;
    verify_receipt_reconciliation_report_v1(&report, &reconciliation_input)
        .map_err(|_| HyperIntelReconciliationError::ReconciliationFailed)?;

    let result = HyperIntelReconciliationV1 {
        schema_version: "hyperai.interface.hyperintel-reconciliation.v1",
        status: "local_record_reconciliation_pass",
        source_product: "HyperIntel",
        report,
        trusted_key_count,
        signed_offer_count: route.offers.len(),
        assurance_blob_count: route.assurance_blobs.len(),
        reliability_artifact_count: route.reliability_artifacts.len(),
        authority_granted: false,
        inputs_persisted: false,
        claim_ceiling: "offline consistency reconstruction of caller-selected HyperIntel records under caller-supplied trust only; no independent metering, observation authenticity, correct computation, output truth, payment, settlement finality, or production claim",
    };
    Ok(result)
}

fn validate_unique_json(input: &str) -> Result<(), serde_json::Error> {
    parse_json_without_duplicate_keys(input).map(|_| ())
}

fn verify_v1(
    evidence: &str,
    trusted_keys: &VerificationKeyRegistry,
) -> Result<SummaryDetailsV1, HyperIntelBundleError> {
    let input: VerificationInputV1 =
        serde_json::from_str(evidence).map_err(|_| HyperIntelBundleError::InvalidEvidence)?;
    verify_market_evidence_bundle_v1(&input.bundle, &input.assurance_blobs, trusted_keys)
        .map_err(|_| HyperIntelBundleError::VerificationFailed)?;
    Ok(SummaryDetailsV1 {
        bundle_schema_version: input.bundle.schema_version,
        state_slice: input.bundle.state_slice,
        evaluated_at: input.bundle.evaluated_at.to_string(),
        bundle_digest: input.bundle.bundle_digest.to_string(),
        candidate_count: input.bundle.offers.len(),
        assurance_blob_count: input.assurance_blobs.len(),
        reliability_artifact_count: 0,
        assumptions: input.bundle.assumptions,
        explicit_nonclaims: input.bundle.explicit_nonclaims,
    })
}

fn verify_v2(
    evidence: &str,
    trusted_keys: &VerificationKeyRegistry,
) -> Result<SummaryDetailsV1, HyperIntelBundleError> {
    let input: VerificationInputV2 =
        serde_json::from_str(evidence).map_err(|_| HyperIntelBundleError::InvalidEvidence)?;
    verify_signed_market_evidence_bundle_v2(&input.bundle, &input.assurance_blobs, trusted_keys)
        .map_err(|_| HyperIntelBundleError::VerificationFailed)?;
    Ok(SummaryDetailsV1 {
        bundle_schema_version: input.bundle.schema_version,
        state_slice: input.bundle.state_slice,
        evaluated_at: input.bundle.evaluated_at.to_string(),
        bundle_digest: input.bundle.bundle_digest.to_string(),
        candidate_count: input.bundle.offers.len(),
        assurance_blob_count: input.assurance_blobs.len(),
        reliability_artifact_count: 0,
        assumptions: input.bundle.assumptions,
        explicit_nonclaims: input.bundle.explicit_nonclaims,
    })
}

fn verify_v3(
    evidence: &str,
    trusted_keys: &VerificationKeyRegistry,
) -> Result<SummaryDetailsV1, HyperIntelBundleError> {
    let input: VerificationInputV3 =
        serde_json::from_str(evidence).map_err(|_| HyperIntelBundleError::InvalidEvidence)?;
    verify_reliability_artifact_verified_market_evidence_bundle_v3(
        &input.bundle,
        &input.assurance_blobs,
        &input.reliability_artifacts,
        trusted_keys,
    )
    .map_err(|_| HyperIntelBundleError::VerificationFailed)?;
    Ok(SummaryDetailsV1 {
        bundle_schema_version: input.bundle.schema_version,
        state_slice: input.bundle.state_slice,
        evaluated_at: input.bundle.evaluated_at.to_string(),
        bundle_digest: input.bundle.bundle_digest.to_string(),
        candidate_count: input.bundle.offers.len(),
        assurance_blob_count: input.assurance_blobs.len(),
        reliability_artifact_count: input.reliability_artifacts.len(),
        assumptions: input.bundle.assumptions,
        explicit_nonclaims: input.bundle.explicit_nonclaims,
    })
}

fn verify_v4(
    evidence: &str,
    trusted_keys: &VerificationKeyRegistry,
) -> Result<SummaryDetailsV1, HyperIntelBundleError> {
    let input: VerificationInputV4 =
        serde_json::from_str(evidence).map_err(|_| HyperIntelBundleError::InvalidEvidence)?;
    verify_replay_protected_market_evidence_bundle_v4(
        &input.bundle,
        &input.assurance_blobs,
        &input.reliability_artifacts,
        trusted_keys,
    )
    .map_err(|_| HyperIntelBundleError::VerificationFailed)?;
    Ok(SummaryDetailsV1 {
        bundle_schema_version: input.bundle.schema_version,
        state_slice: input.bundle.state_slice,
        evaluated_at: input.bundle.evaluated_at.to_string(),
        bundle_digest: input.bundle.bundle_digest.to_string(),
        candidate_count: input.bundle.offers.len(),
        assurance_blob_count: input.assurance_blobs.len(),
        reliability_artifact_count: input.reliability_artifacts.len(),
        assumptions: input.bundle.assumptions,
        explicit_nonclaims: input.bundle.explicit_nonclaims,
    })
}

fn summary(details: SummaryDetailsV1, trusted_key_count: usize) -> HyperIntelBundleVerificationV1 {
    HyperIntelBundleVerificationV1 {
        schema_version: "hyperai.interface.hyperintel-verification.v1",
        status: "local_contract_verification_pass",
        source_product: "HyperIntel",
        bundle_schema_version: details.bundle_schema_version,
        state_slice: details.state_slice,
        evaluated_at: details.evaluated_at,
        bundle_digest: details.bundle_digest,
        candidate_count: details.candidate_count,
        trusted_key_count,
        assurance_blob_count: details.assurance_blob_count,
        reliability_artifact_count: details.reliability_artifact_count,
        assumptions: details.assumptions,
        explicit_nonclaims: details.explicit_nonclaims,
        claim_ceiling: "offline HyperIntel contract reconstruction under caller-supplied trust only; no observation truth, live provider state, execution, output correctness, payment, or production claim",
        authority_granted: false,
        inputs_persisted: false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HyperIntelBundleError, HyperIntelBundleInputV1, MAX_INPUT_BYTES,
        verify_market_evidence_input_v1,
    };
    use serde_json::{Value, json};

    const V1_EVIDENCE: &[u8] = include_bytes!(
        "../../../products/hyperintel/crates/hyperintel-cli/tests/fixtures/evidence-v1.json"
    );
    const V2_EVIDENCE: &[u8] = include_bytes!(
        "../../../products/hyperintel/crates/hyperintel-cli/tests/fixtures/evidence-v2.json"
    );
    const V3_EVIDENCE: &[u8] = include_bytes!(
        "../../../products/hyperintel/crates/hyperintel-cli/tests/fixtures/evidence-v3.json"
    );
    const V4_EVIDENCE: &[u8] = include_bytes!(
        "../../../products/hyperintel/crates/hyperintel-cli/tests/fixtures/evidence-v4.json"
    );
    const V1_V2_TRUST: &[u8] = include_bytes!(
        "../../../products/hyperintel/crates/hyperintel-cli/tests/fixtures/trusted-keys-v1-v2.json"
    );
    const V4_TRUST: &[u8] = include_bytes!(
        "../../../products/hyperintel/crates/hyperintel-cli/tests/fixtures/trusted-keys-v4.json"
    );
    const EMPTY_TRUST: &[u8] = include_bytes!(
        "../../../products/hyperintel/crates/hyperintel-cli/tests/fixtures/empty-trusted-keys.json"
    );

    fn input(
        evidence: &[u8],
        trust: &[u8],
    ) -> Result<HyperIntelBundleInputV1, std::str::Utf8Error> {
        Ok(HyperIntelBundleInputV1 {
            evidence_input: std::str::from_utf8(evidence)?.to_owned(),
            trusted_keys_input: std::str::from_utf8(trust)?.to_owned(),
        })
    }

    #[test]
    fn contract_v1_verification_preserves_unsigned_offer_nonclaim()
    -> Result<(), Box<dyn std::error::Error>> {
        let result = verify_market_evidence_input_v1(&input(V1_EVIDENCE, V1_V2_TRUST)?)?;
        assert_eq!(
            result.bundle_schema_version,
            "hyperintel.market-evidence-bundle.v1"
        );
        assert!(
            result
                .explicit_nonclaims
                .iter()
                .any(|claim| { claim == "no-capacity-offer-signature-verification" })
        );
        assert!(!result.authority_granted);
        assert!(!result.inputs_persisted);
        Ok(())
    }

    #[test]
    fn contract_v4_verification_uses_owner_contract_and_returns_compact_identity()
    -> Result<(), Box<dyn std::error::Error>> {
        let result = verify_market_evidence_input_v1(&input(V4_EVIDENCE, V4_TRUST)?)?;
        assert_eq!(
            result.bundle_schema_version,
            "hyperintel.market-evidence-bundle.v4"
        );
        assert_eq!(result.status, "local_contract_verification_pass");
        assert_eq!(result.candidate_count, 2);
        assert_eq!(result.bundle_digest.len(), 64);
        assert_eq!(result.trusted_key_count, 4);
        assert!(!result.authority_granted);
        assert!(!result.inputs_persisted);
        Ok(())
    }

    #[test]
    fn contract_v2_and_v3_verification_preserve_versioned_artifact_counts()
    -> Result<(), Box<dyn std::error::Error>> {
        let v2 = verify_market_evidence_input_v1(&input(V2_EVIDENCE, V1_V2_TRUST)?)?;
        assert_eq!(
            v2.bundle_schema_version,
            "hyperintel.market-evidence-bundle.v2"
        );
        assert_eq!(v2.reliability_artifact_count, 0);
        assert!(!v2.authority_granted);

        let v3 = verify_market_evidence_input_v1(&input(V3_EVIDENCE, V4_TRUST)?)?;
        assert_eq!(
            v3.bundle_schema_version,
            "hyperintel.market-evidence-bundle.v3"
        );
        assert!(v3.reliability_artifact_count > 0);
        assert!(!v3.authority_granted);
        Ok(())
    }

    #[test]
    fn contract_empty_trust_registry_rejects_v4() -> Result<(), Box<dyn std::error::Error>> {
        let Err(error) = verify_market_evidence_input_v1(&input(V4_EVIDENCE, EMPTY_TRUST)?) else {
            return Err("empty trust registry was unexpectedly accepted".into());
        };
        assert_eq!(error, HyperIntelBundleError::VerificationFailed);
        Ok(())
    }

    #[test]
    fn contract_tampered_bundle_digest_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let mut evidence: Value = serde_json::from_slice(V4_EVIDENCE)?;
        evidence["bundle"]["bundle_digest"] = json!("00".repeat(32));
        let mut input = input(V4_EVIDENCE, V4_TRUST)?;
        input.evidence_input = serde_json::to_string(&evidence)?;
        let Err(error) = verify_market_evidence_input_v1(&input) else {
            return Err("tampered bundle digest was unexpectedly accepted".into());
        };
        assert_eq!(error, HyperIntelBundleError::VerificationFailed);
        Ok(())
    }

    #[test]
    fn contract_unknown_schema_is_rejected_before_deserialization()
    -> Result<(), Box<dyn std::error::Error>> {
        let input = HyperIntelBundleInputV1 {
            evidence_input: serde_json::to_string(&json!({
                "bundle": { "schema_version": "hyperintel.market-evidence-bundle.v99" }
            }))?,
            trusted_keys_input: r#"{"trusted_verification_keys":[]}"#.to_owned(),
        };
        let Err(error) = verify_market_evidence_input_v1(&input) else {
            return Err("unknown schema was unexpectedly accepted".into());
        };
        assert_eq!(error, HyperIntelBundleError::UnsupportedSchema);
        Ok(())
    }

    #[test]
    fn duplicate_json_keys_are_rejected_before_source_contract_parsing()
    -> Result<(), Box<dyn std::error::Error>> {
        let input = HyperIntelBundleInputV1 {
            evidence_input: r#"{"bundle":{"schema_version":"hyperintel.market-evidence-bundle.v4","schema_version":"hyperintel.market-evidence-bundle.v4"}}"#.to_owned(),
            trusted_keys_input: std::str::from_utf8(V4_TRUST)?.to_owned(),
        };
        assert_eq!(
            verify_market_evidence_input_v1(&input),
            Err(HyperIntelBundleError::InvalidEvidence)
        );

        let duplicate_map_key = HyperIntelBundleInputV1 {
            evidence_input: r#"{"bundle":{"schema_version":"hyperintel.market-evidence-bundle.v4"},"assurance_blobs":{"same":[],"same":[]}}"#.to_owned(),
            trusted_keys_input: std::str::from_utf8(V4_TRUST)?.to_owned(),
        };
        assert_eq!(
            verify_market_evidence_input_v1(&duplicate_map_key),
            Err(HyperIntelBundleError::InvalidEvidence)
        );

        let duplicate_trust_key = HyperIntelBundleInputV1 {
            evidence_input: std::str::from_utf8(V4_EVIDENCE)?.to_owned(),
            trusted_keys_input:
                r#"{"trusted_verification_keys":[],"trusted_verification_keys":[]}"#.to_owned(),
        };
        assert_eq!(
            verify_market_evidence_input_v1(&duplicate_trust_key),
            Err(HyperIntelBundleError::InvalidTrust)
        );
        Ok(())
    }

    #[test]
    fn malformed_evidence_and_trust_json_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let malformed_evidence = HyperIntelBundleInputV1 {
            evidence_input: "{\"bundle\":".to_owned(),
            trusted_keys_input: std::str::from_utf8(V4_TRUST)?.to_owned(),
        };
        assert_eq!(
            verify_market_evidence_input_v1(&malformed_evidence),
            Err(HyperIntelBundleError::InvalidEvidence)
        );

        let malformed_trust = HyperIntelBundleInputV1 {
            evidence_input: std::str::from_utf8(V4_EVIDENCE)?.to_owned(),
            trusted_keys_input: "[]".to_owned(),
        };
        assert_eq!(
            verify_market_evidence_input_v1(&malformed_trust),
            Err(HyperIntelBundleError::InvalidTrust)
        );
        Ok(())
    }

    #[test]
    fn combined_input_size_limit_is_enforced_before_json_parsing() {
        let input = HyperIntelBundleInputV1 {
            evidence_input: " ".repeat(MAX_INPUT_BYTES + 1),
            trusted_keys_input: String::new(),
        };
        assert_eq!(
            verify_market_evidence_input_v1(&input),
            Err(HyperIntelBundleError::InputTooLarge)
        );
    }
}
