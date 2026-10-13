//! Local synthetic contracts for buyer-controlled compute procurement.
//!
//! Listings, quotes, buyer approvals, and fulfillment receipts are signed and
//! digest-bound. Settlement remains a mock; the local journal only persists
//! synthetic disposition records and never moves funds or proves computation.

use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use data_encoding::HEXLOWER;
use ed25519_dalek::{Signature, Signer as _, SigningKey, Verifier as _, VerifyingKey};
use hyperai_foundation::{
    CanonicalizationError, Sha256Digest, TimestampError, UtcTimestamp, canonical_json,
    domain_separated_digest,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const OFFER_SCHEMA: &str = "hyperai.compute-market.offer.v1";
const CATALOG_SCHEMA: &str = "hyperai.compute-market.catalog.v1";
const REQUEST_SCHEMA: &str = "hyperai.compute-market.request.v1";
const WORKLOAD_INTENT_SCHEMA: &str = "hyperai.compute-market.workload-intent.v1";
const WORKLOAD_REQUEST_BINDING_SCHEMA: &str = "hyperai.compute-market.workload-request-binding.v1";
const QUOTE_SCHEMA: &str = "hyperai.compute-market.quote.v1";
const DEAL_SCHEMA: &str = "hyperai.compute-market.deal.v1";
const FULFILLMENT_SCHEMA: &str = "hyperai.compute-market.fulfillment.v1";
const FULFILLMENT_FAILURE_SCHEMA: &str = "hyperai.compute-market.fulfillment-failure.v1";
const VERIFIER_SCHEMA: &str = "hyperai.compute-market.verifier-decision.v1";
const SETTLEMENT_SCHEMA: &str = "hyperai.compute-market.mock-settlement.v1";
const OBSERVATION_SCHEMA: &str = "hyperai.compute-market.advisory-observation.v1";
const SUPPLY_COMPARISON_SCHEMA: &str = "hyperai.compute-market.supply-comparison.v1";
const SUPPLY_COMPARISON_SLICE: &str = "hyperai.bazaar.buyer_supply_comparison.v1";
const SUPPLY_QUOTE_PREVIEW_SCHEMA: &str = "hyperai.compute-market.supply-quote-preview.v1";
const SUPPLY_QUOTE_PREVIEW_SLICE: &str = "hyperai.bazaar.supply_to_quote.v1";
const LOCAL_RECEIPT_SCHEMA: &str = "hyperai.compute-market.local-receipt.v1";
const LOCAL_JOURNAL_SCHEMA: &str = "hyperai.compute-market.local-journal.v1";
const LOCAL_QUOTE_PREVIEW_SCHEMA: &str = "hyperai.compute-market.local-quote-preview.v1";
const LOCAL_QUOTE_APPROVAL_SCHEMA: &str = "hyperai.compute-market.local-quote-approval.v1";
const LOCAL_HISTORY_PAGE_SCHEMA: &str = "hyperai.compute-market.local-history-page.v1";
const LOCAL_ACCEPTANCE_SLICE: &str = "hyperai.compute_market.local_acceptance.v1";
const LOCAL_HISTORY_SLICE: &str = "hyperai.compute_market.local_deal_history.v1";
const LOCAL_QUOTE_REVIEW_SLICE: &str = "hyperai.bazaar_interface.synthetic_quote_review.v1";
const LOCAL_FIXTURE_TIME: &str = "2026-09-24T12:00:00Z";
const LOCAL_FIXTURE_ACCEPTANCE_TIME: &str = "2026-09-24T12:05:00Z";
const LOCAL_EFFECTS: &str =
    "synthetic local records only; no network, provider calls, or funds moved";
const LOCAL_CLAIM_CEILING: &str =
    "contract integrity and mock lifecycle only; no computation correctness or payment claim";
const LOCAL_NONCLAIMS: &str = "seller signatures are attribution only; mock verifier and settlement are not independent or real";
const MAX_ARTIFACT_BYTES: usize = 1024 * 1024;
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Errors from local market contract validation.
#[derive(Debug, Error)]
pub enum MarketError {
    /// Canonical serialization or digest construction failed.
    #[error("canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
    /// A UTC timestamp was malformed or not canonical.
    #[error("timestamp is invalid")]
    Timestamp(#[from] TimestampError),
    /// A schema, identity, key, signature, or lifecycle invariant failed.
    #[error("invalid market record: {0}")]
    InvalidRecord(&'static str),
    /// Two registries supplied different signed bytes for one offer identity.
    #[error("conflicting offer identity")]
    ConflictingOffer,
    /// No listing satisfied the buyer's stated constraints.
    #[error("no offer satisfies the buyer request")]
    NoEligibleOffer,
    /// The displayed supply comparison no longer matches the current source.
    #[error("buyer supply comparison changed before quote preview")]
    SupplyComparisonMismatch,
    /// The frozen buyer request has no eligible offer to quote.
    #[error("no eligible offer is available for quote preview")]
    SupplyQuoteNoEligibleOffer,
    /// Integer amount arithmetic exceeded the representable range.
    #[error("amount arithmetic overflow")]
    AmountOverflow,
    /// A local settlement attempt replayed a deal that was already recorded.
    #[error("deal was already settled by this mock ledger")]
    Replay,
    /// A serialized local artifact exceeded the bounded format size.
    #[error("local market artifact exceeds the 1 MiB limit")]
    ArtifactTooLarge,
    /// A caller-selected destination already exists and was left untouched.
    #[error("local market artifact destination already exists")]
    ArtifactExists,
    /// A local artifact path is a symbolic link or not a regular file.
    #[error("local market artifact path is not a regular file")]
    UnsafeArtifactPath,
    /// The caller's quote preview does not match the deterministic fixture.
    #[error("quote preview changed or is no longer valid")]
    QuotePreviewMismatch,
    /// The caller's approved-deal digest does not match the synthetic approval.
    #[error("approved deal changed or does not match the quote preview")]
    QuoteApprovalMismatch,
    /// Local artifact file access failed.
    #[error("local market artifact I/O failed: {0}")]
    Io(#[from] std::io::Error),
    /// A serialized local artifact could not be decoded.
    #[error("local market artifact JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
}

/// Seller-authored compute offer with a seller signature.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OfferV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// Seller-selected stable identifier within the seller's namespace.
    pub offer_id: String,
    /// Seller identity asserted by the signer; registry admission is separate.
    pub seller_id: String,
    /// GPU or compute resource model.
    pub resource_model: String,
    /// Seller's declared region.
    pub region: String,
    /// Available number of units.
    pub capacity_units: u32,
    /// Asking price in integer micro-USD per unit-hour.
    pub price_microusd_per_unit_hour: u64,
    /// Listing creation time.
    pub issued_at: UtcTimestamp,
    /// Listing expiration time.
    pub expires_at: UtcTimestamp,
}

/// Signed offer bytes and the self-declared signing key.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignedOfferV1 {
    /// Signed offer fields.
    pub offer: OfferV1,
    /// Lowercase hexadecimal Ed25519 public key.
    pub public_key_hex: String,
    /// Lowercase hexadecimal Ed25519 signature.
    pub signature_hex: String,
}

/// A source-attributed entry in a caller-supplied catalog snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogEntryV1 {
    /// Verified signed offer.
    pub signed_offer: SignedOfferV1,
    /// Digest of the full signed offer.
    pub offer_digest: Sha256Digest,
    /// Registry/source labels that returned this exact offer.
    pub source_ids: Vec<String>,
}

/// Digest-bound catalog snapshot body.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogSnapshotBodyV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// Evaluation time used for listing freshness checks.
    pub as_of: UtcTimestamp,
    /// Number of distinct caller-supplied source identifiers.
    pub source_count: u32,
    /// Number of identical repeated listing observations collapsed.
    pub duplicate_count: u32,
    /// Deduplicated entries sorted by offer identity.
    pub entries: Vec<CatalogEntryV1>,
}

/// Catalog body and its deterministic digest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogSnapshotV1 {
    /// Snapshot content.
    pub body: CatalogSnapshotBodyV1,
    /// Domain-separated digest of `body`.
    pub digest: Sha256Digest,
}

/// Buyer-controlled resource and spend constraints.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuyerRequestV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// Buyer identity asserted by the later approval signature.
    pub buyer_id: String,
    /// Requested resource model.
    pub resource_model: String,
    /// Required seller-declared region.
    pub region: String,
    /// Number of units.
    pub units: u32,
    /// Whole hours to procure.
    pub duration_hours: u32,
    /// Maximum asking rate in micro-USD per unit-hour.
    pub max_rate_microusd_per_unit_hour: u64,
    /// Maximum total quote amount in micro-USD.
    pub max_total_microusd: u64,
    /// Request creation time.
    pub requested_at: UtcTimestamp,
    /// Latest time at which the buyer may approve a seller quote.
    pub valid_until: UtcTimestamp,
}

/// Source-owned data reference carried by a buyer workload intent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadDataReferenceV1 {
    /// Stable owner identifier such as `hyperai_data`.
    pub owner_id: String,
    /// Versioned source schema identifier.
    pub schema_version: String,
    /// Source-owned record identifier.
    pub record_id: String,
    /// Digest of the exact source-owned record.
    pub digest: Sha256Digest,
}

/// Immutable buyer workload intent body. Private payloads are represented by
/// digests and owner-held references, never embedded in this contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadIntentBodyV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// Stable opaque identity shared across revisions of one workload.
    pub workload_id: String,
    /// Positive immutable revision number.
    pub revision: u32,
    /// Digest of the immediately preceding revision, when revision is greater
    /// than one.
    pub previous_revision_digest: Option<Sha256Digest>,
    /// Digest of the job specification; the specification itself stays with
    /// its owner.
    pub specification_digest: Sha256Digest,
    /// Digest of the buyer-defined acceptance contract.
    pub acceptance_contract_digest: Sha256Digest,
    /// Digest of the data-handling policy applied to this workload.
    pub data_handling_policy_digest: Sha256Digest,
    /// Source-owned data inputs, sorted by owner, schema, and record ID.
    pub data_references: Vec<WorkloadDataReferenceV1>,
    /// Optional latest completion time for this workload.
    pub deadline: Option<UtcTimestamp>,
    /// Intent creation time.
    pub created_at: UtcTimestamp,
}

/// Immutable workload intent body and canonical digest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadIntentV1 {
    /// Intent content.
    pub body: WorkloadIntentBodyV1,
    /// Domain-separated digest of `body`.
    pub digest: Sha256Digest,
}

/// Explicit binding body between one workload intent and an unchanged buyer
/// request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadRequestBindingBodyV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// Stable opaque workload identity.
    pub workload_id: String,
    /// Digest of the exact immutable workload intent.
    pub intent_digest: Sha256Digest,
    /// Digest of the exact existing `BuyerRequestV1`.
    pub buyer_request_digest: Sha256Digest,
}

/// Immutable workload-to-request binding and canonical digest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadRequestBindingV1 {
    /// Binding content.
    pub body: WorkloadRequestBindingBodyV1,
    /// Domain-separated digest of `body`.
    pub digest: Sha256Digest,
}

/// Build and validate one immutable workload intent.
///
/// # Errors
///
/// Returns an error when the schema, identity, revision chain, or ordered
/// source references are invalid, or when canonical digest construction fails.
pub fn create_workload_intent_v1(
    body: WorkloadIntentBodyV1,
) -> Result<WorkloadIntentV1, MarketError> {
    validate_workload_intent_body(&body)?;
    let digest = domain_separated_digest("hyperai.compute-market.workload-intent.v1", &body)?;
    Ok(WorkloadIntentV1 { body, digest })
}

/// Verify a workload intent's shape and canonical digest.
///
/// # Errors
///
/// Returns an error when the intent is malformed or its digest does not match.
pub fn verify_workload_intent_v1(intent: &WorkloadIntentV1) -> Result<(), MarketError> {
    validate_workload_intent_body(&intent.body)?;
    if domain_separated_digest("hyperai.compute-market.workload-intent.v1", &intent.body)?
        != intent.digest
    {
        return Err(MarketError::InvalidRecord(
            "workload intent digest mismatch",
        ));
    }
    Ok(())
}

/// Create an immutable binding to one exact existing buyer request.
///
/// # Errors
///
/// Returns an error when the intent or request is invalid or digest
/// construction fails.
pub fn bind_workload_request_v1(
    intent: &WorkloadIntentV1,
    request: &BuyerRequestV1,
) -> Result<WorkloadRequestBindingV1, MarketError> {
    verify_workload_intent_v1(intent)?;
    validate_request(request, &request.requested_at)?;
    let body = WorkloadRequestBindingBodyV1 {
        schema_version: WORKLOAD_REQUEST_BINDING_SCHEMA.to_owned(),
        workload_id: intent.body.workload_id.clone(),
        intent_digest: intent.digest,
        buyer_request_digest: domain_separated_digest(
            "hyperai.compute-market.request.v1",
            request,
        )?,
    };
    let digest =
        domain_separated_digest("hyperai.compute-market.workload-request-binding.v1", &body)?;
    Ok(WorkloadRequestBindingV1 { body, digest })
}

/// Verify that an intent and buyer request match an immutable binding.
///
/// # Errors
///
/// Returns an error when either source record or the binding differs from the
/// exact digests in the binding.
pub fn verify_workload_request_binding_v1(
    intent: &WorkloadIntentV1,
    request: &BuyerRequestV1,
    binding: &WorkloadRequestBindingV1,
) -> Result<(), MarketError> {
    verify_workload_intent_v1(intent)?;
    validate_request(request, &request.requested_at)?;
    let request_digest = domain_separated_digest("hyperai.compute-market.request.v1", request)?;
    if binding.body.schema_version != WORKLOAD_REQUEST_BINDING_SCHEMA
        || binding.body.workload_id != intent.body.workload_id
        || binding.body.intent_digest != intent.digest
        || binding.body.buyer_request_digest != request_digest
        || domain_separated_digest(
            "hyperai.compute-market.workload-request-binding.v1",
            &binding.body,
        )? != binding.digest
    {
        return Err(MarketError::InvalidRecord(
            "workload request binding mismatch",
        ));
    }
    Ok(())
}

fn validate_workload_intent_body(body: &WorkloadIntentBodyV1) -> Result<(), MarketError> {
    if body.schema_version != WORKLOAD_INTENT_SCHEMA
        || !valid_workload_id(&body.workload_id)
        || body.revision == 0
        || (body.revision == 1) != body.previous_revision_digest.is_none()
        || body.data_references.len() > 64
    {
        return Err(MarketError::InvalidRecord("workload intent fields"));
    }
    for reference in &body.data_references {
        if !valid_workload_id(&reference.owner_id)
            || !valid_schema_id(&reference.schema_version)
            || reference.record_id.is_empty()
            || reference.record_id.len() > 256
            || reference.record_id.chars().any(char::is_control)
        {
            return Err(MarketError::InvalidRecord("workload data reference fields"));
        }
    }
    if body.data_references.windows(2).any(|pair| {
        (
            pair[0].owner_id.as_str(),
            pair[0].schema_version.as_str(),
            pair[0].record_id.as_str(),
        ) >= (
            pair[1].owner_id.as_str(),
            pair[1].schema_version.as_str(),
            pair[1].record_id.as_str(),
        )
    }) {
        return Err(MarketError::InvalidRecord(
            "workload data references must be uniquely sorted",
        ));
    }
    Ok(())
}

fn valid_workload_id(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'))
}

fn valid_fulfillment_attempt_id(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_schema_id(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

/// Seller-signed price and terms for the requested units.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SellerQuoteV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// Listing identity being quoted.
    pub offer_digest: Sha256Digest,
    /// Digest of the buyer's complete request constraints.
    pub request_digest: Sha256Digest,
    /// Seller identity asserted by the offer.
    pub seller_id: String,
    /// Seller's public key, which must match the offer key.
    pub seller_public_key_hex: String,
    /// Agreed rate in micro-USD per unit-hour.
    pub rate_microusd_per_unit_hour: u64,
    /// Number of units.
    pub units: u32,
    /// Agreed whole hours.
    pub duration_hours: u32,
    /// Exact expected total in micro-USD.
    pub quoted_total_microusd: u64,
    /// Buyer-approved hard spend cap in micro-USD.
    pub amount_cap_microusd: u64,
    /// Quote acceptance deadline.
    pub expires_at: UtcTimestamp,
}

/// Seller quote payload and signature.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignedSellerQuoteV1 {
    /// Quote terms.
    pub quote: SellerQuoteV1,
    /// Lowercase hexadecimal Ed25519 signature.
    pub signature_hex: String,
}

/// Buyer-signed approval binding the quote to one deal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuyerApprovedDealV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// Seller quote and its signature.
    pub signed_quote: SignedSellerQuoteV1,
    /// Exact buyer constraints approved with the quote.
    pub buyer_request: BuyerRequestV1,
    /// Buyer identity asserted by the approval signer.
    pub buyer_id: String,
    /// Lowercase hexadecimal Ed25519 buyer public key.
    pub buyer_public_key_hex: String,
    /// Buyer acceptance time.
    pub accepted_at: UtcTimestamp,
    /// Lowercase hexadecimal Ed25519 approval signature.
    pub signature_hex: String,
}

/// Seller-signed completion and metering receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FulfillmentReceiptV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// Digest of the buyer-approved deal.
    pub deal_digest: Sha256Digest,
    /// Seller identity.
    pub seller_id: String,
    /// Seller public key.
    pub seller_public_key_hex: String,
    /// Amount claimed by seller in micro-USD.
    pub billed_microusd: u64,
    /// Seller-reported completion time.
    pub completed_at: UtcTimestamp,
    /// Digest of the private/offline result artifact, if one exists.
    pub result_digest: Sha256Digest,
    /// Seller's signature over all receipt fields above.
    pub signature_hex: String,
}

/// Closed seller-reported reason for one failed fulfillment attempt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FulfillmentFailureCodeV1 {
    /// The seller reports that the requested resource was unavailable.
    ResourceUnavailable,
    /// The seller reports that the attempt ended with an execution error.
    ExecutionError,
    /// The seller reports that the attempt exceeded its deadline.
    DeadlineExceeded,
}

/// Seller-signed failure statement for one attempt bound to an approved deal.
///
/// This record contains no charge, refund, verifier, completion, or settlement
/// decision. Its signature attributes the statement to a key only.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FulfillmentFailureReceiptV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// Digest of the buyer-approved deal.
    pub deal_digest: Sha256Digest,
    /// Seller identity.
    pub seller_id: String,
    /// Seller public key.
    pub seller_public_key_hex: String,
    /// Stable identifier for this seller-reported attempt.
    pub attempt_id: String,
    /// Seller-reported attempt start time.
    pub started_at: UtcTimestamp,
    /// Seller-reported failure time.
    pub failed_at: UtcTimestamp,
    /// Closed failure reason code.
    pub failure_code: FulfillmentFailureCodeV1,
    /// Lowercase hexadecimal Ed25519 signature over all fields above.
    pub signature_hex: String,
}

/// Local verifier verdict; this record is not independent evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionVerdict {
    /// The configured local mock verifier selected completion.
    Verified,
    /// The configured local mock verifier selected non-completion.
    Rejected,
    /// The configured local mock verifier declined to decide.
    Inconclusive,
}

/// Deal-bound decision from the local mock verifier.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MockVerifierDecisionV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// Digest of the buyer-approved deal.
    pub deal_digest: Sha256Digest,
    /// Digest of the seller fulfillment receipt.
    pub receipt_digest: Sha256Digest,
    /// Local mock verdict.
    pub verdict: CompletionVerdict,
    /// Short synthetic reason code.
    pub reason_code: String,
}

/// Financial disposition recorded by the in-memory mock only.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MockDisposition {
    /// Mock ledger would release the approved amount to the seller.
    ReleaseToSeller,
    /// Mock ledger would return funds to the buyer.
    RefundToBuyer,
    /// Mock ledger holds the deal for a dispute path.
    HoldForDispute,
}

/// In-memory mock settlement record. It is not a payment receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MockSettlementReceiptV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// Digest of the buyer-approved deal.
    pub deal_digest: Sha256Digest,
    /// Digest of the seller receipt.
    pub fulfillment_digest: Sha256Digest,
    /// Digest of the mock verifier decision.
    pub decision_digest: Sha256Digest,
    /// Mock-only disposition.
    pub disposition: MockDisposition,
    /// Time the local ledger recorded the disposition.
    pub recorded_at: UtcTimestamp,
}

/// Advisory market-data observation that never changes seller terms or buyer policy.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MarketObservationBodyV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// Synthetic or caller-provided source label.
    pub source_id: String,
    /// Offer identity this observation describes.
    pub offer_id: String,
    /// Source observation time.
    pub observed_at: UtcTimestamp,
    /// Optional modeled rate; it is not a seller quote or executed price.
    pub estimated_rate_microusd_per_unit_hour: Option<u64>,
    /// Optional estimated availability; None means unknown.
    pub estimated_available_units: Option<u32>,
}

/// Advisory observation body and its integrity digest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MarketObservationV1 {
    /// Observation data.
    pub body: MarketObservationBodyV1,
    /// Domain-separated digest of the observation body.
    pub digest: Sha256Digest,
}

/// A buyer constraint that excluded a signed offer from this request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OfferIneligibilityReasonV1 {
    /// The offer names a different resource model.
    ResourceModelMismatch,
    /// The offer names a different region.
    RegionMismatch,
    /// The seller-declared capacity is below the requested units.
    InsufficientCapacity,
    /// The seller asking rate exceeds the buyer's rate cap.
    AskingRateExceedsCap,
    /// The checked asking total exceeds the buyer's total cap.
    AskingTotalExceedsCap,
    /// The asking total overflowed checked integer arithmetic.
    AskingTotalOverflow,
}

/// Buyer-constraint assessment for one exact signed catalog entry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupplyOfferAssessmentV1 {
    /// Stable offer identity from the signed listing.
    pub offer_id: String,
    /// Digest of the full signed offer bytes.
    pub offer_digest: Sha256Digest,
    /// Source identifiers that supplied the exact signed offer.
    pub source_ids: Vec<String>,
    /// Signed seller asking rate in micro-USD per unit-hour.
    pub asking_rate_microusd_per_unit_hour: u64,
    /// Checked total at the seller asking rate; not a quote.
    pub asking_total_microusd: Option<u64>,
    /// False only when at least one buyer constraint excludes the offer.
    pub eligible: bool,
    /// All buyer-constraint reasons excluding this offer; empty if eligible.
    pub ineligibility_reasons: Vec<OfferIneligibilityReasonV1>,
    /// True when this is the deterministic lowest-asking eligible offer.
    pub selected: bool,
}

/// Digest-bound read projection comparing signed offers with advisory signals.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupplyComparisonBodyV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// State slice that owns this synthetic comparison.
    pub state_slice: String,
    /// Exact source-attributed signed catalog used for assessment.
    pub catalog: CatalogSnapshotV1,
    /// Exact buyer constraints used for assessment.
    pub buyer_request: BuyerRequestV1,
    /// Fixed comparison time used to validate the request and catalog.
    pub compared_at: UtcTimestamp,
    /// Deterministically selected eligible offer, or None when none qualify.
    pub selected_offer_id: Option<String>,
    /// One buyer-constraint assessment per catalog entry, sorted by offer ID.
    pub offer_assessments: Vec<SupplyOfferAssessmentV1>,
    /// Separate advisory records; these never affect assessment or selection.
    pub advisory_observations: Vec<MarketObservationV1>,
    /// Freshness is not assessed by this fixed local fixture.
    pub freshness_status: String,
    /// Maximum statement supported by this projection.
    pub claim_ceiling: String,
    /// Explicit excluded claims.
    pub nonclaims: String,
    /// Effects performed by this local projection.
    pub effects: String,
}

/// Supply comparison body and digest over every displayed field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupplyComparisonV1 {
    /// Comparison content.
    pub body: SupplyComparisonBodyV1,
    /// Domain-separated digest binding the complete body.
    pub digest: Sha256Digest,
}

/// Request to create a quote preview for one exact supply comparison.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupplyQuotePreviewRequestV1 {
    /// Digest of the complete buyer supply comparison previously displayed.
    pub comparison_digest: Sha256Digest,
}

/// Supply comparison to quote-preview binding and its source-owned quote.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupplyQuotePreviewBodyV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// State slice that owns this local projection.
    pub state_slice: String,
    /// Exact full digest of the comparison provided by the caller.
    pub comparison_digest: Sha256Digest,
    /// Digest of the catalog shared by comparison and quote preview.
    pub catalog_digest: Sha256Digest,
    /// Digest of the buyer request shared by comparison and quote preview.
    pub buyer_request_digest: Sha256Digest,
    /// Offer selected by the fixed buyer request.
    pub selected_offer_id: String,
    /// Digest of the exact signed offer selected by the comparison.
    pub selected_offer_digest: Sha256Digest,
    /// Existing seller quote preview built from the same verified fixture.
    pub quote_preview: LocalMarketQuotePreviewV1,
    /// Maximum statement supported by this projection.
    pub claim_ceiling: String,
    /// Explicit claims this projection does not support.
    pub nonclaims: String,
    /// Effects performed by this projection.
    pub effects: String,
}

/// Digest-bound link from the displayed supply comparison to its quote.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupplyQuotePreviewV1 {
    /// Projection body.
    pub body: SupplyQuotePreviewBodyV1,
    /// Domain-separated digest of the complete binding and nested preview.
    pub digest: Sha256Digest,
}

/// Full metadata-minimized record from the local synthetic demo.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMarketReceiptBodyV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// Authorized local state slice that produced this artifact.
    pub state_slice: String,
    /// Exact source-labelled registry snapshot.
    pub catalog: CatalogSnapshotV1,
    /// Separate advisory observations; never inputs to offer selection.
    pub advisory_observations: Vec<MarketObservationV1>,
    /// Buyer constraints used for deterministic selection.
    pub buyer_request: BuyerRequestV1,
    /// Listing selected only from the signed catalog and buyer request.
    pub selected_offer_id: String,
    /// Buyer-signed approval binding the seller quote and request.
    pub approved_deal: BuyerApprovedDealV1,
    /// Seller-signed fulfillment and metering statement.
    pub fulfillment: FulfillmentReceiptV1,
    /// Explicit local mock-verifier decision.
    pub verifier_decision: MockVerifierDecisionV1,
    /// Local mock disposition; this is not a payment receipt.
    pub mock_settlement: MockSettlementReceiptV1,
    /// Bound on what this artifact supports.
    pub claim_ceiling: String,
    /// Explicit excluded claims.
    pub nonclaims: String,
    /// Statement of effects performed by this synthetic flow.
    pub effects: String,
}

/// Full receipt body and integrity digest for offline contract verification.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMarketReceiptV1 {
    /// Receipt content.
    pub body: LocalMarketReceiptBodyV1,
    /// Domain-separated digest of the receipt body.
    pub digest: Sha256Digest,
}

/// Exact synthetic buyer request and seller quote shown before approval.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMarketQuotePreviewBodyV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// State slice that owns this staged interface flow.
    pub state_slice: String,
    /// Exact synthetic catalog used to select the offer.
    pub catalog: CatalogSnapshotV1,
    /// Exact buyer constraints bound by the seller quote.
    pub buyer_request: BuyerRequestV1,
    /// Offer selected by the existing buyer-constraint selector.
    pub selected_offer_id: String,
    /// Seller-signed synthetic quote awaiting a separate approval action.
    pub signed_quote: SignedSellerQuoteV1,
    /// Explicit simulated contract clock; not wall-clock freshness.
    pub simulated_at: UtcTimestamp,
    /// Maximum statement supported by this synthetic preview.
    pub claim_ceiling: String,
    /// Explicit excluded claims.
    pub nonclaims: String,
    /// Statement of effects performed by this preview.
    pub effects: String,
}

/// Full synthetic quote preview and digest over every displayed field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMarketQuotePreviewV1 {
    /// Preview content.
    pub body: LocalMarketQuotePreviewBodyV1,
    /// Domain-separated digest binding the complete preview body.
    pub digest: Sha256Digest,
}

/// One closed, HyperAI-authored local quote fixture scenario.
///
/// These scenarios exist to exercise the local synthetic quote and history
/// contracts. They do not represent live listings or buyer requests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalMarketQuoteScenarioV1 {
    /// The original fixed two-unit, three-hour fixture.
    Baseline,
    /// A distinct fixed one-unit, two-hour fixture for pagination coverage.
    Alternate,
}

impl LocalMarketQuoteScenarioV1 {
    /// The complete finite set accepted by local synthetic quote contracts.
    pub const ALL: [Self; 2] = [Self::Baseline, Self::Alternate];

    /// Parse a closed scenario path segment.
    #[must_use]
    pub fn from_scenario_id(scenario_id: &str) -> Option<Self> {
        match scenario_id {
            "baseline" => Some(Self::Baseline),
            "alternate" => Some(Self::Alternate),
            _ => None,
        }
    }

    /// Stable local route identifier for this fixture.
    #[must_use]
    pub const fn scenario_id(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::Alternate => "alternate",
        }
    }
}

/// Buyer approval created after the exact preview digest is submitted.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMarketQuoteApprovalV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// State slice that owns this staged interface flow.
    pub state_slice: String,
    /// Digest of the exact quote preview submitted for approval.
    pub preview_digest: Sha256Digest,
    /// Digest of the buyer-approved deal.
    pub deal_digest: Sha256Digest,
    /// Synthetic buyer signature over the approved quote and request.
    pub approved_deal: BuyerApprovedDealV1,
    /// Maximum statement supported by this synthetic approval.
    pub claim_ceiling: String,
    /// Explicit excluded claims.
    pub nonclaims: String,
    /// Statement of effects performed by this approval.
    pub effects: String,
}

/// One persisted local synthetic deal approval and optional mock completion.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMarketHistoryEntryV1 {
    /// Monotonic local insertion sequence used only for stable pagination.
    pub sequence: u64,
    /// The buyer-approved fixed synthetic quote.
    pub approval: LocalMarketQuoteApprovalV1,
    /// Mock lifecycle receipt, absent until the local completion action.
    pub completion_receipt: Option<LocalMarketReceiptV1>,
}

/// Digest-bound market-history page body.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMarketHistoryBodyV1 {
    /// Contract schema identifier.
    pub schema_version: String,
    /// State slice that owns this history projection.
    pub state_slice: String,
    /// Validated entries ordered newest first.
    pub entries: Vec<LocalMarketHistoryEntryV1>,
    /// Opaque cursor for the next page, when more entries are available.
    pub next_cursor: Option<String>,
    /// Maximum statement supported by this local projection.
    pub claim_ceiling: String,
    /// Explicit excluded claims.
    pub nonclaims: String,
    /// Effects performed by this local projection.
    pub effects: String,
}

/// Local synthetic market-history page and integrity digest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMarketHistoryPageV1 {
    /// Page content.
    pub body: LocalMarketHistoryBodyV1,
    /// Domain-separated digest binding every page field.
    pub digest: Sha256Digest,
}

/// Build and digest one validated local synthetic market-history page.
///
/// # Errors
///
/// Returns a market contract error if the page is not ordered or its records
/// do not satisfy the bounded history contract.
pub fn local_market_history_page_v1(
    entries: Vec<LocalMarketHistoryEntryV1>,
    next_cursor: Option<String>,
) -> Result<LocalMarketHistoryPageV1, MarketError> {
    if entries.len() > 50
        || entries.iter().any(|entry| entry.sequence == 0)
        || entries
            .windows(2)
            .any(|pair| pair[0].sequence <= pair[1].sequence)
        || next_cursor.as_ref().is_some_and(|cursor| cursor.len() > 64)
    {
        return Err(MarketError::InvalidRecord(
            "market history page bounds or order differ",
        ));
    }
    let body = LocalMarketHistoryBodyV1 {
        schema_version: LOCAL_HISTORY_PAGE_SCHEMA.to_owned(),
        state_slice: LOCAL_HISTORY_SLICE.to_owned(),
        entries,
        next_cursor,
        claim_ceiling: "local synthetic deal-history integrity only".to_owned(),
        nonclaims: "not a live marketplace, execution record, correct computation, payment entitlement, settlement, independent review, or production history".to_owned(),
        effects: "validated local synthetic records read from the owner-controlled market database; no external effects".to_owned(),
    };
    let digest = domain_separated_digest(LOCAL_HISTORY_PAGE_SCHEMA, &body)?;
    Ok(LocalMarketHistoryPageV1 { body, digest })
}

/// Verify the schema, ordering, bounds, and full page digest.
///
/// # Errors
///
/// Returns a market contract error when the page is malformed or changed.
pub fn verify_local_market_history_page_v1(
    page: &LocalMarketHistoryPageV1,
) -> Result<(), MarketError> {
    if page.body.schema_version != LOCAL_HISTORY_PAGE_SCHEMA
        || page.body.state_slice != LOCAL_HISTORY_SLICE
    {
        return Err(MarketError::InvalidRecord("market history schema differs"));
    }
    let rebuilt =
        local_market_history_page_v1(page.body.entries.clone(), page.body.next_cursor.clone())?;
    if rebuilt != *page {
        return Err(MarketError::InvalidRecord(
            "market history digest or content differs",
        ));
    }
    Ok(())
}

/// Single-deal local journal body used to recover a lost receipt export.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMarketJournalBodyV1 {
    /// Journal schema identifier.
    pub schema_version: String,
    /// Complete receipt retained for recovery.
    pub receipt: LocalMarketReceiptV1,
}

/// Journal body and digest. It records only one synthetic deal by design.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMarketJournalV1 {
    /// Journal content.
    pub body: LocalMarketJournalBodyV1,
    /// Domain-separated digest of the journal body.
    pub digest: Sha256Digest,
}

/// Aggregate return from the deterministic local synthetic demonstration.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalDemoReportV1 {
    /// Schema identifier.
    pub schema_version: String,
    /// Snapshot digest.
    pub catalog_digest: Sha256Digest,
    /// Number of distinct caller-supplied registry sources.
    pub source_count: u32,
    /// Number of identical offer observations collapsed.
    pub duplicate_count: u32,
    /// Identifier of the selected offer.
    pub selected_offer_id: String,
    /// Approved deal digest.
    pub deal_digest: Sha256Digest,
    /// Seller completion receipt digest.
    pub fulfillment_digest: Sha256Digest,
    /// Mock settlement output.
    pub settlement: MockSettlementReceiptV1,
    /// Disclosure of the mock boundary.
    pub effects: String,
    /// Maximum statement supported by this run.
    pub claim_ceiling: String,
    /// Complete receipt artifact digest.
    pub receipt_digest: Sha256Digest,
    /// Receipt export path relative to the explicitly selected output dir.
    pub receipt_file: Option<String>,
    /// Journal path relative to the explicitly selected output dir.
    pub journal_file: Option<String>,
}

/// Verify an offer signature and its validity interval.
///
/// # Errors
///
/// Returns an error when the listing is malformed, stale, or has an invalid
/// signature or digest encoding.
pub fn verify_offer(signed: &SignedOfferV1, as_of: &UtcTimestamp) -> Result<(), MarketError> {
    let offer = &signed.offer;
    if offer.schema_version != OFFER_SCHEMA
        || offer.offer_id.is_empty()
        || offer.seller_id.is_empty()
        || offer.resource_model.is_empty()
        || offer.region.is_empty()
        || offer.capacity_units == 0
        || offer.price_microusd_per_unit_hour == 0
        || offer.issued_at >= offer.expires_at
    {
        return Err(MarketError::InvalidRecord("offer fields or lifecycle"));
    }
    if as_of < &offer.issued_at || as_of >= &offer.expires_at {
        return Err(MarketError::InvalidRecord("offer is not current"));
    }
    verify_signature(
        &signed.public_key_hex,
        &signed.signature_hex,
        &signing_message("hyperai.compute-market.offer-signature.v1", offer)?,
    )
}

/// Build a deterministic catalog from caller-supplied, signed snapshots.
///
/// This function performs no network reads. A repeated identity is collapsed
/// only when every signed byte matches; conflicts fail the whole snapshot.
///
/// # Errors
///
/// Returns an error for an empty source label, stale or invalid listing, a
/// conflicting offer identity, or a digest/counter overflow.
pub fn build_catalog(
    observations: Vec<(String, SignedOfferV1)>,
    as_of: UtcTimestamp,
) -> Result<CatalogSnapshotV1, MarketError> {
    let mut sources = BTreeSet::new();
    let mut by_offer: BTreeMap<String, CatalogEntryV1> = BTreeMap::new();
    let mut duplicates = 0_u32;
    for (source_id, signed_offer) in observations {
        if source_id.is_empty() {
            return Err(MarketError::InvalidRecord("empty catalog source"));
        }
        sources.insert(source_id.clone());
        verify_offer(&signed_offer, &as_of)?;
        let offer_id = signed_offer.offer.offer_id.clone();
        let digest =
            domain_separated_digest("hyperai.compute-market.signed-offer.v1", &signed_offer)?;
        if let Some(existing) = by_offer.get_mut(&offer_id) {
            if existing.offer_digest != digest {
                return Err(MarketError::ConflictingOffer);
            }
            existing.source_ids.push(source_id);
            duplicates = duplicates
                .checked_add(1)
                .ok_or(MarketError::AmountOverflow)?;
        } else {
            by_offer.insert(
                offer_id,
                CatalogEntryV1 {
                    signed_offer,
                    offer_digest: digest,
                    source_ids: vec![source_id],
                },
            );
        }
    }
    for entry in by_offer.values_mut() {
        entry.source_ids.sort();
        entry.source_ids.dedup();
    }
    let body = CatalogSnapshotBodyV1 {
        schema_version: CATALOG_SCHEMA.to_owned(),
        as_of,
        source_count: u32::try_from(sources.len()).map_err(|_| MarketError::AmountOverflow)?,
        duplicate_count: duplicates,
        entries: by_offer.into_values().collect(),
    };
    let digest = domain_separated_digest("hyperai.compute-market.catalog.v1", &body)?;
    Ok(CatalogSnapshotV1 { body, digest })
}

/// Select the lowest-cost matching listing without widening buyer constraints.
///
/// # Errors
///
/// Returns an error when the request is invalid or no listing satisfies all
/// requested resource and spend constraints.
pub fn select_offer<'a>(
    catalog: &'a CatalogSnapshotV1,
    request: &BuyerRequestV1,
    now: &UtcTimestamp,
) -> Result<&'a CatalogEntryV1, MarketError> {
    validate_request(request, now)?;
    verify_catalog(catalog, now)?;
    catalog
        .body
        .entries
        .iter()
        .filter(|entry| {
            assess_offer_for_request(entry, request)
                .ineligibility_reasons
                .is_empty()
        })
        .min_by_key(|entry| {
            (
                entry.signed_offer.offer.price_microusd_per_unit_hour,
                entry.signed_offer.offer.offer_id.as_str(),
            )
        })
        .ok_or(MarketError::NoEligibleOffer)
}

/// Create a seller-signed quote for a selected listing.
///
/// # Errors
///
/// Returns an error when the listing is stale, the signer does not control the
/// listing key, the buyer request is invalid, or the quote exceeds its limits.
pub fn create_quote(
    entry: &CatalogEntryV1,
    request: &BuyerRequestV1,
    seller_key: &SigningKey,
    now: &UtcTimestamp,
) -> Result<SignedSellerQuoteV1, MarketError> {
    validate_request(request, now)?;
    verify_offer(&entry.signed_offer, now)?;
    let seller_public_key = HEXLOWER.encode(seller_key.verifying_key().as_bytes());
    if seller_public_key != entry.signed_offer.public_key_hex {
        return Err(MarketError::InvalidRecord(
            "quote signer does not own listing key",
        ));
    }
    if domain_separated_digest(
        "hyperai.compute-market.signed-offer.v1",
        &entry.signed_offer,
    )? != entry.offer_digest
    {
        return Err(MarketError::InvalidRecord("catalog offer digest mismatch"));
    }
    let offer = &entry.signed_offer.offer;
    if offer.resource_model != request.resource_model
        || offer.region != request.region
        || offer.capacity_units < request.units
        || offer.price_microusd_per_unit_hour > request.max_rate_microusd_per_unit_hour
    {
        return Err(MarketError::InvalidRecord("quote widens buyer constraints"));
    }
    let total = total_cost(
        offer.price_microusd_per_unit_hour,
        request.units,
        request.duration_hours,
    )?;
    if total > request.max_total_microusd {
        return Err(MarketError::InvalidRecord("quote exceeds buyer spend cap"));
    }
    let expires_at = if request.valid_until < offer.expires_at {
        request.valid_until.clone()
    } else {
        offer.expires_at.clone()
    };
    let quote = SellerQuoteV1 {
        schema_version: QUOTE_SCHEMA.to_owned(),
        offer_digest: entry.offer_digest,
        request_digest: domain_separated_digest("hyperai.compute-market.request.v1", request)?,
        seller_id: offer.seller_id.clone(),
        seller_public_key_hex: seller_public_key,
        rate_microusd_per_unit_hour: offer.price_microusd_per_unit_hour,
        units: request.units,
        duration_hours: request.duration_hours,
        quoted_total_microusd: total,
        amount_cap_microusd: request.max_total_microusd,
        expires_at,
    };
    let signature = seller_key.sign(&signing_message(
        "hyperai.compute-market.quote-signature.v1",
        &quote,
    )?);
    Ok(SignedSellerQuoteV1 {
        quote,
        signature_hex: HEXLOWER.encode(&signature.to_bytes()),
    })
}

/// Approve a seller quote using a buyer-owned Ed25519 key.
///
/// # Errors
///
/// Returns an error when the seller signature is invalid or the quote does not
/// match the buyer's request and acceptance deadline.
pub fn approve_quote(
    signed_quote: SignedSellerQuoteV1,
    buyer_request: BuyerRequestV1,
    accepted_at: UtcTimestamp,
    buyer_key: &SigningKey,
) -> Result<BuyerApprovedDealV1, MarketError> {
    verify_quote(&signed_quote)?;
    validate_request(&buyer_request, &accepted_at)?;
    let buyer_id = buyer_request.buyer_id.clone();
    let request_digest =
        domain_separated_digest("hyperai.compute-market.request.v1", &buyer_request)?;
    if accepted_at >= signed_quote.quote.expires_at
        || signed_quote.quote.request_digest != request_digest
        || signed_quote.quote.rate_microusd_per_unit_hour
            > buyer_request.max_rate_microusd_per_unit_hour
        || signed_quote.quote.units != buyer_request.units
        || signed_quote.quote.duration_hours != buyer_request.duration_hours
        || signed_quote.quote.quoted_total_microusd > buyer_request.max_total_microusd
        || signed_quote.quote.amount_cap_microusd > buyer_request.max_total_microusd
        || signed_quote.quote.expires_at > buyer_request.valid_until
    {
        return Err(MarketError::InvalidRecord(
            "buyer identity or quote deadline",
        ));
    }
    let buyer_public_key_hex = HEXLOWER.encode(buyer_key.verifying_key().as_bytes());
    let approval = BuyerApprovalV1 {
        schema_version: DEAL_SCHEMA.to_owned(),
        quote_digest: domain_separated_digest(
            "hyperai.compute-market.signed-quote.v1",
            &signed_quote,
        )?,
        buyer_id: buyer_id.clone(),
        buyer_public_key_hex: buyer_public_key_hex.clone(),
        request_digest,
        accepted_at: accepted_at.clone(),
    };
    let signature = buyer_key.sign(&signing_message(
        "hyperai.compute-market.buyer-approval.v1",
        &approval,
    )?);
    Ok(BuyerApprovedDealV1 {
        schema_version: DEAL_SCHEMA.to_owned(),
        signed_quote,
        buyer_request,
        buyer_id,
        buyer_public_key_hex,
        accepted_at,
        signature_hex: HEXLOWER.encode(&signature.to_bytes()),
    })
}

/// Build a seller-signed fulfillment receipt bound to the approved deal.
///
/// # Errors
///
/// Returns an error for invalid deal signatures, a seller-key mismatch, a
/// completion before acceptance, or an amount over the buyer's cap.
pub fn create_fulfillment_receipt(
    deal: &BuyerApprovedDealV1,
    billed_microusd: u64,
    completed_at: UtcTimestamp,
    result_digest: Sha256Digest,
    seller_key: &SigningKey,
) -> Result<FulfillmentReceiptV1, MarketError> {
    verify_approved_deal(deal)?;
    let seller_public_key_hex = HEXLOWER.encode(seller_key.verifying_key().as_bytes());
    if seller_public_key_hex != deal.signed_quote.quote.seller_public_key_hex
        || completed_at < deal.accepted_at
        || billed_microusd > deal.signed_quote.quote.amount_cap_microusd
        || billed_microusd > deal.signed_quote.quote.quoted_total_microusd
    {
        return Err(MarketError::InvalidRecord(
            "fulfillment signer, time, or spend cap",
        ));
    }
    let mut receipt = FulfillmentReceiptV1 {
        schema_version: FULFILLMENT_SCHEMA.to_owned(),
        deal_digest: deal_digest(deal)?,
        seller_id: deal.signed_quote.quote.seller_id.clone(),
        seller_public_key_hex,
        billed_microusd,
        completed_at,
        result_digest,
        signature_hex: String::new(),
    };
    receipt.signature_hex = HEXLOWER.encode(
        &seller_key
            .sign(&signing_message(
                "hyperai.compute-market.fulfillment-signature.v1",
                &receipt_payload(&receipt),
            )?)
            .to_bytes(),
    );
    Ok(receipt)
}

/// Build a seller-signed failure statement for one approved-deal attempt.
///
/// The statement does not determine whether any amount is owed or refunded.
///
/// # Errors
///
/// Returns an error for an invalid deal, seller-key mismatch, invalid attempt
/// ID, or timestamps outside the approved-deal lifecycle.
pub fn create_fulfillment_failure_receipt(
    deal: &BuyerApprovedDealV1,
    attempt_id: String,
    started_at: UtcTimestamp,
    failed_at: UtcTimestamp,
    failure_code: FulfillmentFailureCodeV1,
    seller_key: &SigningKey,
) -> Result<FulfillmentFailureReceiptV1, MarketError> {
    verify_approved_deal(deal)?;
    let seller_public_key_hex = HEXLOWER.encode(seller_key.verifying_key().as_bytes());
    if seller_public_key_hex != deal.signed_quote.quote.seller_public_key_hex
        || !valid_fulfillment_attempt_id(&attempt_id)
        || started_at < deal.accepted_at
        || failed_at < started_at
    {
        return Err(MarketError::InvalidRecord(
            "fulfillment failure signer, attempt ID, or time",
        ));
    }
    let mut failure = FulfillmentFailureReceiptV1 {
        schema_version: FULFILLMENT_FAILURE_SCHEMA.to_owned(),
        deal_digest: deal_digest(deal)?,
        seller_id: deal.signed_quote.quote.seller_id.clone(),
        seller_public_key_hex,
        attempt_id,
        started_at,
        failed_at,
        failure_code,
        signature_hex: String::new(),
    };
    failure.signature_hex = HEXLOWER.encode(
        &seller_key
            .sign(&signing_message(
                "hyperai.compute-market.fulfillment-failure-signature.v1",
                &fulfillment_failure_payload(&failure),
            )?)
            .to_bytes(),
    );
    verify_fulfillment_failure_receipt(deal, &failure)?;
    Ok(failure)
}

/// Return the digest used to identify a complete seller failure record.
///
/// # Errors
///
/// Returns an error if canonical serialization fails.
pub fn fulfillment_failure_digest_v1(
    failure: &FulfillmentFailureReceiptV1,
) -> Result<Sha256Digest, MarketError> {
    domain_separated_digest(
        "hyperai.compute-market.fulfillment-failure-record.v1",
        failure,
    )
    .map_err(Into::into)
}

/// Bind a local mock verifier verdict to exact deal and receipt digests.
///
/// # Errors
///
/// Returns an error when deal or receipt validation fails or the synthetic
/// reason code is empty.
pub fn mock_verifier_decision(
    deal: &BuyerApprovedDealV1,
    receipt: &FulfillmentReceiptV1,
    verdict: CompletionVerdict,
    reason_code: String,
) -> Result<MockVerifierDecisionV1, MarketError> {
    let deal_digest = deal_digest(deal)?;
    verify_fulfillment_receipt(deal, receipt)?;
    if reason_code.is_empty() {
        return Err(MarketError::InvalidRecord("empty mock verifier reason"));
    }
    Ok(MockVerifierDecisionV1 {
        schema_version: VERIFIER_SCHEMA.to_owned(),
        deal_digest,
        receipt_digest: domain_separated_digest("hyperai.compute-market.fulfillment.v1", receipt)?,
        verdict,
        reason_code,
    })
}

/// In-memory ledger for local synthetic settlement disposition only.
#[derive(Default)]
pub struct MockSettlementLedger {
    settled_deals: HashSet<String>,
}

impl MockSettlementLedger {
    /// Record one disposition after verifying all deal-bound signatures.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid deal, receipt, or decision, or when the
    /// same deal has already been recorded by this in-memory ledger.
    pub fn settle(
        &mut self,
        deal: &BuyerApprovedDealV1,
        receipt: &FulfillmentReceiptV1,
        decision: &MockVerifierDecisionV1,
        recorded_at: UtcTimestamp,
    ) -> Result<MockSettlementReceiptV1, MarketError> {
        verify_approved_deal(deal)?;
        verify_fulfillment_receipt(deal, receipt)?;
        let deal_digest = deal_digest(deal)?;
        let fulfillment_digest =
            domain_separated_digest("hyperai.compute-market.fulfillment.v1", receipt)?;
        if decision.schema_version != VERIFIER_SCHEMA
            || decision.deal_digest != deal_digest
            || decision.receipt_digest != fulfillment_digest
        {
            return Err(MarketError::InvalidRecord(
                "verifier decision is not deal-bound",
            ));
        }
        let key = deal_digest.to_string();
        if self.settled_deals.contains(&key) {
            return Err(MarketError::Replay);
        }
        let disposition = disposition_for(&decision.verdict);
        let decision_digest =
            domain_separated_digest("hyperai.compute-market.mock-decision.v1", decision)?;
        let record = MockSettlementReceiptV1 {
            schema_version: SETTLEMENT_SCHEMA.to_owned(),
            deal_digest,
            fulfillment_digest,
            decision_digest,
            disposition,
            recorded_at,
        };
        self.settled_deals.insert(key);
        Ok(record)
    }
}

struct LocalMarketQuoteFixture {
    seller_key: SigningKey,
    buyer_key: SigningKey,
    now: UtcTimestamp,
    catalog: CatalogSnapshotV1,
    buyer_request: BuyerRequestV1,
}

fn local_market_quote_fixture_for_scenario(
    scenario: LocalMarketQuoteScenarioV1,
) -> Result<LocalMarketQuoteFixture, MarketError> {
    let now: UtcTimestamp = LOCAL_FIXTURE_TIME.parse()?;
    let seller_key = SigningKey::from_bytes(&[41; 32]);
    let buyer_key = SigningKey::from_bytes(&[42; 32]);
    let offer_a = sample_signed_offer(&seller_key, "offer-gpu-a", 2_000_000, &now)?;
    let offer_b = sample_signed_offer(&seller_key, "offer-gpu-b", 1_800_000, &now)?;
    let catalog = build_catalog(
        vec![
            ("synthetic-registry-alpha".to_owned(), offer_a.clone()),
            ("synthetic-registry-beta".to_owned(), offer_a),
            ("synthetic-registry-beta".to_owned(), offer_b),
        ],
        now.clone(),
    )?;
    let mut buyer_request = BuyerRequestV1 {
        schema_version: REQUEST_SCHEMA.to_owned(),
        buyer_id: "synthetic-buyer".to_owned(),
        resource_model: "GPU-H100".to_owned(),
        region: "us-east".to_owned(),
        units: 2,
        duration_hours: 3,
        max_rate_microusd_per_unit_hour: 2_500_000,
        max_total_microusd: 15_000_000,
        requested_at: now.clone(),
        valid_until: "2026-09-24T12:20:00Z".parse()?,
    };
    if scenario == LocalMarketQuoteScenarioV1::Alternate {
        "synthetic-buyer-alternate".clone_into(&mut buyer_request.buyer_id);
        buyer_request.units = 1;
        buyer_request.duration_hours = 2;
        buyer_request.max_total_microusd = 5_000_000;
    }
    Ok(LocalMarketQuoteFixture {
        seller_key,
        buyer_key,
        now,
        catalog,
        buyer_request,
    })
}

fn local_market_quote_fixture() -> Result<LocalMarketQuoteFixture, MarketError> {
    local_market_quote_fixture_for_scenario(LocalMarketQuoteScenarioV1::Baseline)
}

/// Build the fixed local buyer supply comparison without quote or deal work.
///
/// The comparison is a read-only projection over the signed synthetic catalog
/// and a separate advisory observation set. It does not create quotes,
/// approvals, fulfillment, verifier decisions, receipts, or settlement.
///
/// # Errors
///
/// Returns an error if the synthetic request, catalog, observations, or
/// digest-bound projection fails contract validation.
pub fn local_market_supply_comparison_v1() -> Result<SupplyComparisonV1, MarketError> {
    let fixture = local_market_quote_fixture()?;
    let observations = local_market_advisory_observations(&fixture)?;
    build_supply_comparison(
        fixture.catalog,
        fixture.buyer_request,
        fixture.now,
        observations,
    )
}

/// Verify the source records, derived eligibility, selection, and body digest.
///
/// # Errors
///
/// Returns an error if any nested source record or derived field is invalid.
pub fn verify_supply_comparison_v1(comparison: &SupplyComparisonV1) -> Result<(), MarketError> {
    let body = &comparison.body;
    if body.schema_version != SUPPLY_COMPARISON_SCHEMA
        || body.state_slice != SUPPLY_COMPARISON_SLICE
        || body.freshness_status != "not_evaluated"
    {
        return Err(MarketError::InvalidRecord(
            "supply comparison schema or state",
        ));
    }
    let rebuilt = build_supply_comparison(
        body.catalog.clone(),
        body.buyer_request.clone(),
        body.compared_at.clone(),
        body.advisory_observations.clone(),
    )?;
    if rebuilt != *comparison {
        return Err(MarketError::InvalidRecord(
            "supply comparison derived fields or digest",
        ));
    }
    Ok(())
}

fn build_supply_comparison(
    catalog: CatalogSnapshotV1,
    buyer_request: BuyerRequestV1,
    compared_at: UtcTimestamp,
    mut advisory_observations: Vec<MarketObservationV1>,
) -> Result<SupplyComparisonV1, MarketError> {
    validate_request(&buyer_request, &compared_at)?;
    verify_catalog(&catalog, &compared_at)?;
    for observation in &advisory_observations {
        verify_market_observation(observation, &catalog)?;
    }
    advisory_observations.sort_by(|left, right| {
        (&left.body.offer_id, &left.body.source_id)
            .cmp(&(&right.body.offer_id, &right.body.source_id))
    });

    let selected_offer_id = match select_offer(&catalog, &buyer_request, &compared_at) {
        Ok(entry) => Some(entry.signed_offer.offer.offer_id.clone()),
        Err(MarketError::NoEligibleOffer) => None,
        Err(error) => return Err(error),
    };
    let offer_assessments = catalog
        .body
        .entries
        .iter()
        .map(|entry| {
            let mut assessment = assess_offer_for_request(entry, &buyer_request);
            assessment.selected = selected_offer_id.as_deref() == Some(&assessment.offer_id);
            assessment
        })
        .collect();

    let body = SupplyComparisonBodyV1 {
        schema_version: SUPPLY_COMPARISON_SCHEMA.to_owned(),
        state_slice: SUPPLY_COMPARISON_SLICE.to_owned(),
        catalog,
        buyer_request,
        compared_at,
        selected_offer_id,
        offer_assessments,
        advisory_observations,
        freshness_status: "not_evaluated".to_owned(),
        claim_ceiling: "deterministic eligibility over the fixed local synthetic signed catalog only".to_owned(),
        nonclaims: "no live availability, seller quote, offer performance, compute execution, correct computation, payment, settlement, independent verification, or source freshness claim".to_owned(),
        effects: "read-only in-memory synthetic projection; no quote, approval, fulfillment, verifier, receipt, settlement, network, provider, file, or funds effect".to_owned(),
    };
    let digest = domain_separated_digest("hyperai.compute-market.supply-comparison.v1", &body)?;
    Ok(SupplyComparisonV1 { body, digest })
}

fn assess_offer_for_request(
    entry: &CatalogEntryV1,
    request: &BuyerRequestV1,
) -> SupplyOfferAssessmentV1 {
    let offer = &entry.signed_offer.offer;
    let mut ineligibility_reasons = Vec::new();
    if offer.resource_model != request.resource_model {
        ineligibility_reasons.push(OfferIneligibilityReasonV1::ResourceModelMismatch);
    }
    if offer.region != request.region {
        ineligibility_reasons.push(OfferIneligibilityReasonV1::RegionMismatch);
    }
    if offer.capacity_units < request.units {
        ineligibility_reasons.push(OfferIneligibilityReasonV1::InsufficientCapacity);
    }
    if offer.price_microusd_per_unit_hour > request.max_rate_microusd_per_unit_hour {
        ineligibility_reasons.push(OfferIneligibilityReasonV1::AskingRateExceedsCap);
    }
    let asking_total_microusd = offer
        .price_microusd_per_unit_hour
        .checked_mul(u64::from(request.units))
        .and_then(|subtotal| subtotal.checked_mul(u64::from(request.duration_hours)));
    if asking_total_microusd.is_none() {
        ineligibility_reasons.push(OfferIneligibilityReasonV1::AskingTotalOverflow);
    }
    if asking_total_microusd.is_some_and(|total| total > request.max_total_microusd) {
        ineligibility_reasons.push(OfferIneligibilityReasonV1::AskingTotalExceedsCap);
    }
    SupplyOfferAssessmentV1 {
        offer_id: offer.offer_id.clone(),
        offer_digest: entry.offer_digest,
        source_ids: entry.source_ids.clone(),
        asking_rate_microusd_per_unit_hour: offer.price_microusd_per_unit_hour,
        asking_total_microusd,
        eligible: ineligibility_reasons.is_empty(),
        ineligibility_reasons,
        selected: false,
    }
}

fn local_market_advisory_observations(
    fixture: &LocalMarketQuoteFixture,
) -> Result<Vec<MarketObservationV1>, MarketError> {
    let mut observations = vec![
        build_market_observation(MarketObservationBodyV1 {
            schema_version: OBSERVATION_SCHEMA.to_owned(),
            source_id: "synthetic-market-observation-feed".to_owned(),
            offer_id: "offer-gpu-a".to_owned(),
            observed_at: fixture.now.clone(),
            estimated_rate_microusd_per_unit_hour: Some(2_100_000),
            estimated_available_units: Some(7),
        })?,
        build_market_observation(MarketObservationBodyV1 {
            schema_version: OBSERVATION_SCHEMA.to_owned(),
            source_id: "synthetic-market-observation-feed".to_owned(),
            offer_id: "offer-gpu-b".to_owned(),
            observed_at: fixture.now.clone(),
            // This lower modeled estimate remains advisory; the firm listing
            // and seller quote keep the actual offered rate at 1,800,000.
            estimated_rate_microusd_per_unit_hour: Some(1_600_000),
            estimated_available_units: Some(4),
        })?,
    ];
    observations.sort_by(|left, right| left.body.offer_id.cmp(&right.body.offer_id));
    Ok(observations)
}

fn local_market_quote_preview_from_fixture(
    fixture: &LocalMarketQuoteFixture,
) -> Result<LocalMarketQuotePreviewV1, MarketError> {
    let selected = select_offer(&fixture.catalog, &fixture.buyer_request, &fixture.now)?;
    let signed_quote = create_quote(
        selected,
        &fixture.buyer_request,
        &fixture.seller_key,
        &fixture.now,
    )?;
    verify_quote(&signed_quote)?;
    let body = LocalMarketQuotePreviewBodyV1 {
        schema_version: LOCAL_QUOTE_PREVIEW_SCHEMA.to_owned(),
        state_slice: LOCAL_QUOTE_REVIEW_SLICE.to_owned(),
        catalog: fixture.catalog.clone(),
        buyer_request: fixture.buyer_request.clone(),
        selected_offer_id: selected.signed_offer.offer.offer_id.clone(),
        signed_quote,
        simulated_at: fixture.now.clone(),
        claim_ceiling: "synthetic quote terms and market contract integrity only".to_owned(),
        nonclaims: "no real buyer identity, seller identity, availability, fulfillment, correct computation, payment, or independent verification".to_owned(),
        effects: "synthetic in-memory preview; no seller, provider, network, wallet, chain, file, or funds effect".to_owned(),
    };
    let digest = domain_separated_digest("hyperai.compute-market.local-quote-preview.v1", &body)?;
    Ok(LocalMarketQuotePreviewV1 { body, digest })
}

/// Build the exact local synthetic request and seller quote for review.
///
/// The returned timestamp is a simulated fixture clock. No network request,
/// seller negotiation, approval, fulfillment, or settlement occurs here.
///
/// # Errors
///
/// Returns an error if the synthetic catalog, request, signature, or digest
/// fails the existing market contract.
pub fn local_market_quote_preview() -> Result<LocalMarketQuotePreviewV1, MarketError> {
    let fixture = local_market_quote_fixture()?;
    local_market_quote_preview_from_fixture(&fixture)
}

/// Build one declared fixed synthetic quote scenario.
///
/// # Errors
///
/// Returns a market contract error if fixture reconstruction or quote
/// validation fails.
pub fn local_market_quote_preview_for_scenario(
    scenario: LocalMarketQuoteScenarioV1,
) -> Result<LocalMarketQuotePreviewV1, MarketError> {
    let fixture = local_market_quote_fixture_for_scenario(scenario)?;
    local_market_quote_preview_from_fixture(&fixture)
}

fn local_supply_quote_preview_from_fixture(
    fixture: &LocalMarketQuoteFixture,
    expected_comparison_digest: Sha256Digest,
) -> Result<SupplyQuotePreviewV1, MarketError> {
    let comparison = build_supply_comparison(
        fixture.catalog.clone(),
        fixture.buyer_request.clone(),
        fixture.now.clone(),
        local_market_advisory_observations(fixture)?,
    )?;
    verify_supply_comparison_v1(&comparison)?;
    if comparison.digest != expected_comparison_digest {
        return Err(MarketError::SupplyComparisonMismatch);
    }

    let selected_offer_id = comparison
        .body
        .selected_offer_id
        .as_ref()
        .ok_or(MarketError::SupplyQuoteNoEligibleOffer)?;
    let assessment = comparison
        .body
        .offer_assessments
        .iter()
        .find(|offer| offer.offer_id == *selected_offer_id && offer.selected)
        .ok_or(MarketError::InvalidRecord(
            "selected supply offer assessment is missing",
        ))?;
    let catalog_entry = comparison
        .body
        .catalog
        .body
        .entries
        .iter()
        .find(|entry| entry.signed_offer.offer.offer_id == *selected_offer_id)
        .ok_or(MarketError::InvalidRecord(
            "selected supply offer is missing from catalog",
        ))?;
    if catalog_entry.offer_digest != assessment.offer_digest {
        return Err(MarketError::InvalidRecord(
            "selected supply offer digest differs from catalog",
        ));
    }

    let quote_preview = local_market_quote_preview_from_fixture(fixture)?;
    let buyer_request_digest = domain_separated_digest(
        "hyperai.compute-market.request.v1",
        &comparison.body.buyer_request,
    )?;
    let quote = &quote_preview.body.signed_quote.quote;
    if quote_preview.body.catalog.digest != comparison.body.catalog.digest
        || quote_preview.body.buyer_request != comparison.body.buyer_request
        || quote_preview.body.selected_offer_id != *selected_offer_id
        || quote.offer_digest != assessment.offer_digest
        || quote.request_digest != buyer_request_digest
    {
        return Err(MarketError::InvalidRecord(
            "supply comparison and quote preview differ",
        ));
    }

    let body = SupplyQuotePreviewBodyV1 {
        schema_version: SUPPLY_QUOTE_PREVIEW_SCHEMA.to_owned(),
        state_slice: SUPPLY_QUOTE_PREVIEW_SLICE.to_owned(),
        comparison_digest: comparison.digest,
        catalog_digest: comparison.body.catalog.digest,
        buyer_request_digest,
        selected_offer_id: selected_offer_id.clone(),
        selected_offer_digest: assessment.offer_digest,
        quote_preview,
        claim_ceiling: "fixed local synthetic supply-to-quote contract consistency only".to_owned(),
        nonclaims: "no live availability, seller intent, external quote validity, fulfillment, correct computation, payment, settlement, or independent verification".to_owned(),
        effects: "read-only in-memory binding of the fixed synthetic comparison and quote preview; no seller, provider, network, storage, approval, fulfillment, or funds effect".to_owned(),
    };
    let digest = domain_separated_digest("hyperai.compute-market.supply-quote-preview.v1", &body)?;
    Ok(SupplyQuotePreviewV1 { body, digest })
}

/// Build the existing synthetic quote preview only for the exact displayed
/// buyer supply comparison.
///
/// This operation is read-only. It rebuilds both projections from one local
/// fixture and checks their catalog, request, selected offer, and offer-digest
/// bindings before returning the new wrapper.
///
/// # Errors
///
/// Returns `SupplyComparisonMismatch` for a changed comparison digest,
/// `SupplyQuoteNoEligibleOffer` when the request has no eligible offer, or a
/// contract error when any nested source or quote binding is invalid.
pub fn local_market_supply_quote_preview_v1(
    expected_comparison_digest: Sha256Digest,
) -> Result<SupplyQuotePreviewV1, MarketError> {
    let fixture = local_market_quote_fixture()?;
    local_supply_quote_preview_from_fixture(&fixture, expected_comparison_digest)
}

/// Verify a supply-to-quote binding against the current fixed local fixture.
///
/// # Errors
///
/// Returns an error when the comparison digest, nested quote, selected offer,
/// or full wrapper digest differs from the deterministic fixture.
pub fn verify_supply_quote_preview_v1(preview: &SupplyQuotePreviewV1) -> Result<(), MarketError> {
    let rebuilt = local_market_supply_quote_preview_v1(preview.body.comparison_digest)?;
    if rebuilt != *preview {
        return Err(MarketError::InvalidRecord(
            "supply quote preview digest or binding differs",
        ));
    }
    Ok(())
}

fn local_market_quote_approval_from_fixture(
    fixture: &LocalMarketQuoteFixture,
    preview: &LocalMarketQuotePreviewV1,
) -> Result<LocalMarketQuoteApprovalV1, MarketError> {
    let accepted_at: UtcTimestamp = LOCAL_FIXTURE_ACCEPTANCE_TIME.parse()?;
    let approved_deal = approve_quote(
        preview.body.signed_quote.clone(),
        preview.body.buyer_request.clone(),
        accepted_at,
        &fixture.buyer_key,
    )?;
    verify_approved_deal(&approved_deal)?;
    Ok(LocalMarketQuoteApprovalV1 {
        schema_version: LOCAL_QUOTE_APPROVAL_SCHEMA.to_owned(),
        state_slice: LOCAL_QUOTE_REVIEW_SLICE.to_owned(),
        preview_digest: preview.digest,
        deal_digest: deal_digest(&approved_deal)?,
        approved_deal,
        claim_ceiling: "synthetic buyer approval contract integrity only".to_owned(),
        nonclaims: "not a real buyer signature or purchase authorization; no fulfillment, correct computation, payment, independent verification, or settlement".to_owned(),
        effects: "synthetic in-memory approval only; no fulfillment, verifier, settlement, network, wallet, chain, file, or funds effect".to_owned(),
    })
}

/// Approve one displayed local quote after checking its full preview digest.
///
/// This uses the deterministic synthetic buyer key and fixture clock. It is a
/// contract demonstration, not a real user's signing authority.
///
/// # Errors
///
/// Returns [`MarketError::QuotePreviewMismatch`] if the supplied digest does
/// not match the recomputed preview, or a contract error if approval fails.
pub fn approve_local_market_quote_preview(
    expected_preview_digest: Sha256Digest,
) -> Result<LocalMarketQuoteApprovalV1, MarketError> {
    approve_local_market_quote_preview_for_scenario(
        expected_preview_digest,
        LocalMarketQuoteScenarioV1::Baseline,
    )
}

/// Approve one exact declared scenario after rebuilding its full preview.
///
/// # Errors
///
/// Returns [`MarketError::QuotePreviewMismatch`] when the digest does not
/// match the selected scenario.
pub fn approve_local_market_quote_preview_for_scenario(
    expected_preview_digest: Sha256Digest,
    scenario: LocalMarketQuoteScenarioV1,
) -> Result<LocalMarketQuoteApprovalV1, MarketError> {
    let fixture = local_market_quote_fixture_for_scenario(scenario)?;
    let preview = local_market_quote_preview_from_fixture(&fixture)?;
    if expected_preview_digest != preview.digest {
        return Err(MarketError::QuotePreviewMismatch);
    }
    local_market_quote_approval_from_fixture(&fixture, &preview)
}

/// Approve a preview only when it exactly matches one member of the closed
/// local synthetic scenario set.
///
/// # Errors
///
/// Returns [`MarketError::QuotePreviewMismatch`] for unknown or modified
/// preview digests.
pub fn approve_local_market_quote_preview_for_known_scenarios(
    expected_preview_digest: Sha256Digest,
) -> Result<LocalMarketQuoteApprovalV1, MarketError> {
    for scenario in LocalMarketQuoteScenarioV1::ALL {
        let fixture = local_market_quote_fixture_for_scenario(scenario)?;
        let preview = local_market_quote_preview_from_fixture(&fixture)?;
        if expected_preview_digest == preview.digest {
            return local_market_quote_approval_from_fixture(&fixture, &preview);
        }
    }
    Err(MarketError::QuotePreviewMismatch)
}

/// Continue the exact local quote preview and approval into a mock lifecycle.
///
/// Both digests are checked against a freshly rebuilt deterministic fixture
/// before any fulfillment, verifier, or settlement record is created.
///
/// # Errors
///
/// Returns a digest mismatch error if the quote or approval differs, or a
/// market contract error if local receipt construction or verification fails.
pub fn complete_local_market_quote_preview(
    expected_preview_digest: Sha256Digest,
    expected_deal_digest: Sha256Digest,
) -> Result<LocalMarketReceiptV1, MarketError> {
    complete_local_market_quote_preview_for_scenario(
        expected_preview_digest,
        expected_deal_digest,
        LocalMarketQuoteScenarioV1::Baseline,
    )
}

/// Continue one exact declared scenario into its deterministic mock receipt.
///
/// # Errors
///
/// Returns a digest mismatch error when the preview or approval differs.
pub fn complete_local_market_quote_preview_for_scenario(
    expected_preview_digest: Sha256Digest,
    expected_deal_digest: Sha256Digest,
    scenario: LocalMarketQuoteScenarioV1,
) -> Result<LocalMarketReceiptV1, MarketError> {
    let fixture = local_market_quote_fixture_for_scenario(scenario)?;
    let preview = local_market_quote_preview_from_fixture(&fixture)?;
    if expected_preview_digest != preview.digest {
        return Err(MarketError::QuotePreviewMismatch);
    }
    let approval = local_market_quote_approval_from_fixture(&fixture, &preview)?;
    if expected_deal_digest != approval.deal_digest {
        return Err(MarketError::QuoteApprovalMismatch);
    }
    local_market_receipt_for_approved_deal(&fixture, &preview, approval.approved_deal)
}

/// Complete a deal only when both digests match one closed local scenario.
///
/// # Errors
///
/// Returns a digest mismatch error for unknown or modified preview/deal pairs.
pub fn complete_local_market_quote_preview_for_known_scenarios(
    expected_preview_digest: Sha256Digest,
    expected_deal_digest: Sha256Digest,
) -> Result<LocalMarketReceiptV1, MarketError> {
    for scenario in LocalMarketQuoteScenarioV1::ALL {
        let fixture = local_market_quote_fixture_for_scenario(scenario)?;
        let preview = local_market_quote_preview_from_fixture(&fixture)?;
        if expected_preview_digest != preview.digest {
            continue;
        }
        let approval = local_market_quote_approval_from_fixture(&fixture, &preview)?;
        if expected_deal_digest != approval.deal_digest {
            return Err(MarketError::QuoteApprovalMismatch);
        }
        return local_market_receipt_for_approved_deal(&fixture, &preview, approval.approved_deal);
    }
    Err(MarketError::QuotePreviewMismatch)
}

/// Build one fixed seller-reported failure for an explicitly checked local quote.
///
/// This is a deterministic fixture path only. It does not start work, record
/// charges, determine a refund, or move funds.
///
/// # Errors
///
/// Returns a digest mismatch error if the quote preview or approved deal has
/// changed, or a contract error if failure-record construction fails.
pub fn fail_local_market_quote_preview(
    expected_preview_digest: Sha256Digest,
    expected_deal_digest: Sha256Digest,
) -> Result<FulfillmentFailureReceiptV1, MarketError> {
    let fixture = local_market_quote_fixture()?;
    let preview = local_market_quote_preview_from_fixture(&fixture)?;
    if expected_preview_digest != preview.digest {
        return Err(MarketError::QuotePreviewMismatch);
    }
    let approval = local_market_quote_approval_from_fixture(&fixture, &preview)?;
    if expected_deal_digest != approval.deal_digest {
        return Err(MarketError::QuoteApprovalMismatch);
    }
    create_fulfillment_failure_receipt(
        &approval.approved_deal,
        "synthetic-attempt-1".to_owned(),
        "2026-09-24T12:06:00Z".parse()?,
        "2026-09-24T12:07:00Z".parse()?,
        FulfillmentFailureCodeV1::ResourceUnavailable,
        &fixture.seller_key,
    )
}

fn local_market_receipt_for_approved_deal(
    fixture: &LocalMarketQuoteFixture,
    preview: &LocalMarketQuotePreviewV1,
    deal: BuyerApprovedDealV1,
) -> Result<LocalMarketReceiptV1, MarketError> {
    verify_approved_deal(&deal)?;
    let advisory_observations = local_market_advisory_observations(fixture)?;

    let seller_key = &fixture.seller_key;
    let billed_microusd = deal.signed_quote.quote.quoted_total_microusd;
    let fulfillment = create_fulfillment_receipt(
        &deal,
        billed_microusd,
        "2026-09-24T15:00:00Z".parse()?,
        Sha256Digest::from_bytes([0x5a; 32]),
        seller_key,
    )?;
    let decision = mock_verifier_decision(
        &deal,
        &fulfillment,
        CompletionVerdict::Verified,
        "synthetic_fixture_pass".to_owned(),
    )?;
    let mut ledger = MockSettlementLedger::default();
    let settlement = ledger.settle(
        &deal,
        &fulfillment,
        &decision,
        "2026-09-24T15:01:00Z".parse()?,
    )?;
    let body = LocalMarketReceiptBodyV1 {
        schema_version: LOCAL_RECEIPT_SCHEMA.to_owned(),
        state_slice: LOCAL_ACCEPTANCE_SLICE.to_owned(),
        catalog: preview.body.catalog.clone(),
        advisory_observations,
        buyer_request: deal.buyer_request.clone(),
        selected_offer_id: preview.body.selected_offer_id.clone(),
        approved_deal: deal,
        fulfillment,
        verifier_decision: decision,
        mock_settlement: settlement,
        claim_ceiling: LOCAL_CLAIM_CEILING.to_owned(),
        nonclaims: LOCAL_NONCLAIMS.to_owned(),
        effects: LOCAL_EFFECTS.to_owned(),
    };
    let receipt = LocalMarketReceiptV1 {
        digest: domain_separated_digest("hyperai.compute-market.local-receipt.v1", &body)?,
        body,
    };
    verify_local_market_receipt(&receipt)?;
    Ok(receipt)
}

/// Run the complete synthetic catalog-to-mock-settlement path and return its receipt.
///
/// The two registry labels and market observations are fixed synthetic data.
/// Market observations are carried into the receipt but are not used to select
/// offers, set seller terms, or map the mock settlement decision.
///
/// # Errors
///
/// Returns an error if any locally constructed contract fails validation.
pub fn local_market_receipt() -> Result<LocalMarketReceiptV1, MarketError> {
    let fixture = local_market_quote_fixture()?;
    let preview = local_market_quote_preview_from_fixture(&fixture)?;
    let approval = local_market_quote_approval_from_fixture(&fixture, &preview)?;
    local_market_receipt_for_approved_deal(&fixture, &preview, approval.approved_deal)
}

/// Verify all nested contracts and digests in a local synthetic receipt.
///
/// This establishes structural integrity and signature consistency only. It
/// does not establish that any seller, verifier, or market observation is
/// independently trusted.
///
/// # Errors
///
/// Returns an error for any digest, signature, source, cap, lifecycle, or
/// disposition mismatch.
pub fn verify_local_market_receipt(receipt: &LocalMarketReceiptV1) -> Result<(), MarketError> {
    let body = &receipt.body;
    if body.schema_version != LOCAL_RECEIPT_SCHEMA
        || body.state_slice != LOCAL_ACCEPTANCE_SLICE
        || body.claim_ceiling != LOCAL_CLAIM_CEILING
        || body.nonclaims != LOCAL_NONCLAIMS
        || body.effects != LOCAL_EFFECTS
        || domain_separated_digest("hyperai.compute-market.local-receipt.v1", body)?
            != receipt.digest
    {
        return Err(MarketError::InvalidRecord(
            "local receipt schema, claim boundary, or digest",
        ));
    }

    verify_catalog(&body.catalog, &body.catalog.body.as_of)?;
    validate_request(&body.buyer_request, &body.catalog.body.as_of)?;
    if body.catalog.body.source_count < 2
        || body.catalog.body.duplicate_count == 0
        || body.advisory_observations.is_empty()
    {
        return Err(MarketError::InvalidRecord(
            "local receipt lacks multi-source synthetic provenance",
        ));
    }

    let mut previous_observation: Option<&str> = None;
    for observation in &body.advisory_observations {
        verify_market_observation(observation, &body.catalog)?;
        if previous_observation
            .is_some_and(|previous| previous >= observation.body.offer_id.as_str())
        {
            return Err(MarketError::InvalidRecord(
                "market observations are not uniquely ordered",
            ));
        }
        previous_observation = Some(&observation.body.offer_id);
    }

    verify_approved_deal(&body.approved_deal)?;
    let selected = select_offer(&body.catalog, &body.buyer_request, &body.catalog.body.as_of)?;
    let quote = &body.approved_deal.signed_quote.quote;
    if body.approved_deal.buyer_request != body.buyer_request
        || selected.signed_offer.offer.offer_id != body.selected_offer_id
        || quote.offer_digest != selected.offer_digest
        || quote.seller_id != selected.signed_offer.offer.seller_id
        || quote.seller_public_key_hex != selected.signed_offer.public_key_hex
        || quote.rate_microusd_per_unit_hour
            != selected.signed_offer.offer.price_microusd_per_unit_hour
    {
        return Err(MarketError::InvalidRecord(
            "approved deal differs from deterministic buyer selection",
        ));
    }

    verify_fulfillment_receipt(&body.approved_deal, &body.fulfillment)?;
    let deal_digest = deal_digest(&body.approved_deal)?;
    let fulfillment_digest =
        domain_separated_digest("hyperai.compute-market.fulfillment.v1", &body.fulfillment)?;
    let decision = &body.verifier_decision;
    if decision.schema_version != VERIFIER_SCHEMA
        || decision.deal_digest != deal_digest
        || decision.receipt_digest != fulfillment_digest
        || decision.reason_code.is_empty()
    {
        return Err(MarketError::InvalidRecord(
            "mock verifier decision does not bind the fulfillment",
        ));
    }

    let settlement = &body.mock_settlement;
    let expected_disposition = disposition_for(&decision.verdict);
    if settlement.schema_version != SETTLEMENT_SCHEMA
        || settlement.deal_digest != deal_digest
        || settlement.fulfillment_digest != fulfillment_digest
        || settlement.decision_digest
            != domain_separated_digest("hyperai.compute-market.mock-decision.v1", decision)?
        || settlement.disposition != expected_disposition
        || settlement.recorded_at < body.fulfillment.completed_at
    {
        return Err(MarketError::InvalidRecord(
            "mock settlement disposition is inconsistent",
        ));
    }
    Ok(())
}

/// Build a digest-bound journal containing exactly one verified synthetic deal.
///
/// # Errors
///
/// Returns an error if the receipt is invalid or canonical encoding fails.
pub fn local_market_journal(
    receipt: LocalMarketReceiptV1,
) -> Result<LocalMarketJournalV1, MarketError> {
    verify_local_market_receipt(&receipt)?;
    let body = LocalMarketJournalBodyV1 {
        schema_version: LOCAL_JOURNAL_SCHEMA.to_owned(),
        receipt,
    };
    Ok(LocalMarketJournalV1 {
        digest: domain_separated_digest("hyperai.compute-market.local-journal.v1", &body)?,
        body,
    })
}

/// Verify the digest and receipt held by a local mock journal.
///
/// # Errors
///
/// Returns an error for an invalid journal schema, digest, or nested receipt.
pub fn verify_local_market_journal(journal: &LocalMarketJournalV1) -> Result<(), MarketError> {
    if journal.body.schema_version != LOCAL_JOURNAL_SCHEMA
        || domain_separated_digest("hyperai.compute-market.local-journal.v1", &journal.body)?
            != journal.digest
    {
        return Err(MarketError::InvalidRecord("local journal schema or digest"));
    }
    verify_local_market_receipt(&journal.body.receipt)
}

/// Create a new output directory containing the receipt export and local journal.
///
/// The directory must not exist. If journal persistence succeeds but receipt
/// export fails, the journal is retained so the receipt can be recovered.
///
/// # Errors
///
/// Returns an error for an existing output path, invalid receipt, oversized
/// artifact, or filesystem failure.
pub fn write_local_market_run(
    output_dir: &Path,
    receipt: &LocalMarketReceiptV1,
) -> Result<LocalDemoReportV1, MarketError> {
    verify_local_market_receipt(receipt)?;
    fs::create_dir(output_dir).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            MarketError::ArtifactExists
        } else {
            MarketError::Io(error)
        }
    })?;

    let journal_path = output_dir.join("market-journal.json");
    let receipt_path = output_dir.join("market-receipt.json");
    record_local_market_receipt(&journal_path, receipt.clone())?;
    write_json_new(&receipt_path, receipt)?;
    Ok(local_demo_report(receipt, true))
}

/// Load and validate a bounded receipt artifact from a regular local file.
///
/// # Errors
///
/// Returns an error for a symbolic link, non-file, oversized, malformed, or
/// invalid artifact.
pub fn read_local_market_receipt(path: &Path) -> Result<LocalMarketReceiptV1, MarketError> {
    let receipt: LocalMarketReceiptV1 = read_json_bounded(path)?;
    verify_local_market_receipt(&receipt)?;
    Ok(receipt)
}

/// Record one receipt in a one-deal local journal.
///
/// An exact retry after reopening the journal is idempotent and returns false.
/// A conflicting receipt for the same deal fails with Replay.
///
/// # Errors
///
/// Returns an error for an invalid receipt, conflicting deal replay, unsafe
/// journal path, or filesystem failure.
pub fn record_local_market_receipt(
    journal_path: &Path,
    receipt: LocalMarketReceiptV1,
) -> Result<bool, MarketError> {
    verify_local_market_receipt(&receipt)?;
    match fs::symlink_metadata(journal_path) {
        Ok(metadata) => {
            if !metadata.file_type().is_file() {
                return Err(MarketError::UnsafeArtifactPath);
            }
            let existing = read_local_market_journal(journal_path)?;
            let existing_deal = existing.body.receipt.body.mock_settlement.deal_digest;
            let new_deal = receipt.body.mock_settlement.deal_digest;
            if existing_deal == new_deal && existing.body.receipt == receipt {
                return Ok(false);
            }
            if existing_deal == new_deal {
                return Err(MarketError::Replay);
            }
            Err(MarketError::InvalidRecord(
                "local journal is bounded to one deal",
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let journal = local_market_journal(receipt)?;
            write_json_new(journal_path, &journal)?;
            Ok(true)
        }
        Err(error) => Err(MarketError::Io(error)),
    }
}

/// Load and validate a bounded one-deal local mock journal.
///
/// # Errors
///
/// Returns an error for a symbolic link, non-file, oversized, malformed, or
/// invalid journal.
pub fn read_local_market_journal(path: &Path) -> Result<LocalMarketJournalV1, MarketError> {
    let journal: LocalMarketJournalV1 = read_json_bounded(path)?;
    verify_local_market_journal(&journal)?;
    Ok(journal)
}

/// Recover an offline receipt export from the verified local journal.
///
/// The output is created atomically without replacing an existing destination.
///
/// # Errors
///
/// Returns an error for an invalid journal, an existing destination, or local
/// filesystem failure.
pub fn recover_local_market_receipt(
    journal_path: &Path,
    output_path: &Path,
) -> Result<LocalMarketReceiptV1, MarketError> {
    let journal = read_local_market_journal(journal_path)?;
    write_json_new(output_path, &journal.body.receipt)?;
    Ok(journal.body.receipt)
}

/// Summarize a verified local receipt for the command-line demo.
#[must_use]
pub fn local_demo_report(
    receipt: &LocalMarketReceiptV1,
    artifacts_written: bool,
) -> LocalDemoReportV1 {
    LocalDemoReportV1 {
        schema_version: "hyperai.compute-market.local-demo.v1".to_owned(),
        catalog_digest: receipt.body.catalog.digest,
        source_count: receipt.body.catalog.body.source_count,
        duplicate_count: receipt.body.catalog.body.duplicate_count,
        selected_offer_id: receipt.body.selected_offer_id.clone(),
        deal_digest: receipt.body.mock_settlement.deal_digest,
        fulfillment_digest: receipt.body.mock_settlement.fulfillment_digest,
        settlement: receipt.body.mock_settlement.clone(),
        effects: LOCAL_EFFECTS.to_owned(),
        claim_ceiling: LOCAL_CLAIM_CEILING.to_owned(),
        receipt_digest: receipt.digest,
        receipt_file: artifacts_written.then(|| "market-receipt.json".to_owned()),
        journal_file: artifacts_written.then(|| "market-journal.json".to_owned()),
    }
}

/// Run the deterministic synthetic demo without writing local files.
///
/// # Errors
///
/// Returns an error if any locally constructed contract fails validation.
pub fn local_demo() -> Result<LocalDemoReportV1, MarketError> {
    let receipt = local_market_receipt()?;
    Ok(local_demo_report(&receipt, false))
}

fn build_market_observation(
    body: MarketObservationBodyV1,
) -> Result<MarketObservationV1, MarketError> {
    if body.schema_version != OBSERVATION_SCHEMA
        || body.source_id.is_empty()
        || body.offer_id.is_empty()
        || body.estimated_rate_microusd_per_unit_hour == Some(0)
    {
        return Err(MarketError::InvalidRecord("advisory observation fields"));
    }
    let digest = domain_separated_digest("hyperai.compute-market.advisory-observation.v1", &body)?;
    Ok(MarketObservationV1 { body, digest })
}

fn verify_market_observation(
    observation: &MarketObservationV1,
    catalog: &CatalogSnapshotV1,
) -> Result<(), MarketError> {
    let body = &observation.body;
    if body.schema_version != OBSERVATION_SCHEMA
        || body.source_id.is_empty()
        || body.offer_id.is_empty()
        || body.observed_at > catalog.body.as_of
        || body.estimated_rate_microusd_per_unit_hour == Some(0)
        || !catalog
            .body
            .entries
            .iter()
            .any(|entry| entry.signed_offer.offer.offer_id == body.offer_id)
        || domain_separated_digest("hyperai.compute-market.advisory-observation.v1", body)?
            != observation.digest
    {
        return Err(MarketError::InvalidRecord(
            "advisory observation schema, time, source, or digest",
        ));
    }
    Ok(())
}

fn disposition_for(verdict: &CompletionVerdict) -> MockDisposition {
    match verdict {
        CompletionVerdict::Verified => MockDisposition::ReleaseToSeller,
        CompletionVerdict::Rejected => MockDisposition::RefundToBuyer,
        CompletionVerdict::Inconclusive => MockDisposition::HoldForDispute,
    }
}

fn read_json_bounded<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, MarketError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(MarketError::UnsafeArtifactPath);
    }
    if metadata.len() > MAX_ARTIFACT_BYTES as u64 {
        return Err(MarketError::ArtifactTooLarge);
    }
    let file = File::open(path)?;
    let mut bytes = Vec::new();
    file.take((MAX_ARTIFACT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_ARTIFACT_BYTES {
        return Err(MarketError::ArtifactTooLarge);
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn write_json_new<T: Serialize>(path: &Path, value: &T) -> Result<(), MarketError> {
    let bytes = serde_json::to_vec_pretty(value)?;
    if bytes.len() > MAX_ARTIFACT_BYTES {
        return Err(MarketError::ArtifactTooLarge);
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or(MarketError::InvalidRecord("artifact path has no filename"))?;

    for _attempt in 0..32 {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let mut temporary_name = OsString::from(".");
        temporary_name.push(file_name);
        temporary_name.push(format!(".{}.{}.tmp", std::process::id(), sequence));
        let temporary_path: PathBuf = parent.join(temporary_name);
        let mut temporary_file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(MarketError::Io(error)),
        };

        let write_result = temporary_file
            .write_all(&bytes)
            .and_then(|()| temporary_file.sync_all());
        drop(temporary_file);
        if let Err(error) = write_result {
            let _ = fs::remove_file(&temporary_path);
            return Err(MarketError::Io(error));
        }

        match fs::hard_link(&temporary_path, path) {
            Ok(()) => {
                let _ = fs::remove_file(&temporary_path);
                return Ok(());
            }
            Err(error) => {
                let _ = fs::remove_file(&temporary_path);
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    return Err(MarketError::ArtifactExists);
                }
                return Err(MarketError::Io(error));
            }
        }
    }

    Err(MarketError::Io(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not allocate a unique temporary artifact path",
    )))
}

/// Deterministic helper used by local fixtures and examples.
///
/// # Errors
///
/// Returns an error when timestamp arithmetic or canonical signing bytes fail.
pub fn sample_signed_offer(
    seller_key: &SigningKey,
    offer_id: &str,
    price_microusd_per_unit_hour: u64,
    issued_at: &UtcTimestamp,
) -> Result<SignedOfferV1, MarketError> {
    let offer = OfferV1 {
        schema_version: OFFER_SCHEMA.to_owned(),
        offer_id: offer_id.to_owned(),
        seller_id: "synthetic-seller".to_owned(),
        resource_model: "GPU-H100".to_owned(),
        region: "us-east".to_owned(),
        capacity_units: 8,
        price_microusd_per_unit_hour,
        issued_at: issued_at.clone(),
        expires_at: issued_at.checked_add_seconds(3_600)?,
    };
    let signature = seller_key.sign(&signing_message(
        "hyperai.compute-market.offer-signature.v1",
        &offer,
    )?);
    Ok(SignedOfferV1 {
        offer,
        public_key_hex: HEXLOWER.encode(seller_key.verifying_key().as_bytes()),
        signature_hex: HEXLOWER.encode(&signature.to_bytes()),
    })
}

fn validate_request(request: &BuyerRequestV1, now: &UtcTimestamp) -> Result<(), MarketError> {
    if request.schema_version != REQUEST_SCHEMA
        || request.buyer_id.is_empty()
        || request.resource_model.is_empty()
        || request.region.is_empty()
        || request.units == 0
        || request.duration_hours == 0
        || request.max_rate_microusd_per_unit_hour == 0
        || request.max_total_microusd == 0
        || &request.requested_at > now
        || request.requested_at >= request.valid_until
        || now >= &request.valid_until
    {
        return Err(MarketError::InvalidRecord(
            "buyer request fields or lifecycle",
        ));
    }
    Ok(())
}

fn verify_catalog(catalog: &CatalogSnapshotV1, now: &UtcTimestamp) -> Result<(), MarketError> {
    if catalog.body.schema_version != CATALOG_SCHEMA
        || &catalog.body.as_of > now
        || domain_separated_digest("hyperai.compute-market.catalog.v1", &catalog.body)?
            != catalog.digest
    {
        return Err(MarketError::InvalidRecord(
            "catalog snapshot digest or time",
        ));
    }
    let mut previous_offer_id: Option<&str> = None;
    let mut source_ids = BTreeSet::new();
    for entry in &catalog.body.entries {
        verify_offer(&entry.signed_offer, now)?;
        let offer = &entry.signed_offer.offer;
        if previous_offer_id.is_some_and(|previous| previous >= offer.offer_id.as_str())
            || entry.source_ids.is_empty()
            || entry.source_ids.windows(2).any(|pair| pair[0] >= pair[1])
            || domain_separated_digest(
                "hyperai.compute-market.signed-offer.v1",
                &entry.signed_offer,
            )? != entry.offer_digest
        {
            return Err(MarketError::InvalidRecord(
                "catalog entry ordering or digest",
            ));
        }
        source_ids.extend(entry.source_ids.iter().cloned());
        previous_offer_id = Some(&offer.offer_id);
    }
    if u32::try_from(source_ids.len()).map_err(|_| MarketError::AmountOverflow)?
        != catalog.body.source_count
    {
        return Err(MarketError::InvalidRecord(
            "catalog source count does not match entries",
        ));
    }
    Ok(())
}

fn total_cost(rate: u64, units: u32, hours: u32) -> Result<u64, MarketError> {
    rate.checked_mul(u64::from(units))
        .and_then(|amount| amount.checked_mul(u64::from(hours)))
        .ok_or(MarketError::AmountOverflow)
}

fn verify_quote(quote: &SignedSellerQuoteV1) -> Result<(), MarketError> {
    if quote.quote.schema_version != QUOTE_SCHEMA
        || quote.quote.seller_id.is_empty()
        || quote.quote.units == 0
        || quote.quote.duration_hours == 0
        || quote.quote.quoted_total_microusd > quote.quote.amount_cap_microusd
    {
        return Err(MarketError::InvalidRecord("seller quote fields"));
    }
    verify_signature(
        &quote.quote.seller_public_key_hex,
        &quote.signature_hex,
        &signing_message("hyperai.compute-market.quote-signature.v1", &quote.quote)?,
    )
}

fn verify_approved_deal(deal: &BuyerApprovedDealV1) -> Result<(), MarketError> {
    if deal.schema_version != DEAL_SCHEMA
        || deal.buyer_id.is_empty()
        || deal.buyer_id != deal.buyer_request.buyer_id
    {
        return Err(MarketError::InvalidRecord("approved deal fields"));
    }
    verify_quote(&deal.signed_quote)?;
    validate_request(&deal.buyer_request, &deal.accepted_at)?;
    let request_digest =
        domain_separated_digest("hyperai.compute-market.request.v1", &deal.buyer_request)?;
    if deal.accepted_at >= deal.signed_quote.quote.expires_at
        || deal.signed_quote.quote.request_digest != request_digest
        || deal.signed_quote.quote.rate_microusd_per_unit_hour
            > deal.buyer_request.max_rate_microusd_per_unit_hour
        || deal.signed_quote.quote.units != deal.buyer_request.units
        || deal.signed_quote.quote.duration_hours != deal.buyer_request.duration_hours
        || deal.signed_quote.quote.quoted_total_microusd > deal.buyer_request.max_total_microusd
        || deal.signed_quote.quote.amount_cap_microusd > deal.buyer_request.max_total_microusd
        || deal.signed_quote.quote.expires_at > deal.buyer_request.valid_until
    {
        return Err(MarketError::InvalidRecord(
            "quote expired before buyer approval",
        ));
    }
    let approval = BuyerApprovalV1 {
        schema_version: deal.schema_version.clone(),
        quote_digest: domain_separated_digest(
            "hyperai.compute-market.signed-quote.v1",
            &deal.signed_quote,
        )?,
        buyer_id: deal.buyer_id.clone(),
        buyer_public_key_hex: deal.buyer_public_key_hex.clone(),
        request_digest,
        accepted_at: deal.accepted_at.clone(),
    };
    verify_signature(
        &deal.buyer_public_key_hex,
        &deal.signature_hex,
        &signing_message("hyperai.compute-market.buyer-approval.v1", &approval)?,
    )
}

fn verify_fulfillment_receipt(
    deal: &BuyerApprovedDealV1,
    receipt: &FulfillmentReceiptV1,
) -> Result<(), MarketError> {
    verify_approved_deal(deal)?;
    if receipt.schema_version != FULFILLMENT_SCHEMA
        || receipt.deal_digest != deal_digest(deal)?
        || receipt.seller_id != deal.signed_quote.quote.seller_id
        || receipt.seller_public_key_hex != deal.signed_quote.quote.seller_public_key_hex
        || receipt.completed_at < deal.accepted_at
        || receipt.billed_microusd > deal.signed_quote.quote.amount_cap_microusd
        || receipt.billed_microusd > deal.signed_quote.quote.quoted_total_microusd
    {
        return Err(MarketError::InvalidRecord(
            "fulfillment receipt fields or cap",
        ));
    }
    verify_signature(
        &receipt.seller_public_key_hex,
        &receipt.signature_hex,
        &signing_message(
            "hyperai.compute-market.fulfillment-signature.v1",
            &receipt_payload(receipt),
        )?,
    )
}

/// Verify a seller's deal-bound failure statement.
///
/// This confirms structural and signature consistency only; it cannot
/// establish that the reported failure occurred.
///
/// # Errors
///
/// Returns an error for an invalid deal, record fields, seller signature, or
/// lifecycle timestamps.
pub fn verify_fulfillment_failure_receipt(
    deal: &BuyerApprovedDealV1,
    failure: &FulfillmentFailureReceiptV1,
) -> Result<(), MarketError> {
    verify_approved_deal(deal)?;
    if failure.schema_version != FULFILLMENT_FAILURE_SCHEMA
        || failure.deal_digest != deal_digest(deal)?
        || failure.seller_id != deal.signed_quote.quote.seller_id
        || failure.seller_public_key_hex != deal.signed_quote.quote.seller_public_key_hex
        || !valid_fulfillment_attempt_id(&failure.attempt_id)
        || failure.started_at < deal.accepted_at
        || failure.failed_at < failure.started_at
    {
        return Err(MarketError::InvalidRecord(
            "fulfillment failure fields or lifecycle",
        ));
    }
    verify_signature(
        &failure.seller_public_key_hex,
        &failure.signature_hex,
        &signing_message(
            "hyperai.compute-market.fulfillment-failure-signature.v1",
            &fulfillment_failure_payload(failure),
        )?,
    )
}

fn deal_digest(deal: &BuyerApprovedDealV1) -> Result<Sha256Digest, MarketError> {
    domain_separated_digest("hyperai.compute-market.approved-deal.v1", deal).map_err(Into::into)
}

fn receipt_payload(receipt: &FulfillmentReceiptV1) -> FulfillmentPayloadV1<'_> {
    FulfillmentPayloadV1 {
        schema_version: &receipt.schema_version,
        deal_digest: receipt.deal_digest,
        seller_id: &receipt.seller_id,
        seller_public_key_hex: &receipt.seller_public_key_hex,
        billed_microusd: receipt.billed_microusd,
        completed_at: &receipt.completed_at,
        result_digest: receipt.result_digest,
    }
}

fn fulfillment_failure_payload(
    failure: &FulfillmentFailureReceiptV1,
) -> FulfillmentFailurePayloadV1<'_> {
    FulfillmentFailurePayloadV1 {
        schema_version: &failure.schema_version,
        deal_digest: failure.deal_digest,
        seller_id: &failure.seller_id,
        seller_public_key_hex: &failure.seller_public_key_hex,
        attempt_id: &failure.attempt_id,
        started_at: &failure.started_at,
        failed_at: &failure.failed_at,
        failure_code: &failure.failure_code,
    }
}

#[derive(Serialize)]
struct FulfillmentPayloadV1<'a> {
    schema_version: &'a str,
    deal_digest: Sha256Digest,
    seller_id: &'a str,
    seller_public_key_hex: &'a str,
    billed_microusd: u64,
    completed_at: &'a UtcTimestamp,
    result_digest: Sha256Digest,
}

#[derive(Serialize)]
struct FulfillmentFailurePayloadV1<'a> {
    schema_version: &'a str,
    deal_digest: Sha256Digest,
    seller_id: &'a str,
    seller_public_key_hex: &'a str,
    attempt_id: &'a str,
    started_at: &'a UtcTimestamp,
    failed_at: &'a UtcTimestamp,
    failure_code: &'a FulfillmentFailureCodeV1,
}

#[derive(Serialize)]
struct BuyerApprovalV1 {
    schema_version: String,
    quote_digest: Sha256Digest,
    buyer_id: String,
    buyer_public_key_hex: String,
    request_digest: Sha256Digest,
    accepted_at: UtcTimestamp,
}

fn signing_message<T: Serialize>(domain: &str, value: &T) -> Result<Vec<u8>, MarketError> {
    let canonical = canonical_json(value)?;
    let mut message = Vec::with_capacity(domain.len() + canonical.len() + 1);
    message.extend_from_slice(domain.as_bytes());
    message.push(0);
    message.extend_from_slice(&canonical);
    Ok(message)
}

fn verify_signature(
    public_key_hex: &str,
    signature_hex: &str,
    message: &[u8],
) -> Result<(), MarketError> {
    let public_key_bytes = HEXLOWER
        .decode(public_key_hex.as_bytes())
        .map_err(|_| MarketError::InvalidRecord("public key encoding"))?;
    let public_key: [u8; 32] = public_key_bytes
        .try_into()
        .map_err(|_| MarketError::InvalidRecord("public key length"))?;
    let verifying_key = VerifyingKey::from_bytes(&public_key)
        .map_err(|_| MarketError::InvalidRecord("public key is invalid"))?;
    let signature_bytes = HEXLOWER
        .decode(signature_hex.as_bytes())
        .map_err(|_| MarketError::InvalidRecord("signature encoding"))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| MarketError::InvalidRecord("signature length"))?;
    verifying_key
        .verify(message, &signature)
        .map_err(|_| MarketError::InvalidRecord("signature verification failed"))
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    fn at(value: &str) -> Result<UtcTimestamp, MarketError> {
        value.parse().map_err(MarketError::from)
    }

    fn fixture() -> Result<(SigningKey, SigningKey, CatalogSnapshotV1, BuyerRequestV1), MarketError>
    {
        let seller = SigningKey::from_bytes(&[41; 32]);
        let buyer = SigningKey::from_bytes(&[42; 32]);
        let now = at("2026-09-24T12:00:00Z")?;
        let offer = sample_signed_offer(&seller, "offer-a", 2_000_000, &now)?;
        let catalog = build_catalog(vec![("registry-a".to_owned(), offer)], now.clone())?;
        let request = BuyerRequestV1 {
            schema_version: REQUEST_SCHEMA.to_owned(),
            buyer_id: "buyer-a".to_owned(),
            resource_model: "GPU-H100".to_owned(),
            region: "us-east".to_owned(),
            units: 2,
            duration_hours: 3,
            max_rate_microusd_per_unit_hour: 2_500_000,
            max_total_microusd: 15_000_000,
            requested_at: now,
            valid_until: at("2026-09-24T12:20:00Z")?,
        };
        Ok((seller, buyer, catalog, request))
    }

    fn workload_intent() -> Result<WorkloadIntentV1, MarketError> {
        create_workload_intent_v1(WorkloadIntentBodyV1 {
            schema_version: WORKLOAD_INTENT_SCHEMA.to_owned(),
            workload_id: "workload-synthetic-batch-v1".to_owned(),
            revision: 1,
            previous_revision_digest: None,
            specification_digest: Sha256Digest::from_bytes([0x11; 32]),
            acceptance_contract_digest: Sha256Digest::from_bytes([0x22; 32]),
            data_handling_policy_digest: Sha256Digest::from_bytes([0x33; 32]),
            data_references: vec![WorkloadDataReferenceV1 {
                owner_id: "hyperai-data".to_owned(),
                schema_version: "hyperai.data.release.v1".to_owned(),
                record_id: "synthetic-batch-input".to_owned(),
                digest: Sha256Digest::from_bytes([0x44; 32]),
            }],
            deadline: Some(at("2026-09-24T18:00:00Z")?),
            created_at: at("2026-09-24T12:00:00Z")?,
        })
    }

    #[test]
    fn workload_intent_digest_and_revision_chain_are_immutable() -> Result<(), Box<dyn Error>> {
        let intent = workload_intent()?;
        verify_workload_intent_v1(&intent)?;

        let mut changed = intent.clone();
        changed.body.acceptance_contract_digest = Sha256Digest::from_bytes([0x55; 32]);
        assert!(verify_workload_intent_v1(&changed).is_err());

        let mut revision_body = intent.body.clone();
        revision_body.revision = 2;
        revision_body.previous_revision_digest = Some(intent.digest);
        let revision = create_workload_intent_v1(revision_body)?;
        verify_workload_intent_v1(&revision)?;
        assert_ne!(intent.digest, revision.digest);

        let mut invalid_body = revision.body.clone();
        invalid_body.previous_revision_digest = None;
        assert!(create_workload_intent_v1(invalid_body).is_err());
        Ok(())
    }

    #[test]
    fn workload_request_binding_rejects_changed_source_records() -> Result<(), Box<dyn Error>> {
        let (_, _, _, request) = fixture()?;
        let intent = workload_intent()?;
        let binding = bind_workload_request_v1(&intent, &request)?;
        verify_workload_request_binding_v1(&intent, &request, &binding)?;

        let mut changed_request = request.clone();
        changed_request.units += 1;
        assert!(verify_workload_request_binding_v1(&intent, &changed_request, &binding).is_err());

        let mut changed_binding = binding.clone();
        changed_binding.body.workload_id = "workload-other".to_owned();
        assert!(verify_workload_request_binding_v1(&intent, &request, &changed_binding).is_err());
        Ok(())
    }

    #[test]
    fn workload_intent_requires_sorted_unique_source_references() -> Result<(), Box<dyn Error>> {
        let intent = workload_intent()?;
        let mut body = intent.body;
        body.data_references.push(WorkloadDataReferenceV1 {
            owner_id: "hyperai-data".to_owned(),
            schema_version: "hyperai.data.release.v1".to_owned(),
            record_id: "another-record".to_owned(),
            digest: Sha256Digest::from_bytes([0x66; 32]),
        });
        assert!(create_workload_intent_v1(body).is_err());
        Ok(())
    }

    fn approved_fixture() -> Result<(SigningKey, BuyerApprovedDealV1), MarketError> {
        let (seller, buyer, catalog, request) = fixture()?;
        let now = at("2026-09-24T12:00:00Z")?;
        let selected = select_offer(&catalog, &request, &now)?;
        let quote = create_quote(selected, &request, &seller, &now)?;
        let deal = approve_quote(quote, request, at("2026-09-24T12:05:00Z")?, &buyer)?;
        Ok((seller, deal))
    }

    #[test]
    fn local_demo_completes_bounded_synthetic_lifecycle() -> Result<(), Box<dyn Error>> {
        let receipt = local_market_receipt()?;
        verify_local_market_receipt(&receipt)?;
        let report = local_demo_report(&receipt, false);
        assert_eq!(local_demo()?, report);
        assert_eq!(
            report.settlement.disposition,
            MockDisposition::ReleaseToSeller
        );
        assert_eq!(report.effects, LOCAL_EFFECTS);
        assert_eq!(report.claim_ceiling, LOCAL_CLAIM_CEILING);
        assert_eq!(report.source_count, 2);
        assert_eq!(report.duplicate_count, 1);
        assert_eq!(report.selected_offer_id, "offer-gpu-b");
        assert!(report.receipt_file.is_none());
        assert_eq!(report.receipt_digest, receipt.digest);
        Ok(())
    }

    #[test]
    fn supply_comparison_matches_selector_and_verifies_its_full_digest()
    -> Result<(), Box<dyn Error>> {
        let comparison = local_market_supply_comparison_v1()?;
        verify_supply_comparison_v1(&comparison)?;
        let selected = select_offer(
            &comparison.body.catalog,
            &comparison.body.buyer_request,
            &comparison.body.compared_at,
        )?;
        assert_eq!(
            comparison.body.selected_offer_id.as_deref(),
            Some(selected.signed_offer.offer.offer_id.as_str())
        );
        assert_eq!(
            comparison.body.selected_offer_id.as_deref(),
            Some("offer-gpu-b")
        );
        assert_eq!(comparison.body.offer_assessments.len(), 2);
        assert!(comparison.body.offer_assessments.iter().all(|assessment| {
            assessment.eligible == assessment.ineligibility_reasons.is_empty()
                && assessment.selected
                    == (Some(assessment.offer_id.as_str())
                        == comparison.body.selected_offer_id.as_deref())
        }));
        let mut changed = comparison.clone();
        changed.body.advisory_observations[0]
            .body
            .estimated_available_units = Some(999);
        assert!(verify_supply_comparison_v1(&changed).is_err());
        Ok(())
    }

    #[test]
    fn supply_quote_preview_binds_the_displayed_comparison_to_its_quote()
    -> Result<(), Box<dyn Error>> {
        let comparison = local_market_supply_comparison_v1()?;
        let preview = local_market_supply_quote_preview_v1(comparison.digest)?;
        verify_supply_quote_preview_v1(&preview)?;
        assert_eq!(
            preview.body.schema_version,
            "hyperai.compute-market.supply-quote-preview.v1"
        );
        assert_eq!(preview.body.comparison_digest, comparison.digest);
        assert_eq!(preview.body.catalog_digest, comparison.body.catalog.digest);
        assert_eq!(
            preview.body.buyer_request_digest,
            domain_separated_digest(
                "hyperai.compute-market.request.v1",
                &comparison.body.buyer_request
            )?
        );
        assert_eq!(
            preview.body.selected_offer_id,
            comparison
                .body
                .selected_offer_id
                .as_deref()
                .unwrap_or_default()
        );
        let selected = comparison
            .body
            .offer_assessments
            .iter()
            .find(|assessment| assessment.selected)
            .ok_or_else(|| std::io::Error::other("synthetic comparison has no selected offer"))?;
        assert_eq!(preview.body.selected_offer_digest, selected.offer_digest);
        assert_eq!(
            preview
                .body
                .quote_preview
                .body
                .signed_quote
                .quote
                .offer_digest,
            selected.offer_digest
        );

        let repeated = local_market_supply_quote_preview_v1(comparison.digest)?;
        assert_eq!(preview, repeated);
        let mut changed = preview.clone();
        changed.body.selected_offer_digest = Sha256Digest::from_bytes([0x99; 32]);
        assert!(verify_supply_quote_preview_v1(&changed).is_err());
        assert!(matches!(
            local_market_supply_quote_preview_v1(Sha256Digest::from_bytes([0; 32])),
            Err(MarketError::SupplyComparisonMismatch)
        ));
        Ok(())
    }

    #[test]
    fn supply_quote_preview_refuses_a_comparison_without_an_eligible_offer()
    -> Result<(), Box<dyn Error>> {
        let mut fixture = local_market_quote_fixture()?;
        fixture.buyer_request.resource_model = "GPU-H200".to_owned();
        let comparison = build_supply_comparison(
            fixture.catalog.clone(),
            fixture.buyer_request.clone(),
            fixture.now.clone(),
            local_market_advisory_observations(&fixture)?,
        )?;
        assert_eq!(comparison.body.selected_offer_id, None);
        assert!(matches!(
            local_supply_quote_preview_from_fixture(&fixture, comparison.digest),
            Err(MarketError::SupplyQuoteNoEligibleOffer)
        ));
        Ok(())
    }

    #[test]
    fn advisory_changes_do_not_change_offer_eligibility_or_selection() -> Result<(), Box<dyn Error>>
    {
        let fixture = local_market_quote_fixture()?;
        let observations = local_market_advisory_observations(&fixture)?;
        let before = build_supply_comparison(
            fixture.catalog.clone(),
            fixture.buyer_request.clone(),
            fixture.now.clone(),
            observations.clone(),
        )?;
        let mut changed_observations = observations;
        changed_observations[0]
            .body
            .estimated_rate_microusd_per_unit_hour = Some(9_900_000);
        changed_observations[0].body.estimated_available_units = Some(0);
        changed_observations[0] = build_market_observation(changed_observations[0].body.clone())?;
        let after = build_supply_comparison(
            fixture.catalog,
            fixture.buyer_request,
            fixture.now,
            changed_observations,
        )?;
        assert_eq!(before.body.selected_offer_id, after.body.selected_offer_id);
        assert_eq!(before.body.offer_assessments, after.body.offer_assessments);
        assert_ne!(before.digest, after.digest);
        Ok(())
    }

    #[test]
    fn comparison_represents_no_eligible_offer_without_promoting_observations()
    -> Result<(), Box<dyn Error>> {
        let fixture = local_market_quote_fixture()?;
        let observations = local_market_advisory_observations(&fixture)?;
        let mut request = fixture.buyer_request.clone();
        request.resource_model = "GPU-H200".to_owned();
        let comparison =
            build_supply_comparison(fixture.catalog, request, fixture.now, observations)?;
        verify_supply_comparison_v1(&comparison)?;
        assert_eq!(comparison.body.selected_offer_id, None);
        assert!(
            comparison
                .body
                .offer_assessments
                .iter()
                .all(|assessment| !assessment.eligible && !assessment.selected)
        );
        assert!(
            comparison
                .body
                .advisory_observations
                .iter()
                .any(|observation| { observation.body.estimated_available_units.is_some() })
        );
        Ok(())
    }

    #[test]
    fn offer_assessment_reports_every_buyer_constraint_and_checked_overflow()
    -> Result<(), Box<dyn Error>> {
        let (_, _, catalog, mut request) = fixture()?;
        let mut entry = catalog.body.entries[0].clone();
        entry.signed_offer.offer.resource_model = "GPU-other".to_owned();
        entry.signed_offer.offer.region = "eu-west".to_owned();
        entry.signed_offer.offer.capacity_units = 1;
        entry.signed_offer.offer.price_microusd_per_unit_hour = 3_000_000;
        request.max_total_microusd = 1;
        let assessment = assess_offer_for_request(&entry, &request);
        assert_eq!(
            assessment.ineligibility_reasons,
            [
                OfferIneligibilityReasonV1::ResourceModelMismatch,
                OfferIneligibilityReasonV1::RegionMismatch,
                OfferIneligibilityReasonV1::InsufficientCapacity,
                OfferIneligibilityReasonV1::AskingRateExceedsCap,
                OfferIneligibilityReasonV1::AskingTotalExceedsCap,
            ]
        );
        assert_eq!(assessment.asking_total_microusd, Some(18_000_000));
        assert!(!assessment.eligible);

        request.units = u32::MAX;
        request.duration_hours = u32::MAX;
        let overflow = assess_offer_for_request(&entry, &request);
        assert_eq!(overflow.asking_total_microusd, None);
        assert!(
            overflow
                .ineligibility_reasons
                .contains(&OfferIneligibilityReasonV1::AskingTotalOverflow)
        );
        Ok(())
    }

    #[test]
    fn advisory_market_estimates_are_recorded_but_do_not_set_the_seller_quote()
    -> Result<(), Box<dyn Error>> {
        let receipt = local_market_receipt()?;
        let estimate = receipt
            .body
            .advisory_observations
            .iter()
            .find(|observation| observation.body.offer_id == "offer-gpu-b")
            .ok_or(MarketError::InvalidRecord("missing synthetic observation"))?;
        assert_eq!(
            estimate.body.estimated_rate_microusd_per_unit_hour,
            Some(1_600_000)
        );
        assert_eq!(
            receipt
                .body
                .approved_deal
                .signed_quote
                .quote
                .rate_microusd_per_unit_hour,
            1_800_000
        );
        assert_eq!(
            receipt.body.approved_deal.signed_quote.quote.offer_digest,
            receipt
                .body
                .catalog
                .body
                .entries
                .iter()
                .find(|entry| entry.signed_offer.offer.offer_id == "offer-gpu-b")
                .ok_or(MarketError::InvalidRecord("missing selected offer"))?
                .offer_digest
        );
        Ok(())
    }

    #[test]
    fn file_backed_journal_is_idempotent_and_recovers_a_lost_receipt() -> Result<(), Box<dyn Error>>
    {
        let temporary = tempfile::tempdir()?;
        let journal_path = temporary.path().join("market-journal.json");
        let receipt_path = temporary.path().join("recovered-receipt.json");
        let receipt = local_market_receipt()?;

        assert!(record_local_market_receipt(&journal_path, receipt.clone())?);
        let original_journal = fs::read(&journal_path)?;
        assert!(!record_local_market_receipt(
            &journal_path,
            receipt.clone()
        )?);
        assert_eq!(fs::read(&journal_path)?, original_journal);

        let mut conflicting = receipt.clone();
        let observation = conflicting
            .body
            .advisory_observations
            .first_mut()
            .ok_or(MarketError::InvalidRecord("missing advisory observation"))?;
        observation.body.estimated_rate_microusd_per_unit_hour = Some(2_200_000);
        observation.digest = domain_separated_digest(
            "hyperai.compute-market.advisory-observation.v1",
            &observation.body,
        )?;
        conflicting.digest =
            domain_separated_digest("hyperai.compute-market.local-receipt.v1", &conflicting.body)?;
        verify_local_market_receipt(&conflicting)?;
        assert!(matches!(
            record_local_market_receipt(&journal_path, conflicting),
            Err(MarketError::Replay)
        ));
        assert_eq!(fs::read(&journal_path)?, original_journal);

        let journal = read_local_market_journal(&journal_path)?;
        assert_eq!(journal.body.receipt, receipt);
        let recovered = recover_local_market_receipt(&journal_path, &receipt_path)?;
        assert_eq!(recovered, receipt);
        assert_eq!(read_local_market_receipt(&receipt_path)?, receipt);
        assert!(matches!(
            recover_local_market_receipt(&journal_path, &receipt_path),
            Err(MarketError::ArtifactExists)
        ));
        Ok(())
    }

    #[test]
    fn offline_receipt_check_rejects_tampering_and_unsafe_paths() -> Result<(), Box<dyn Error>> {
        let temporary = tempfile::tempdir()?;
        let oversized = temporary.path().join("oversized.json");
        fs::write(&oversized, vec![b'x'; MAX_ARTIFACT_BYTES + 1])?;
        assert!(matches!(
            read_local_market_receipt(&oversized),
            Err(MarketError::ArtifactTooLarge)
        ));

        let receipt = local_market_receipt()?;
        #[cfg(unix)]
        {
            let symlink = temporary.path().join("receipt-link.json");
            std::os::unix::fs::symlink(&oversized, &symlink)?;
            assert!(matches!(
                read_local_market_receipt(&symlink),
                Err(MarketError::UnsafeArtifactPath)
            ));
        }

        let mut tampered = receipt.clone();
        tampered.body.fulfillment.billed_microusd += 1;
        tampered.digest =
            domain_separated_digest("hyperai.compute-market.local-receipt.v1", &tampered.body)?;
        assert!(matches!(
            verify_local_market_receipt(&tampered),
            Err(MarketError::InvalidRecord(_))
        ));

        let mut changed_claim = receipt;
        changed_claim.body.claim_ceiling = "all compute is correct".to_owned();
        changed_claim.digest = domain_separated_digest(
            "hyperai.compute-market.local-receipt.v1",
            &changed_claim.body,
        )?;
        assert!(matches!(
            verify_local_market_receipt(&changed_claim),
            Err(MarketError::InvalidRecord(_))
        ));
        Ok(())
    }

    #[test]
    fn signed_offer_tampering_and_staleness_are_rejected() -> Result<(), Box<dyn Error>> {
        let seller = SigningKey::from_bytes(&[41; 32]);
        let now = at("2026-09-24T12:00:00Z")?;
        let mut offer = sample_signed_offer(&seller, "offer-a", 2_000_000, &now)?;
        offer.offer.price_microusd_per_unit_hour += 1;
        assert!(matches!(
            verify_offer(&offer, &now),
            Err(MarketError::InvalidRecord(_))
        ));
        let old = at("2026-09-24T14:00:00Z")?;
        let fresh_offer = sample_signed_offer(&seller, "offer-b", 2_000_000, &now)?;
        assert!(matches!(
            verify_offer(&fresh_offer, &old),
            Err(MarketError::InvalidRecord(_))
        ));
        Ok(())
    }

    #[test]
    fn duplicate_snapshots_deduplicate_but_conflicts_fail_closed() -> Result<(), Box<dyn Error>> {
        let seller = SigningKey::from_bytes(&[41; 32]);
        let now = at("2026-09-24T12:00:00Z")?;
        let offer = sample_signed_offer(&seller, "offer-a", 2_000_000, &now)?;
        let catalog = build_catalog(
            vec![
                ("a".to_owned(), offer.clone()),
                ("b".to_owned(), offer.clone()),
            ],
            now.clone(),
        )?;
        assert_eq!(catalog.body.entries.len(), 1);
        assert_eq!(catalog.body.duplicate_count, 1);
        let conflicting = sample_signed_offer(&seller, "offer-a", 2_100_000, &now)?;
        assert!(matches!(
            build_catalog(
                vec![("a".to_owned(), offer), ("b".to_owned(), conflicting)],
                now,
            ),
            Err(MarketError::ConflictingOffer)
        ));
        Ok(())
    }

    #[test]
    fn buyer_constraints_are_never_widened_by_selection_or_quote() -> Result<(), Box<dyn Error>> {
        let (seller, _buyer, catalog, mut request) = fixture()?;
        request.max_total_microusd = 11_999_999;
        let now = at("2026-09-24T12:00:00Z")?;
        assert!(matches!(
            select_offer(&catalog, &request, &now),
            Err(MarketError::NoEligibleOffer)
        ));
        request.max_total_microusd = 12_000_000;
        let selected = select_offer(&catalog, &request, &now)?;
        let quote = create_quote(selected, &request, &seller, &now)?;
        assert_eq!(quote.quote.amount_cap_microusd, request.max_total_microusd);
        assert!(quote.quote.quoted_total_microusd <= request.max_total_microusd);
        Ok(())
    }

    #[test]
    fn buyer_approval_rejects_changed_request_and_expired_quote() -> Result<(), Box<dyn Error>> {
        let (seller, buyer, catalog, request) = fixture()?;
        let now = at("2026-09-24T12:00:00Z")?;
        let selected = select_offer(&catalog, &request, &now)?;
        let quote = create_quote(selected, &request, &seller, &now)?;
        let mut changed_request = request.clone();
        changed_request.max_total_microusd += 1;
        assert!(
            approve_quote(
                quote.clone(),
                changed_request,
                at("2026-09-24T12:05:00Z")?,
                &buyer,
            )
            .is_err()
        );
        assert!(approve_quote(quote, request.clone(), request.valid_until, &buyer,).is_err());
        Ok(())
    }

    #[test]
    fn wrong_fulfillment_signer_and_over_cap_amount_are_rejected() -> Result<(), Box<dyn Error>> {
        let (seller, deal) = approved_fixture()?;
        let wrong_seller = SigningKey::from_bytes(&[77; 32]);
        assert!(
            create_fulfillment_receipt(
                &deal,
                12_000_000,
                at("2026-09-24T15:00:00Z")?,
                Sha256Digest::from_bytes([0x5a; 32]),
                &wrong_seller,
            )
            .is_err()
        );
        assert!(
            create_fulfillment_receipt(
                &deal,
                12_000_001,
                at("2026-09-24T15:00:00Z")?,
                Sha256Digest::from_bytes([0x5a; 32]),
                &seller,
            )
            .is_err()
        );
        assert!(
            create_fulfillment_receipt(
                &deal,
                15_000_001,
                at("2026-09-24T15:00:00Z")?,
                Sha256Digest::from_bytes([0x5a; 32]),
                &seller,
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn fulfillment_failure_receipt_is_signed_deal_bound_and_non_settling()
    -> Result<(), Box<dyn Error>> {
        let (seller, deal) = approved_fixture()?;
        let failure = create_fulfillment_failure_receipt(
            &deal,
            "attempt-1".to_owned(),
            at("2026-09-24T12:06:00Z")?,
            at("2026-09-24T12:07:00Z")?,
            FulfillmentFailureCodeV1::ResourceUnavailable,
            &seller,
        )?;
        verify_fulfillment_failure_receipt(&deal, &failure)?;
        assert_ne!(
            fulfillment_failure_digest_v1(&failure)?,
            Sha256Digest::from_bytes([0; 32])
        );

        let serialized = serde_json::to_value(&failure)?;
        for field in ["billed_microusd", "refund", "disposition", "result_digest"] {
            assert!(serialized.get(field).is_none());
        }

        let mut tampered = failure.clone();
        tampered.failure_code = FulfillmentFailureCodeV1::ExecutionError;
        assert!(verify_fulfillment_failure_receipt(&deal, &tampered).is_err());

        let wrong_seller = SigningKey::from_bytes(&[77; 32]);
        assert!(
            create_fulfillment_failure_receipt(
                &deal,
                "attempt-1".to_owned(),
                at("2026-09-24T12:06:00Z")?,
                at("2026-09-24T12:07:00Z")?,
                FulfillmentFailureCodeV1::ResourceUnavailable,
                &wrong_seller,
            )
            .is_err()
        );
        assert!(
            create_fulfillment_failure_receipt(
                &deal,
                "../attempt".to_owned(),
                at("2026-09-24T12:06:00Z")?,
                at("2026-09-24T12:07:00Z")?,
                FulfillmentFailureCodeV1::ResourceUnavailable,
                &seller,
            )
            .is_err()
        );
        assert!(
            create_fulfillment_failure_receipt(
                &deal,
                "attempt-1".to_owned(),
                at("2026-09-24T12:07:00Z")?,
                at("2026-09-24T12:06:00Z")?,
                FulfillmentFailureCodeV1::ResourceUnavailable,
                &seller,
            )
            .is_err()
        );
        assert!(
            create_fulfillment_failure_receipt(
                &deal,
                "attempt-1".to_owned(),
                at("2026-09-24T12:04:00Z")?,
                at("2026-09-24T12:07:00Z")?,
                FulfillmentFailureCodeV1::ResourceUnavailable,
                &seller,
            )
            .is_err()
        );

        let (_, buyer, catalog, mut other_request) = fixture()?;
        other_request.buyer_id = "buyer-b".to_owned();
        let now = at("2026-09-24T12:00:00Z")?;
        let selected = select_offer(&catalog, &other_request, &now)?;
        let other_quote = create_quote(selected, &other_request, &seller, &now)?;
        let other_deal = approve_quote(
            other_quote,
            other_request,
            at("2026-09-24T12:05:00Z")?,
            &buyer,
        )?;
        assert!(verify_fulfillment_failure_receipt(&other_deal, &failure).is_err());
        Ok(())
    }

    #[test]
    fn local_failure_fixture_requires_the_current_preview_and_approval()
    -> Result<(), Box<dyn Error>> {
        let preview = local_market_quote_preview()?;
        let approval = approve_local_market_quote_preview(preview.digest)?;
        let failure = fail_local_market_quote_preview(preview.digest, approval.deal_digest)?;
        verify_fulfillment_failure_receipt(&approval.approved_deal, &failure)?;
        assert_eq!(failure.attempt_id, "synthetic-attempt-1");
        assert_eq!(
            failure.failure_code,
            FulfillmentFailureCodeV1::ResourceUnavailable
        );
        assert!(
            fail_local_market_quote_preview(
                Sha256Digest::from_bytes([0x99; 32]),
                approval.deal_digest,
            )
            .is_err()
        );
        assert!(
            fail_local_market_quote_preview(preview.digest, Sha256Digest::from_bytes([0x99; 32]),)
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn local_quote_scenarios_are_distinct_rebuilt_and_closed() -> Result<(), Box<dyn Error>> {
        let baseline =
            local_market_quote_preview_for_scenario(LocalMarketQuoteScenarioV1::Baseline)?;
        let alternate =
            local_market_quote_preview_for_scenario(LocalMarketQuoteScenarioV1::Alternate)?;
        assert_eq!(
            LocalMarketQuoteScenarioV1::from_scenario_id("alternate"),
            Some(LocalMarketQuoteScenarioV1::Alternate)
        );
        assert_eq!(
            LocalMarketQuoteScenarioV1::from_scenario_id("unlisted"),
            None
        );
        assert_ne!(baseline.digest, alternate.digest);
        assert_ne!(
            baseline.body.buyer_request.buyer_id,
            alternate.body.buyer_request.buyer_id
        );
        assert_eq!(alternate.body.buyer_request.units, 1);
        assert_eq!(alternate.body.buyer_request.duration_hours, 2);

        let baseline_approval =
            approve_local_market_quote_preview_for_known_scenarios(baseline.digest)?;
        let alternate_approval =
            approve_local_market_quote_preview_for_known_scenarios(alternate.digest)?;
        assert_ne!(
            baseline_approval.deal_digest,
            alternate_approval.deal_digest
        );
        assert_eq!(
            alternate_approval,
            approve_local_market_quote_preview_for_scenario(
                alternate.digest,
                LocalMarketQuoteScenarioV1::Alternate,
            )?
        );

        let baseline_receipt = complete_local_market_quote_preview_for_known_scenarios(
            baseline.digest,
            baseline_approval.deal_digest,
        )?;
        let alternate_receipt = complete_local_market_quote_preview_for_known_scenarios(
            alternate.digest,
            alternate_approval.deal_digest,
        )?;
        verify_local_market_receipt(&baseline_receipt)?;
        verify_local_market_receipt(&alternate_receipt)?;
        assert_eq!(
            alternate_receipt.body.buyer_request.buyer_id,
            "synthetic-buyer-alternate"
        );
        assert!(
            approve_local_market_quote_preview_for_known_scenarios(Sha256Digest::from_bytes(
                [0x99; 32]
            ))
            .is_err()
        );
        assert!(
            complete_local_market_quote_preview_for_known_scenarios(
                alternate.digest,
                Sha256Digest::from_bytes([0x99; 32])
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn decision_mapping_and_replay_guard_are_explicit() -> Result<(), Box<dyn Error>> {
        let (seller, deal) = approved_fixture()?;
        let receipt = create_fulfillment_receipt(
            &deal,
            12_000_000,
            at("2026-09-24T15:00:00Z")?,
            Sha256Digest::from_bytes([0x5a; 32]),
            &seller,
        )?;
        let decision = mock_verifier_decision(
            &deal,
            &receipt,
            CompletionVerdict::Inconclusive,
            "fixture_has_no_independent_verifier".to_owned(),
        )?;
        let mut ledger = MockSettlementLedger::default();
        let settled = ledger.settle(&deal, &receipt, &decision, at("2026-09-24T15:01:00Z")?)?;
        assert_eq!(settled.disposition, MockDisposition::HoldForDispute);
        assert!(matches!(
            ledger.settle(&deal, &receipt, &decision, at("2026-09-24T15:02:00Z")?),
            Err(MarketError::Replay)
        ));
        let rejected = mock_verifier_decision(
            &deal,
            &receipt,
            CompletionVerdict::Rejected,
            "synthetic_fixture_fail".to_owned(),
        )?;
        let refunded = MockSettlementLedger::default().settle(
            &deal,
            &receipt,
            &rejected,
            at("2026-09-24T15:03:00Z")?,
        )?;
        assert_eq!(refunded.disposition, MockDisposition::RefundToBuyer);
        Ok(())
    }
}
