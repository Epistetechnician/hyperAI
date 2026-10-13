//! `HyperAI` application facade for its composed local interface.
//!
//! This facade composes module-owned contracts without taking ownership of
//! provider routing, market settlement, outcome resolution, or evidence custody.

mod interface_error;

pub use interface_error::{
    API_ERROR_MESSAGE, API_ERROR_SCHEMA_VERSION, NormalizedInterfaceErrorV1,
    normalize_interface_error_v1,
};

use hyperai_data::DataError;
use hyperai_foundation::{Sha256Digest, domain_separated_digest};
use hyperai_market::{
    CompletionVerdict, LocalDemoReportV1, LocalMarketReceiptV1, MarketError, MockDisposition,
    MockSettlementLedger, SupplyComparisonV1, SupplyQuotePreviewRequestV1, WorkloadDataReferenceV1,
    WorkloadIntentBodyV1, WorkloadIntentV1, WorkloadRequestBindingV1,
    approve_local_market_quote_preview_for_known_scenarios, bind_workload_request_v1,
    complete_local_market_quote_preview_for_known_scenarios, create_workload_intent_v1,
    local_demo_report, local_market_quote_preview, local_market_quote_preview_for_scenario,
    local_market_receipt, local_market_supply_comparison_v1, local_market_supply_quote_preview_v1,
    mock_verifier_decision, verify_local_market_receipt, verify_supply_quote_preview_v1,
    verify_workload_request_binding_v1,
};
use hyperai_market::{
    FulfillmentFailureReceiptV1, fail_local_market_quote_preview, fulfillment_failure_digest_v1,
    verify_fulfillment_failure_receipt,
};
use serde::{Deserialize, Serialize};

const WORKLOAD_TRACE_SUCCESS_SLICE: &str = "hyperai.bazaar.workload_trace_synthetic.v1";
const WORKLOAD_TRACE_FAILURE_SLICE: &str = "hyperai.bazaar.workload_trace_seller_failure.v1";
const WORKLOAD_TRACE_INCONCLUSIVE_SLICE: &str =
    "hyperai.bazaar.workload_trace_inconclusive_served.v1";
const WORKLOAD_TRACE_DATA_STATUS_SLICE: &str =
    "hyperai.bazaar.workload_trace_data_status_served.v1";
const WORKLOAD_TRACE_DUPLICATE_DISPOSITION_SLICE: &str =
    "hyperai.bazaar.workload_trace_duplicate_disposition_served.v1";
const WORKLOAD_TRACE_SUCCESS_CLAIM: &str =
    "digest-bound local synthetic workload-to-receipt projection";
const WORKLOAD_TRACE_FAILURE_CLAIM: &str =
    "digest-bound local synthetic seller-attributed failure projection";
const WORKLOAD_TRACE_INCONCLUSIVE_CLAIM: &str =
    "digest-bound local synthetic inconclusive mock-outcome preview";
const WORKLOAD_TRACE_DATA_STATUS_CLAIM: &str =
    "digest-bound local synthetic unresolved Data status preview";
const WORKLOAD_TRACE_DUPLICATE_DISPOSITION_CLAIM: &str =
    "digest-bound local synthetic duplicate disposition replay refusal preview";
const WORKLOAD_TRACE_NONCLAIMS: &str = "no real availability, execution, correct computation, seller identity, independent verification, payment entitlement, inference, outcome resolution, custody, or public evidence claim";
const WORKLOAD_TRACE_DATA_STATUS_NONCLAIMS: &str = "no real availability, execution, correct computation, seller identity, independent verification, payment entitlement, inference, outcome resolution, custody, or public evidence claim; the Data status is a fixed synthetic fixture, no Data source was read, and no real source conflict or access decision occurred";
const WORKLOAD_TRACE_DATA_STATUS_LINK_BASIS: &str =
    "fixed_synthetic_status_fixture_no_data_source_read";
const WORKLOAD_TRACE_DATA_STATUS_CLAIM_CEILING: &str =
    "synthetic_data_status_fixture_only_no_source_read_conflict_or_access_decision";
const WORKLOAD_TRACE_DUPLICATE_DISPOSITION_NONCLAIMS: &str = "no second mock settlement receipt, durable idempotency, payment, funds movement, execution, correct computation, independent verification, or payment entitlement; the replay refusal is an in-memory synthetic contract result";

/// Fixed unresolved Data states exposed by the local Workload Trace preview.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkloadTraceDataStatusPreviewV1 {
    /// The selected synthetic input is not present in this fixture scenario.
    Missing,
    /// The selected synthetic input has a fixed conflicting-state fixture.
    Conflicting,
    /// The selected synthetic input has a fixed denied-state fixture.
    Denied,
}

impl WorkloadTraceDataStatusPreviewV1 {
    /// Parse a supported fixed status from its route path segment.
    #[must_use]
    pub fn from_status(status: &str) -> Option<Self> {
        match status {
            "missing" => Some(Self::Missing),
            "conflicting" => Some(Self::Conflicting),
            "denied" => Some(Self::Denied),
            _ => None,
        }
    }

    const fn as_status(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Conflicting => "conflicting",
            Self::Denied => "denied",
        }
    }
}

pub use hyperai_data::{
    BronzeReferenceV1, DataCatalogV1, DataModelCatalogV1, DataModelRecordV1, DataModelV1,
    DataViewCatalogV1, DataViewRecordV1, DataViewV1, DatasetManifestV1, DefinitionPreviewV1,
    DefinitionSavedV1, GPUDIFF_SOURCE_URL_V1, GpudiffImportReceiptV1, GpudiffImportV1,
    NormalizedObservationV1, QueryFormat, QueryLimitsV1, QueryRequestV1, QueryResultV1,
    SyntheticDataReleaseV1, SyntheticGroupSummaryV1,
};
pub use hyperai_eval::{
    EVAL_PAIR_ID_V1, EVAL_REPEAT_COUNT_V1, EVAL_SEED_LABEL_V1, EVAL_STATE_SLICE_V1,
    EVAL_SUITE_ID_V1, EvalCaseOutcomeV1, EvalCaseSummaryV1, EvalCatalogV1, EvalCategoryV1,
    EvalError, EvalFixtureScoreV1, EvalRunReportV1, EvalRunRequestV1, EvalScorerOutputV1,
    EvalScorerRequestV1, MAX_SCORER_INPUT_BYTES_V1, MAX_SCORER_OUTPUT_BYTES_V1, eval_run_report_v1,
    eval_run_request_digest_v1, run_scorer_stdio_v1, score_fixture_pair_v1,
    validate_eval_run_request_v1, validate_scorer_output_v1,
};
pub use hyperai_fleet::{
    FleetDoctorV1, FleetError, FleetHealthEventV1, FleetHealthStateV1, FleetInventoryV1,
    FleetNodeV1, FleetOperationPlanV1, FleetOperationV1, FleetPlanRequestV1, FleetProvenanceV1,
    FleetSimulationFaultV1, FleetSimulationLeaseStateV1, FleetSimulationOperationV1,
    FleetSimulationPhaseV1, FleetSimulationSnapshotV1, FleetSimulationStoreV1,
    FleetSimulationTransitionRequestV1, FleetSimulationTransitionV1,
};
pub use hyperai_hyperintel_adapter::{
    HyperIntelBundleError, HyperIntelBundleVerificationV1, HyperIntelReconciliationError,
    HyperIntelReconciliationV1,
};
pub use hyperai_market::{
    LocalMarketQuoteApprovalV1, LocalMarketQuotePreviewV1, LocalMarketQuoteScenarioV1,
    SupplyQuotePreviewV1,
};
pub use hyperai_scm_adapter::{ListingsCaptureError, ListingsCaptureVerificationV1};
pub use hyperai_trade::{
    TradeCatalogV1, TradeError, TradeFixtureCompositionV1, TradeInstrumentV1,
    TradePaperPeriodStatusV1, TradePaperPeriodV1, TradePaperReplayReportV1,
    TradePaperReplayRequestV1, TradeScenarioReportV1, TradeScenarioRequestV1, TradeSensitivityV1,
    TradeSeriesReportV1,
};

/// Versioned, read-only inventory of product connections visible to the interface.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InterfaceStatusV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// Runtime mode shown in the interface.
    pub mode: &'static str,
    /// User-visible module connection inventory.
    pub connections: Vec<ModuleConnectionV1>,
}

/// Describes one product boundary without implying a health or trust check.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ModuleConnectionV1 {
    /// Stable module identifier.
    pub module_id: &'static str,
    /// Display name.
    pub name: &'static str,
    /// Connection state; this is inventory metadata, not a live probe.
    pub state: &'static str,
    /// Product or module that owns the underlying contract.
    pub owner: &'static str,
    /// Boundary currently exposed through this interface.
    pub boundary: &'static str,
}

/// Static mode used to describe how a product is represented to the interface.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionModeV1 {
    /// HyperAI-owned local synthetic behavior is available.
    LocalSynthetic,
    /// Only caller-selected offline inputs are verified or reconciled.
    OfflineCallerSelected,
    /// A design reference is visible, but no implementation or data is loaded.
    ReferenceOnly,
    /// A related workspace exists but has no runtime connection here.
    WorkspaceNotConnected,
    /// A possible future connection is not in this runtime.
    Planned,
}

/// A documented prerequisite that blocks a broader connection or claim.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConnectionGateV1 {
    /// Stable identifier for the next documented gate.
    pub id: &'static str,
    /// User-readable explanation of the gate.
    pub summary: &'static str,
}

/// One static product connection descriptor shown in `HyperAI`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConnectionDescriptorV1 {
    /// Stable module identifier.
    pub module_id: &'static str,
    /// Display name.
    pub name: &'static str,
    /// Product or module that owns the underlying records and authority.
    pub owner: &'static str,
    /// Recorded source revision, when this descriptor represents a pinned source.
    pub source_revision: Option<&'static str>,
    /// How this source is represented in the current local runtime.
    pub connection_mode: ConnectionModeV1,
    /// Static state label retained by the compatibility status contract.
    pub state: &'static str,
    /// Current user-visible scope and limitation.
    pub boundary: &'static str,
    /// Stable identifiers for descriptive surfaces; these are not grants.
    pub exposed_capability_ids: Vec<&'static str>,
    /// Next documented gate; this field does not imply owner approval.
    pub next_blocked_gate: ConnectionGateV1,
}

/// Versioned, static connection inventory shared by first-party clients.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConnectionInventoryV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// Inventory-generation mode; it is not a readiness status.
    pub mode: &'static str,
    /// Explicit limit on what this static inventory means.
    pub authority_note: &'static str,
    /// Stable, ordered product connection descriptors.
    pub connections: Vec<ConnectionDescriptorV1>,
}

/// The exact local synthetic market flow and its full receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LocalMarketDemoV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// Existing aggregate report for the synthetic flow.
    pub report: LocalDemoReportV1,
    /// Complete digest-bound receipt rendered by the interface.
    pub receipt: LocalMarketReceiptV1,
    /// Explicit effect boundary for this result.
    pub effect_boundary: &'static str,
}

/// Result of rechecking one local market receipt against its owning contract.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LocalMarketVerificationV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// Structural and signature-consistency result.
    pub status: &'static str,
    /// Digest of the checked full receipt.
    pub receipt_digest: Sha256Digest,
    /// Exact claim ceiling carried by the checked receipt.
    pub claim_ceiling: String,
    /// Explicit limits on what the check establishes.
    pub nonclaims: String,
}

/// Source-owned record pointer carried by the local workload trace.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WorkloadTraceReferenceV1 {
    /// Product or module that owns the referenced record.
    pub owner_id: &'static str,
    /// Versioned source record schema, or `not_selected` when no record was selected.
    pub schema_version: String,
    /// Stable source record identity, or a fixed explanation when absent.
    pub record_id: String,
    /// Source-provided digest when the owner contract exposes one.
    pub digest: Option<Sha256Digest>,
    /// Relationship between this record and the workload.
    pub status: &'static str,
    /// Why this record appears in the projection.
    pub link_basis: &'static str,
    /// Narrowest claim supported by this reference.
    pub claim_ceiling: &'static str,
}

/// Digest-bound, read-only projection across the local synthetic workload path.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WorkloadTraceV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// State slice that owns this local synthetic projection.
    pub state_slice: &'static str,
    /// Immutable buyer intent and exact request binding.
    pub intent: WorkloadIntentV1,
    /// Binding to the unchanged market-owned buyer request.
    pub request_binding: WorkloadRequestBindingV1,
    /// Owner-labelled record references in stable order.
    pub references: Vec<WorkloadTraceReferenceV1>,
    /// Maximum statement supported by the complete trace.
    pub claim_ceiling: &'static str,
    /// Explicit excluded claims.
    pub nonclaims: &'static str,
    /// Digest over all preceding trace fields.
    pub digest: Sha256Digest,
}

#[derive(Serialize)]
struct WorkloadTraceDigestBodyV1<'a> {
    schema_version: &'a str,
    state_slice: &'a str,
    intent: &'a WorkloadIntentV1,
    request_binding: &'a WorkloadRequestBindingV1,
    references: &'a [WorkloadTraceReferenceV1],
    claim_ceiling: &'a str,
    nonclaims: &'a str,
}

/// Digest-only request to approve the exact synthetic quote shown to the user.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMarketQuoteApprovalRequestV1 {
    /// Digest of the full local quote preview body.
    pub preview_digest: Sha256Digest,
}

/// Digest-only request to complete one locally approved synthetic deal.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMarketQuoteCompletionRequestV1 {
    /// Digest of the full quote preview previously shown to the user.
    pub preview_digest: Sha256Digest,
    /// Digest of the synthetic buyer-approved deal previously returned.
    pub deal_digest: Sha256Digest,
}

/// Receipt returned after the separately confirmed local mock-completion step.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LocalMarketQuoteCompletionV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// State slice that owns the staged completion action.
    pub state_slice: &'static str,
    /// Exact quote preview digest carried into the receipt.
    pub preview_digest: Sha256Digest,
    /// Exact approved deal digest carried into the receipt.
    pub deal_digest: Sha256Digest,
    /// Full local synthetic market demo report and verified receipt.
    pub demo: LocalMarketDemoV1,
    /// Explicit effect boundary for this result.
    pub effect_boundary: &'static str,
}

/// Errors returned by the application facade.
#[derive(Debug, thiserror::Error)]
pub enum InterfaceError {
    /// A canonical, versioned application projection could not be encoded.
    #[error(transparent)]
    Canonicalization(#[from] hyperai_foundation::CanonicalizationError),
    /// The local data catalog, model, view, or query operation failed.
    #[error(transparent)]
    Data(#[from] DataError),
    /// The local fixed-fixture evaluation contract rejected a request.
    #[error(transparent)]
    Eval(#[from] EvalError),
    /// The local Fleet fixture or preview contract rejected an operation.
    #[error(transparent)]
    Fleet(#[from] FleetError),
    /// The local Trade synthetic research contract rejected an operation.
    #[error(transparent)]
    Trade(#[from] TradeError),
    /// The existing compute-market contract rejected a local operation.
    #[error(transparent)]
    Market(#[from] MarketError),
    /// The pinned `HyperIntel` evidence verifier rejected an offline input.
    #[error(transparent)]
    HyperIntelBundle(#[from] hyperai_hyperintel_adapter::HyperIntelBundleError),
    /// The pinned `HyperIntel` execution reconciler rejected an offline lineage.
    #[error(transparent)]
    HyperIntelReconciliation(#[from] hyperai_hyperintel_adapter::HyperIntelReconciliationError),
    /// The pinned Simple Compute Market source signature rejected an offline capture.
    #[error(transparent)]
    SimpleMarketCapture(#[from] hyperai_scm_adapter::ListingsCaptureError),
}

/// Read the source-verified local Data catalog beneath the `HyperAI` home.
///
/// # Errors
///
/// Returns a data contract or integrity error for an invalid local catalog.
pub fn data_catalog_v1(data_root: &std::path::Path) -> Result<DataCatalogV1, InterfaceError> {
    hyperai_data::catalog(data_root).map_err(InterfaceError::from)
}

/// Validate and retain one response from the fixed public `GPUDiff` endpoint.
///
/// Network access remains the local service's responsibility; this facade
/// accepts only already bounded response bytes and never performs a fetch.
///
/// # Errors
///
/// Returns a data-contract error when the response or local storage is invalid.
pub fn import_gpudiff_data_snapshot_v1(
    data_root: &std::path::Path,
    raw_bytes: &[u8],
    etag: Option<&str>,
    last_modified: Option<&str>,
) -> Result<GpudiffImportV1, InterfaceError> {
    hyperai_data::import_gpudiff_snapshot_v1(data_root, raw_bytes, etag, last_modified)
        .map_err(InterfaceError::from)
}

/// Read the fixed HyperAI-authored synthetic Fleet inventory.
///
/// This returns scenario records only; it does not inspect the host, read
/// imported inventory, or probe a provider.
///
/// # Errors
///
/// Returns an error if the versioned fixture cannot be canonically encoded.
pub fn fleet_inventory_v1() -> Result<FleetInventoryV1, InterfaceError> {
    hyperai_fleet::inventory_v1().map_err(InterfaceError::from)
}

/// Read deterministic local Fleet fixture checks.
///
/// Live health and operation authority remain explicitly unknown or blocked.
#[must_use]
pub fn fleet_doctor_v1() -> FleetDoctorV1 {
    hyperai_fleet::doctor_v1()
}

/// Build a digest-bound Fleet operation preview without invoking it.
///
/// # Errors
///
/// Returns an error when the node is not in the synthetic fixture or the
/// preview cannot be canonically encoded.
pub fn preview_fleet_operation_v1(
    request: &FleetPlanRequestV1,
) -> Result<FleetOperationPlanV1, InterfaceError> {
    hyperai_fleet::preview_operation_v1(request).map_err(InterfaceError::from)
}

/// Read the current snapshot from the local service's volatile Fleet simulator.
///
/// # Errors
///
/// Returns an error if the simulator is unavailable or its state cannot be
/// represented canonically.
pub fn fleet_simulation_snapshot_v1(
    store: &FleetSimulationStoreV1,
) -> Result<FleetSimulationSnapshotV1, InterfaceError> {
    hyperai_fleet::simulation_snapshot_v1(store).map_err(InterfaceError::from)
}

/// Read the public fixed synthetic Eval catalog without answer fixture codes.
///
/// # Errors
///
/// Returns an error if the owned catalog cannot be canonically hashed.
pub fn eval_catalog_v1() -> Result<EvalCatalogV1, InterfaceError> {
    hyperai_eval::eval_catalog_v1().map_err(InterfaceError::from)
}

/// Read the catalog for the Data-owned synthetic Trade release.
///
/// # Errors
///
/// Returns an error if the fixed source release or research summary is invalid.
pub fn trade_catalog_v1() -> Result<TradeCatalogV1, InterfaceError> {
    hyperai_trade::trade_catalog_v1().map_err(InterfaceError::from)
}

/// Read a gap-aware synthetic series analysis by research instrument ID.
///
/// # Errors
///
/// Returns an error for an unknown instrument or invalid source summary.
pub fn trade_series_v1(instrument_id: &str) -> Result<TradeSeriesReportV1, InterfaceError> {
    hyperai_trade::trade_series_v1(instrument_id).map_err(InterfaceError::from)
}

/// Compute a bounded local asking-price scenario from a typed request.
///
/// # Errors
///
/// Returns an error for a stale release digest, invalid assumption, or
/// incompatible research instruments.
pub fn trade_scenario_v1(
    request: &TradeScenarioRequestV1,
) -> Result<TradeScenarioReportV1, InterfaceError> {
    hyperai_trade::trade_scenario_v1(request).map_err(InterfaceError::from)
}

/// Compute a deterministic paper procurement-cost replay.
///
/// # Errors
///
/// Returns an error for a stale release digest, unknown baseline, or invalid
/// requested capacity.
pub fn trade_paper_replay_v1(
    request: &TradePaperReplayRequestV1,
) -> Result<TradePaperReplayReportV1, InterfaceError> {
    hyperai_trade::trade_paper_replay_v1(request).map_err(InterfaceError::from)
}

/// Apply one digest-bound simulator transition without invoking a resource.
///
/// # Errors
///
/// Returns an error for a stale digest, conflicting idempotency key, invalid
/// transition, full bounded cache, or unavailable simulator state.
pub fn transition_fleet_simulation_v1(
    store: &FleetSimulationStoreV1,
    request: &FleetSimulationTransitionRequestV1,
) -> Result<FleetSimulationTransitionV1, InterfaceError> {
    hyperai_fleet::transition_simulation_v1(store, request).map_err(InterfaceError::from)
}

/// Derive the deterministic synthetic Bronze/Silver/Gold view through the
/// HyperAI-owned Data contract.
///
/// # Errors
///
/// Returns a data contract or integrity error if the synthetic input is
/// absent, stale, unsafe, or malformed.
pub async fn synthetic_data_release_v1(
    data_root: &std::path::Path,
) -> Result<SyntheticDataReleaseV1, InterfaceError> {
    hyperai_data::derive_synthetic_release(data_root)
        .await
        .map_err(InterfaceError::from)
}

/// List saved query models and their input-snapshot state.
///
/// # Errors
///
/// Returns an error if a saved model definition or its catalog is invalid.
pub fn data_models_v1(data_root: &std::path::Path) -> Result<DataModelCatalogV1, InterfaceError> {
    hyperai_data::saved_models(data_root).map_err(InterfaceError::from)
}

/// List saved display views and whether their model digest remains linked.
///
/// # Errors
///
/// Returns an error if a saved view definition or its model catalog is invalid.
pub fn data_views_v1(data_root: &std::path::Path) -> Result<DataViewCatalogV1, InterfaceError> {
    hyperai_data::saved_views(data_root).map_err(InterfaceError::from)
}

/// Create or verify the deterministic local synthetic JSON and Parquet sample.
///
/// # Errors
///
/// Returns a storage, path-safety, or existing-bytes conflict error.
pub fn seed_data_synthetic_v1(
    data_root: &std::path::Path,
) -> Result<DatasetManifestV1, InterfaceError> {
    hyperai_data::seed_synthetic(data_root).map_err(InterfaceError::from)
}

/// Preview the deterministic local JSON/Parquet sample seed.
///
/// # Errors
///
/// Returns an error if the versioned seed identity cannot be hashed.
pub fn preview_data_synthetic_seed_v1() -> Result<DefinitionPreviewV1, InterfaceError> {
    hyperai_data::synthetic_seed_preview().map_err(InterfaceError::from)
}

/// Apply the deterministic local seed after digest and idempotency checks.
///
/// # Errors
///
/// Returns an error if the preview, idempotency key, path, or existing bytes
/// conflict with the versioned operation.
pub fn seed_data_synthetic_confirmed_v1(
    data_root: &std::path::Path,
    idempotency_key: &str,
    preview_digest: Sha256Digest,
) -> Result<DatasetManifestV1, InterfaceError> {
    hyperai_data::seed_synthetic_confirmed(data_root, idempotency_key, preview_digest)
        .map_err(InterfaceError::from)
}

/// Read one local dataset manifest and verify its source bytes.
///
/// # Errors
///
/// Returns an error for an absent dataset, unsafe path, or integrity mismatch.
pub fn describe_data_v1(
    data_root: &std::path::Path,
    dataset_id: &str,
) -> Result<DatasetManifestV1, InterfaceError> {
    hyperai_data::describe(data_root, dataset_id).map_err(InterfaceError::from)
}

/// Execute a bounded read-only query over the local `HyperAI` catalog.
///
/// # Errors
///
/// Returns an error when the query, source, resource use, or cancellation fails.
pub async fn query_data_v1(
    data_root: &std::path::Path,
    request: &QueryRequestV1,
    cancellation: tokio::sync::watch::Receiver<bool>,
) -> Result<QueryResultV1, InterfaceError> {
    hyperai_data::query_with_cancellation(data_root, request, cancellation)
        .await
        .map_err(InterfaceError::from)
}

/// Preview one model definition against the exact local dataset snapshot.
///
/// # Errors
///
/// Returns an error for invalid SQL, stale input, or an invalid model name.
pub fn preview_data_model_v1(
    data_root: &std::path::Path,
    model: &DataModelV1,
) -> Result<DefinitionPreviewV1, InterfaceError> {
    hyperai_data::preview_model(data_root, model).map_err(InterfaceError::from)
}

/// Persist one previewed model under the local `HyperAI` data root.
///
/// # Errors
///
/// Returns an error for stale preview, idempotency conflict, or unsafe storage.
pub fn save_data_model_v1(
    data_root: &std::path::Path,
    model: &DataModelV1,
    idempotency_key: &str,
    preview_digest: Sha256Digest,
) -> Result<DefinitionSavedV1, InterfaceError> {
    hyperai_data::save_model(data_root, model, idempotency_key, preview_digest)
        .map_err(InterfaceError::from)
}

/// Preview one table/chart view definition against a saved model digest.
///
/// # Errors
///
/// Returns an error for an invalid view name, presentation, or column set.
pub fn preview_data_view_v1(
    data_root: &std::path::Path,
    view: &DataViewV1,
) -> Result<DefinitionPreviewV1, InterfaceError> {
    hyperai_data::preview_view(data_root, view).map_err(InterfaceError::from)
}

/// Persist one previewed view under the local `HyperAI` data root.
///
/// # Errors
///
/// Returns an error for stale preview, idempotency conflict, or unsafe storage.
pub fn save_data_view_v1(
    data_root: &std::path::Path,
    view: &DataViewV1,
    idempotency_key: &str,
    preview_digest: Sha256Digest,
) -> Result<DefinitionSavedV1, InterfaceError> {
    hyperai_data::save_view(data_root, view, idempotency_key, preview_digest)
        .map_err(InterfaceError::from)
}

/// Two caller-selected raw JSON documents for an in-memory `HyperIntel` bundle check.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HyperIntelBundleRequestV1 {
    /// `HyperIntel` CLI-compatible V1–V4 evidence input.
    pub evidence_input: String,
    /// Caller-controlled public verification keys.
    pub trusted_keys_input: String,
}

/// Two caller-selected raw JSON documents for an in-memory execution
/// reconciliation. The lineage can contain execution-stream text; use only
/// synthetic or otherwise explicitly authorized non-sensitive evidence.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HyperIntelReconciliationRequestV1 {
    /// Complete `HyperIntel` execution, journal, replay, and route input.
    pub execution_input: String,
    /// Caller-controlled public verification keys.
    pub trusted_keys_input: String,
}

/// Two caller-selected raw JSON documents for an in-memory registry capture check.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimpleMarketCaptureRequestV1 {
    /// Signed buyer request and raw registry response body.
    pub capture_json: String,
    /// Caller-controlled buyer and registry public identity pins.
    pub trust_json: String,
}

#[allow(clippy::too_many_arguments)] // Each call lists one complete static descriptor for review.
fn connection_descriptor(
    module_id: &'static str,
    name: &'static str,
    owner: &'static str,
    source_revision: Option<&'static str>,
    connection_mode: ConnectionModeV1,
    state: &'static str,
    boundary: &'static str,
    exposed_capability_ids: &[&'static str],
    gate_id: &'static str,
    gate_summary: &'static str,
) -> ConnectionDescriptorV1 {
    ConnectionDescriptorV1 {
        module_id,
        name,
        owner,
        source_revision,
        connection_mode,
        state,
        boundary,
        exposed_capability_ids: exposed_capability_ids.to_vec(),
        next_blocked_gate: ConnectionGateV1 {
            id: gate_id,
            summary: gate_summary,
        },
    }
}

/// Return the static, versioned connection inventory shared by first-party clients.
///
/// This metadata is build-time inventory only. It performs no external probe,
/// connection attempt, storage operation, or capability authorization.
#[must_use]
#[allow(clippy::too_many_lines)] // The ordered static catalog stays together as one auditable contract.
pub fn interface_connection_inventory_v1() -> ConnectionInventoryV1 {
    use ConnectionModeV1::{
        LocalSynthetic, OfflineCallerSelected, Planned, ReferenceOnly, WorkspaceNotConnected,
    };

    ConnectionInventoryV1 {
        schema_version: "hyperai.interface.connection-inventory.v1",
        mode: "static_local_inventory",
        authority_note: "Descriptive local inventory only; not health, trust, rights, authorization, execution, or payment authority.",
        connections: vec![
            connection_descriptor(
                "compute_market",
                "HyperAI Compute Market",
                "HyperAI market contracts",
                None,
                LocalSynthetic,
                "local_synthetic",
                "In-process synthetic catalog, deal, receipt, and mock disposition",
                &[
                    "hyperai.market.synthetic_workload_trace.v1",
                    "hyperai.market.mock_deal_lifecycle.v1",
                    "hyperai.market.supply_comparison.v1",
                ],
                "seller_registry_rights_and_authorization",
                "Live discovery or seller negotiation needs a rights-cleared source contract and separate buyer and seller authorization.",
            ),
            connection_descriptor(
                "simple_compute_market",
                "Simple Compute Market",
                "Simple Compute Market",
                Some("a1c46d20b1b2f1aabe53e1b052921a87cf9d7360"),
                OfflineCallerSelected,
                "local_offline_capture_verifier_available",
                "Offline v2 GET /listings capture verification under caller pins; no registry or seller network access",
                &["hyperai.scm.offline_listings_capture_verification.v1"],
                "live_registry_read_contract_and_authorization",
                "Live registry reads require a source contract, rights review, and separate authorization.",
            ),
            connection_descriptor(
                "compute_bazaar",
                "The Compute Bazaar",
                "The Compute Bazaar",
                Some("d1809e5cfa843c087ebc4de9f00561bf0be67d55"),
                ReferenceOnly,
                "reference_only_rights_unresolved",
                "Architecture reference only; no source or data is loaded by this app",
                &[],
                "written_rights_for_exact_materials",
                "Written rights must cover each exact source, data, fixture, asset, and dependency before reuse.",
            ),
            connection_descriptor(
                "hyperintel",
                "HyperIntel",
                "HyperIntel",
                Some("d52111a07baa5fc4a7f6bfb4407731e2b9f609dc"),
                OfflineCallerSelected,
                "local_offline_verification_and_reconciliation_available",
                "Offline V1–V4 bundle verification and pure-data execution-lineage reconciliation; routing and execution remain owner-controlled",
                &[
                    "hyperai.hyperintel.offline_bundle_verification.v1",
                    "hyperai.hyperintel.offline_lineage_reconciliation.v1",
                ],
                "hyperintel_routing_or_provider_authority",
                "Routing or provider calls require a separate HyperIntel-owned authorization slice.",
            ),
            connection_descriptor(
                "fleet",
                "HyperAI Fleet",
                "HyperAI Fleet contracts",
                None,
                LocalSynthetic,
                "local_synthetic_fixture_only",
                "Fixed illustrative inventory and non-executable operation previews; no host probe or provider adapter",
                &[
                    "hyperai.fleet.synthetic_inventory.v1",
                    "hyperai.fleet.fixture_doctor.v1",
                    "hyperai.fleet.advisory_operation_preview.v1",
                    "hyperai.fleet.local_simulation.v1",
                ],
                "live_fleet_operator_authorization",
                "Live host or provider operations require a separately authorized Fleet operator slice.",
            ),
            connection_descriptor(
                "eval",
                "HyperAI Eval",
                "HyperAI Eval",
                None,
                LocalSynthetic,
                "local_synthetic_harness_only",
                "Fixed synthetic answer fixtures only; no model or agent execution",
                &[
                    "hyperai.eval.synthetic_fixture_catalog.v1",
                    "hyperai.eval.local_fixture_comparison.v1",
                ],
                "provider_or_paid_eval_authorization",
                "Provider calls or paid evaluation require a separate execution authorization.",
            ),
            connection_descriptor(
                "outcomes",
                "HyperAI Outcomes",
                "HyperAI outcome protocol",
                None,
                WorkspaceNotConnected,
                "workspace_present_not_bridged",
                "No compute-receipt-to-outcome mapping is enabled",
                &[],
                "matching_claim_and_evidence_contract",
                "A typed claim and evidence contract must be reviewed before bridging compute receipts.",
            ),
            connection_descriptor(
                "justai",
                "JustAI",
                "JustAI",
                Some("9a0e56536b968cbb2cb7173e142068b53c4f9fe3"),
                WorkspaceNotConnected,
                "workspace_present_not_connected",
                "No evidence projection, custody operation, or payment authority",
                &[],
                "owner_approved_bounded_projection",
                "Any JustAI projection requires its owner approval and a bounded evidence-only contract.",
            ),
            connection_descriptor(
                "esn_justnet",
                "ESN-JustNet",
                "ESN-JustNet",
                None,
                Planned,
                "planned_not_in_runtime",
                "No preference, consent, budget, or revocation adapter",
                &[],
                "buyer_preference_adapter_authorization",
                "A buyer-owned preference adapter needs its own consent, budget, and revocation authorization.",
            ),
            connection_descriptor(
                "trade",
                "Trade",
                "HyperAI Trade research contracts",
                None,
                LocalSynthetic,
                "local_synthetic_research_only",
                "Versioned synthetic asking-price scenarios and paper procurement-cost replay; no orders, forecasts, or financial authority",
                &["hyperai.trade.synthetic_research.v1"],
                "financial_product_and_oracle_plan",
                "Financial contracts require a separate product, oracle, and operator plan.",
            ),
            connection_descriptor(
                "data",
                "HyperAI Data",
                "HyperAI Data contracts",
                None,
                LocalSynthetic,
                "local_synthetic_workspace",
                "Manifest-verified local JSON/Parquet snapshots and bounded read-only queries; no external ingestion",
                &[
                    "hyperai.data.local_catalog.v1",
                    "hyperai.data.bounded_read_query.v1",
                    "hyperai.data.saved_definitions.v1",
                ],
                "phase2_acceptance_and_source_specific_rights",
                "Broader ingestion requires Phase 2 acceptance and rights for each source.",
            ),
        ],
    }
}

/// Return the compatibility status projection of the static connection catalog.
///
/// The schema and values remain fixed for `hyperai.interface.status.v1`.
#[must_use]
pub fn interface_status() -> InterfaceStatusV1 {
    let inventory = interface_connection_inventory_v1();
    InterfaceStatusV1 {
        schema_version: "hyperai.interface.status.v1",
        mode: "local_synthetic_interface",
        connections: inventory
            .connections
            .into_iter()
            .map(|item| ModuleConnectionV1 {
                module_id: item.module_id,
                name: item.name,
                state: item.state,
                owner: item.owner,
                boundary: item.boundary,
            })
            .collect(),
    }
}

/// Run the existing deterministic synthetic catalog-to-mock-settlement contract.
///
/// This function is in-memory and performs no file, network, provider, seller,
/// wallet, chain, or public-evidence operation.
///
/// # Errors
///
/// Returns the market contract error if locally constructed bytes fail validation.
pub fn run_local_market_demo() -> Result<LocalMarketDemoV1, InterfaceError> {
    let receipt = local_market_receipt()?;
    verify_local_market_receipt(&receipt)?;
    let report = local_demo_report(&receipt, false);
    Ok(LocalMarketDemoV1 {
        schema_version: "hyperai.interface.market-demo.v1",
        report,
        receipt,
        effect_boundary: "synthetic_in_memory_no_external_effects_no_funds_move",
    })
}

/// Build the shared fixed local synthetic buyer supply comparison.
///
/// This read projection shares the compute-market eligibility contract and
/// does not create a quote, approval, fulfillment, or settlement record.
///
/// # Errors
///
/// Returns an interface error when the source records or projection fail
/// contract validation.
pub fn market_supply_comparison_v1() -> Result<SupplyComparisonV1, InterfaceError> {
    local_market_supply_comparison_v1().map_err(InterfaceError::from)
}

/// Bind one displayed buyer supply comparison to its matching quote preview.
///
/// The market owner rebuilds both records from one fixed local fixture and
/// rejects a stale comparison digest before returning the preview.
///
/// # Errors
///
/// Returns an interface error if the comparison changed, has no eligible
/// offer, or any source/quote digest binding fails validation.
pub fn market_supply_quote_preview_v1(
    request: &SupplyQuotePreviewRequestV1,
) -> Result<SupplyQuotePreviewV1, InterfaceError> {
    let preview = local_market_supply_quote_preview_v1(request.comparison_digest)?;
    verify_supply_quote_preview_v1(&preview)?;
    Ok(preview)
}

/// Reconstruct the fixed local synthetic workload-to-receipt trace.
///
/// The Data reference is included only when the source-owned local catalog
/// contains the fixed synthetic market-observations dataset. Eval, Fleet, and
/// Trade appear as clearly labelled context records; they are not represented
/// as workload execution, completion, or price authority. `HyperIntel`,
/// `Outcomes`, and `JustAI` records are omitted unless a caller-selected source
/// contract is added in a later slice.
///
/// This function reconstructs the existing deterministic market fixture in
/// memory. It does not persist a second ledger or perform execution, network,
/// provider, seller, verifier, settlement, or payment operations.
///
/// # Errors
///
/// Returns an owner-contract or canonicalization error if any included source
/// record or the resulting projection is invalid.
pub fn workload_trace_v1(
    data_root: Option<&std::path::Path>,
) -> Result<WorkloadTraceV1, InterfaceError> {
    const STATE_SLICE: &str = "hyperai.bazaar.workload_trace_synthetic.v1";
    const DATASET_ID: &str = "hyperai.synthetic_market_observations_json.v1";
    let data_link = load_workload_data_link(data_root, DATASET_ID);
    let fixture = local_workload_trace_fixture()?;
    compose_workload_trace(&fixture, &data_link, STATE_SLICE)
}

/// Build the fixed seller-attributed failure preview for the local interface.
///
/// This composes a signed failure report against the existing deterministic
/// approved-deal fixture. It does not execute a workload, persist state, or
/// make a refund or payment-entitlement decision.
///
/// # Errors
///
/// Returns an owner-contract or canonicalization error if any fixture record,
/// local Data link, or resulting projection is invalid.
pub fn workload_trace_seller_failure_preview_v1(
    data_root: Option<&std::path::Path>,
) -> Result<WorkloadTraceV1, InterfaceError> {
    let data_link =
        load_workload_data_link(data_root, "hyperai.synthetic_market_observations_json.v1");
    let fixture = local_workload_trace_fixture()?;
    let failure =
        fail_local_market_quote_preview(fixture.quote_preview.digest, fixture.report.deal_digest)?;
    compose_failure_workload_trace(&fixture, &failure, &data_link)
}

/// Build the fixed inconclusive-verifier and mock-dispute-hold preview.
///
/// This applies the existing local mock contracts to the deterministic
/// synthetic deal and fulfillment fixture. It does not invoke a verifier,
/// create a dispute, hold funds, or persist a record.
///
/// # Errors
///
/// Returns an owner-contract or canonicalization error if any fixture record,
/// mock disposition, local Data link, or resulting projection is invalid.
pub fn workload_trace_inconclusive_preview_v1(
    data_root: Option<&std::path::Path>,
) -> Result<WorkloadTraceV1, InterfaceError> {
    let data_link =
        load_workload_data_link(data_root, "hyperai.synthetic_market_observations_json.v1");
    let mut fixture = local_workload_trace_fixture()?;
    let recorded_at = fixture.receipt.body.mock_settlement.recorded_at.clone();
    let decision = mock_verifier_decision(
        &fixture.receipt.body.approved_deal,
        &fixture.receipt.body.fulfillment,
        CompletionVerdict::Inconclusive,
        "synthetic_fixture_has_no_independent_verifier".to_owned(),
    )?;
    let settlement = MockSettlementLedger::default().settle(
        &fixture.receipt.body.approved_deal,
        &fixture.receipt.body.fulfillment,
        &decision,
        recorded_at,
    )?;
    fixture.receipt.body.verifier_decision = decision;
    fixture.receipt.body.mock_settlement = settlement;
    fixture.receipt.digest = domain_separated_digest(
        "hyperai.compute-market.local-receipt.v1",
        &fixture.receipt.body,
    )?;
    verify_local_market_receipt(&fixture.receipt)?;

    compose_workload_trace(&fixture, &data_link, WORKLOAD_TRACE_INCONCLUSIVE_SLICE)
}

/// Build one fixed unresolved Data-status fixture without reading Data.
///
/// The expected digest belongs only to the synthetic buyer intent. The linked
/// Data reference deliberately has no source-observed digest. This function
/// performs no catalog, manifest, or filesystem operation.
///
/// # Errors
///
/// Returns an owner-contract or canonicalization error if the fixed records
/// or resulting trace fail validation.
pub fn workload_trace_data_status_preview_v1(
    status: WorkloadTraceDataStatusPreviewV1,
) -> Result<WorkloadTraceV1, InterfaceError> {
    const DATASET_ID: &str = "hyperai.synthetic_market_observations_json.v1";
    let data_link = WorkloadDataLinkV1 {
        intent_reference: Some(WorkloadDataReferenceV1 {
            owner_id: "hyperai-data".to_owned(),
            schema_version: "hyperai.data.manifest.v1".to_owned(),
            record_id: DATASET_ID.to_owned(),
            digest: Sha256Digest::from_bytes([0x44; 32]),
        }),
        schema_version: "hyperai.data.manifest.v1".to_owned(),
        status: status.as_status(),
        link_basis: WORKLOAD_TRACE_DATA_STATUS_LINK_BASIS,
        claim_ceiling: WORKLOAD_TRACE_DATA_STATUS_CLAIM_CEILING,
    };
    let fixture = local_workload_trace_fixture()?;
    compose_workload_trace(&fixture, &data_link, WORKLOAD_TRACE_DATA_STATUS_SLICE)
}

/// Build the fixed same-deal duplicate-disposition replay preview.
///
/// The first in-memory settle result must match the existing synthetic
/// receipt. An exact second call must return `MarketError::Replay`. No second
/// receipt is constructed and no source, journal, or Data file is read.
///
/// # Errors
///
/// Returns an owner-contract or canonicalization error if either the existing
/// disposition or its exact retry differs from the fixed contract behavior.
pub fn workload_trace_duplicate_disposition_preview_v1() -> Result<WorkloadTraceV1, InterfaceError>
{
    let fixture = local_workload_trace_fixture()?;
    let recorded_at = fixture.receipt.body.mock_settlement.recorded_at.clone();
    let mut ledger = MockSettlementLedger::default();
    let first = ledger.settle(
        &fixture.receipt.body.approved_deal,
        &fixture.receipt.body.fulfillment,
        &fixture.receipt.body.verifier_decision,
        recorded_at.clone(),
    )?;
    if first != fixture.receipt.body.mock_settlement
        || !matches!(
            ledger.settle(
                &fixture.receipt.body.approved_deal,
                &fixture.receipt.body.fulfillment,
                &fixture.receipt.body.verifier_decision,
                recorded_at,
            ),
            Err(MarketError::Replay)
        )
    {
        return Err(
            MarketError::InvalidRecord("duplicate mock disposition did not return replay").into(),
        );
    }

    let data_link = WorkloadDataLinkV1 {
        intent_reference: None,
        schema_version: "not_selected".to_owned(),
        status: "not_requested",
        link_basis: "no_local_data_root_selected",
        claim_ceiling: "no_local_data_source_was_requested_or_read",
    };
    let intent = synthetic_workload_intent(&fixture.receipt.body.buyer_request, &data_link)?;
    let request_binding = bind_workload_request_v1(&intent, &fixture.receipt.body.buyer_request)?;
    verify_workload_request_binding_v1(
        &intent,
        &fixture.receipt.body.buyer_request,
        &request_binding,
    )?;
    let mut references = market_workload_references(&intent, &request_binding, &fixture)?;
    references.push(reference(
        "hyperai-market",
        &fixture.receipt.body.mock_settlement.schema_version,
        "synthetic-mock-disposition-replay-attempt-v1".to_owned(),
        None,
        "mock_settlement_replay_refused",
        "same_deal_retry_rejected_by_in_memory_mock_ledger",
        "replay_refusal_only_no_second_mock_receipt_or_funds_move",
    ));
    references.extend(context_workload_references(&data_link)?);
    sort_workload_references(&mut references);
    build_workload_trace(
        WORKLOAD_TRACE_DUPLICATE_DISPOSITION_SLICE,
        intent,
        request_binding,
        references,
    )
}

fn compose_workload_trace(
    fixture: &LocalWorkloadTraceFixtureV1,
    data_link: &WorkloadDataLinkV1,
    state_slice: &'static str,
) -> Result<WorkloadTraceV1, InterfaceError> {
    let intent = synthetic_workload_intent(&fixture.receipt.body.buyer_request, data_link)?;
    let request_binding = bind_workload_request_v1(&intent, &fixture.receipt.body.buyer_request)?;
    verify_workload_request_binding_v1(
        &intent,
        &fixture.receipt.body.buyer_request,
        &request_binding,
    )?;

    let mut references = market_workload_references(&intent, &request_binding, fixture)?;
    references.extend(context_workload_references(data_link)?);
    sort_workload_references(&mut references);
    build_workload_trace(state_slice, intent, request_binding, references)
}

fn compose_failure_workload_trace(
    fixture: &LocalWorkloadTraceFixtureV1,
    failure: &FulfillmentFailureReceiptV1,
    data_link: &WorkloadDataLinkV1,
) -> Result<WorkloadTraceV1, InterfaceError> {
    let deal = &fixture.receipt.body.approved_deal;
    verify_fulfillment_failure_receipt(deal, failure)?;
    let intent = synthetic_workload_intent(&fixture.receipt.body.buyer_request, data_link)?;
    let request_binding = bind_workload_request_v1(&intent, &fixture.receipt.body.buyer_request)?;
    verify_workload_request_binding_v1(
        &intent,
        &fixture.receipt.body.buyer_request,
        &request_binding,
    )?;

    let mut references = market_prefulfillment_references(&intent, &request_binding, fixture)?;
    references.extend(market_approved_deal_references(fixture)?);
    references.push(reference(
        "hyperai-market",
        &failure.schema_version,
        failure.attempt_id.clone(),
        Some(fulfillment_failure_digest_v1(failure)?),
        "fulfillment_failed",
        "seller_signed_failure_report_bound_to_approved_deal",
        "seller_attributed_failure_report_only_not_independent_execution_or_payment_evidence",
    ));
    references.extend(context_workload_references(data_link)?);
    sort_workload_references(&mut references);
    build_workload_trace(
        WORKLOAD_TRACE_FAILURE_SLICE,
        intent,
        request_binding,
        references,
    )
}

struct WorkloadDataLinkV1 {
    intent_reference: Option<WorkloadDataReferenceV1>,
    schema_version: String,
    status: &'static str,
    link_basis: &'static str,
    claim_ceiling: &'static str,
}

struct LocalWorkloadTraceFixtureV1 {
    receipt: LocalMarketReceiptV1,
    report: LocalDemoReportV1,
    quote_preview: LocalMarketQuotePreviewV1,
}

#[derive(Serialize)]
struct SyntheticDigestInputV1<'a> {
    schema_version: &'a str,
    value: &'a str,
}

fn local_workload_trace_fixture() -> Result<LocalWorkloadTraceFixtureV1, InterfaceError> {
    let demo = run_local_market_demo()?;
    let quote_preview = local_market_quote_preview()?;
    if quote_preview.body.buyer_request != demo.receipt.body.buyer_request
        || quote_preview.body.signed_quote != demo.receipt.body.approved_deal.signed_quote
    {
        return Err(MarketError::InvalidRecord(
            "synthetic quote preview and receipt do not share source records",
        )
        .into());
    }
    Ok(LocalWorkloadTraceFixtureV1 {
        receipt: demo.receipt,
        report: demo.report,
        quote_preview,
    })
}

fn load_workload_data_link(
    data_root: Option<&std::path::Path>,
    dataset_id: &str,
) -> WorkloadDataLinkV1 {
    let Some(root) = data_root else {
        return WorkloadDataLinkV1 {
            intent_reference: None,
            schema_version: "not_selected".to_owned(),
            status: "not_requested",
            link_basis: "no_local_data_root_selected",
            claim_ceiling: "no_local_data_source_was_requested_or_read",
        };
    };
    let verified_status = hyperai_data::inspect_dataset_status_v1(root, dataset_id);
    let manifest_schema = "hyperai.data.manifest.v1".to_owned();
    match verified_status {
        hyperai_data::DatasetReadStatusV1::NotSeeded => WorkloadDataLinkV1 {
            intent_reference: None,
            schema_version: manifest_schema,
            status: "not_seeded",
            link_basis: "local_catalog_does_not_contain_requested_dataset",
            claim_ceiling: "local_catalog_absence_only_no_external_availability_claim",
        },
        hyperai_data::DatasetReadStatusV1::Bound(digest) => WorkloadDataLinkV1 {
            intent_reference: Some(WorkloadDataReferenceV1 {
                owner_id: "hyperai-data".to_owned(),
                schema_version: manifest_schema.clone(),
                record_id: dataset_id.to_owned(),
                digest,
            }),
            schema_version: manifest_schema,
            status: "bound",
            link_basis: "verified_local_manifest_and_snapshot_content_digest",
            claim_ceiling: "local_synthetic_snapshot_integrity_only_no_market_or_execution_claim",
        },
        hyperai_data::DatasetReadStatusV1::Missing => WorkloadDataLinkV1 {
            intent_reference: None,
            schema_version: manifest_schema,
            status: "missing",
            link_basis: "local_manifest_referenced_file_was_missing",
            claim_ceiling: "local_read_missing_only_no_source_content_claim",
        },
        hyperai_data::DatasetReadStatusV1::Conflicting => WorkloadDataLinkV1 {
            intent_reference: None,
            schema_version: manifest_schema,
            status: "conflicting",
            link_basis: "local_catalog_manifest_or_content_integrity_rejected",
            claim_ceiling: "local_integrity_conflict_only_no_computation_claim",
        },
        hyperai_data::DatasetReadStatusV1::Denied => WorkloadDataLinkV1 {
            intent_reference: None,
            schema_version: manifest_schema,
            status: "denied",
            link_basis: "local_filesystem_permission_denied",
            claim_ceiling: "local_read_permission_denied_only_no_policy_entitlement_claim",
        },
        hyperai_data::DatasetReadStatusV1::Unavailable => WorkloadDataLinkV1 {
            intent_reference: None,
            schema_version: manifest_schema,
            status: "unavailable",
            link_basis: "local_data_read_unavailable_without_classification",
            claim_ceiling: "local_read_unavailable_only_no_source_claim",
        },
    }
}

fn synthetic_workload_intent(
    request: &hyperai_market::BuyerRequestV1,
    data_link: &WorkloadDataLinkV1,
) -> Result<WorkloadIntentV1, InterfaceError> {
    fn fixed_digest(domain: &str, value: &str) -> Result<Sha256Digest, InterfaceError> {
        domain_separated_digest(
            domain,
            &SyntheticDigestInputV1 {
                schema_version: domain,
                value,
            },
        )
        .map_err(InterfaceError::from)
    }

    Ok(create_workload_intent_v1(WorkloadIntentBodyV1 {
        schema_version: "hyperai.compute-market.workload-intent.v1".to_owned(),
        workload_id: "workload-hyperai-local-synthetic-v1".to_owned(),
        revision: 1,
        previous_revision_digest: None,
        specification_digest: fixed_digest(
            "hyperai.compute-market.synthetic-workload-spec.v1",
            "GPU-H100:us-east:2-units:3-hours",
        )?,
        acceptance_contract_digest: fixed_digest(
            "hyperai.compute-market.synthetic-acceptance-contract.v1",
            "local-fixture-structure-only-no-correctness-or-payment-claim",
        )?,
        data_handling_policy_digest: fixed_digest(
            "hyperai.compute-market.synthetic-data-policy.v1",
            "metadata-and-digests-only-no-private-payloads",
        )?,
        data_references: data_link.intent_reference.clone().into_iter().collect(),
        deadline: Some(request.valid_until.clone()),
        created_at: request.requested_at.clone(),
    })?)
}

fn market_workload_references(
    intent: &WorkloadIntentV1,
    binding: &WorkloadRequestBindingV1,
    fixture: &LocalWorkloadTraceFixtureV1,
) -> Result<Vec<WorkloadTraceReferenceV1>, InterfaceError> {
    let mut references = market_prefulfillment_references(intent, binding, fixture)?;
    references.extend(market_approved_deal_references(fixture)?);
    references.extend(market_completion_references(fixture));
    Ok(references)
}

fn market_prefulfillment_references(
    intent: &WorkloadIntentV1,
    binding: &WorkloadRequestBindingV1,
    fixture: &LocalWorkloadTraceFixtureV1,
) -> Result<Vec<WorkloadTraceReferenceV1>, InterfaceError> {
    let request = &fixture.receipt.body.buyer_request;
    let selected_offer = fixture
        .receipt
        .body
        .catalog
        .body
        .entries
        .iter()
        .find(|entry| entry.signed_offer.offer.offer_id == fixture.receipt.body.selected_offer_id)
        .ok_or(MarketError::InvalidRecord(
            "selected synthetic offer is absent",
        ))?;
    let references = vec![
        reference(
            "hyperai-market",
            "hyperai.compute-market.workload-intent.v1",
            intent.body.workload_id.clone(),
            Some(intent.digest),
            "bound",
            "immutable_buyer_intent_for_this_synthetic_request",
            "intent_metadata_and_digest_integrity_only",
        ),
        reference(
            "hyperai-market",
            &request.schema_version,
            "synthetic-buyer-request-v1".to_owned(),
            Some(binding.body.buyer_request_digest),
            "bound",
            "request_digest_verified_by_market_contract",
            "buyer_constraints_and_digest_link_only",
        ),
        reference(
            "hyperai-market",
            "hyperai.compute-market.catalog.v1",
            "local-synthetic-catalog-v1".to_owned(),
            Some(fixture.receipt.body.catalog.digest),
            "synthetic_source_record",
            "existing_deterministic_market_fixture",
            "synthetic_listing_contract_integrity_only",
        ),
        reference(
            "hyperai-market",
            "hyperai.compute-market.signed-offer.v1",
            selected_offer.signed_offer.offer.offer_id.clone(),
            Some(selected_offer.offer_digest),
            "synthetic_source_record",
            "selected_by_existing_buyer_request_contract",
            "fixture_listing_attribution_only",
        ),
        reference(
            "hyperai-market",
            &fixture.quote_preview.body.schema_version,
            "synthetic-quote-preview-v1".to_owned(),
            Some(fixture.quote_preview.digest),
            "synthetic_source_record",
            "preview_binds_same_request_catalog_offer_and_signed_quote",
            "synthetic_quote_preview_integrity_only",
        ),
    ];
    Ok(references)
}

fn market_approved_deal_references(
    fixture: &LocalWorkloadTraceFixtureV1,
) -> Result<Vec<WorkloadTraceReferenceV1>, InterfaceError> {
    let receipt = &fixture.receipt.body;
    let signed_quote_digest = domain_separated_digest(
        "hyperai.compute-market.signed-quote.v1",
        &receipt.approved_deal.signed_quote,
    )?;
    Ok(vec![
        reference(
            "hyperai-market",
            &receipt.approved_deal.signed_quote.quote.schema_version,
            "synthetic-seller-quote-v1".to_owned(),
            Some(signed_quote_digest),
            "synthetic_source_record",
            "seller_quote_signature_and_request_digest_verified",
            "fixture_quote_terms_and_signature_consistency_only",
        ),
        reference(
            "hyperai-market",
            &receipt.approved_deal.schema_version,
            "synthetic-approved-deal-v1".to_owned(),
            Some(fixture.report.deal_digest),
            "synthetic_source_record",
            "existing_buyer_approval_digest",
            "synthetic_approval_contract_integrity_only",
        ),
    ])
}

fn market_completion_references(
    fixture: &LocalWorkloadTraceFixtureV1,
) -> Vec<WorkloadTraceReferenceV1> {
    let receipt = &fixture.receipt.body;
    vec![
        reference(
            "hyperai-market",
            &receipt.fulfillment.schema_version,
            "synthetic-fulfillment-v1".to_owned(),
            Some(fixture.report.fulfillment_digest),
            "synthetic_source_record",
            "seller_fixture_statement_bound_to_approved_deal",
            "seller_attribution_and_receipt_integrity_only",
        ),
        reference(
            "hyperai-market",
            &receipt.verifier_decision.schema_version,
            "synthetic-mock-verifier-decision-v1".to_owned(),
            Some(receipt.mock_settlement.decision_digest),
            match &receipt.verifier_decision.verdict {
                CompletionVerdict::Verified => "mock_verdict_verified",
                CompletionVerdict::Rejected => "mock_verdict_rejected",
                CompletionVerdict::Inconclusive => "mock_verdict_inconclusive",
            },
            "decision_digest_bound_into_mock_disposition",
            "local_fixture_verdict_only_not_independent_correctness_evidence",
        ),
        reference(
            "hyperai-market",
            &receipt.mock_settlement.schema_version,
            "synthetic-mock-disposition-v1".to_owned(),
            None,
            match &receipt.mock_settlement.disposition {
                MockDisposition::ReleaseToSeller => "mock_disposition_release_to_seller",
                MockDisposition::RefundToBuyer => "mock_disposition_refund_to_buyer",
                MockDisposition::HoldForDispute => "mock_disposition_hold_for_dispute",
            },
            "covered_by_the_enclosing_local_market_receipt_digest",
            "no_payment_or_entitlement_claim",
        ),
        reference(
            "hyperai-market",
            &receipt.schema_version,
            "synthetic-local-market-receipt-v1".to_owned(),
            Some(fixture.receipt.digest),
            "synthetic_source_record",
            "existing_receipt_binds_the_full_mock_lifecycle",
            "market_contract_integrity_and_mock_lifecycle_only",
        ),
    ]
}

fn context_workload_references(
    data_link: &WorkloadDataLinkV1,
) -> Result<Vec<WorkloadTraceReferenceV1>, InterfaceError> {
    let eval = eval_catalog_v1()?;
    let fleet = fleet_inventory_v1()?;
    let trade = trade_catalog_v1()?;
    Ok(vec![
        reference(
            "hyperai-data",
            &data_link.schema_version,
            "hyperai.synthetic_market_observations_json.v1".to_owned(),
            if data_link.status == "bound" {
                data_link
                    .intent_reference
                    .as_ref()
                    .map(|reference| reference.digest)
            } else {
                None
            },
            data_link.status,
            data_link.link_basis,
            data_link.claim_ceiling,
        ),
        reference(
            "hyperai-eval",
            eval.schema_version,
            eval.suite_id.to_owned(),
            Some(eval.suite_digest),
            "advisory_only_not_run_for_this_workload",
            "fixed_eval_catalog_context_only",
            "synthetic_suite_identity_only_no_model_or_job_score",
        ),
        reference(
            "hyperai-fleet",
            fleet.schema_version,
            "synthetic-fleet-inventory-v1".to_owned(),
            Some(fleet.snapshot_digest),
            "advisory_only_not_workload_execution",
            "fixed_fleet_fixture_context_only",
            "illustrative_inventory_only_no_host_or_provider_state",
        ),
        reference(
            "hyperai-trade",
            trade.schema_version,
            trade.dataset_id.to_owned(),
            Some(trade.dataset_digest),
            "advisory_market_data_only",
            "fixed_trade_research_catalog_context_only",
            "synthetic_observations_only_no_quote_or_payment_authority",
        ),
    ])
}

#[allow(clippy::too_many_arguments)]
fn reference(
    owner_id: &'static str,
    schema_version: &str,
    record_id: String,
    digest: Option<Sha256Digest>,
    status: &'static str,
    link_basis: &'static str,
    claim_ceiling: &'static str,
) -> WorkloadTraceReferenceV1 {
    WorkloadTraceReferenceV1 {
        owner_id,
        schema_version: schema_version.to_owned(),
        record_id,
        digest,
        status,
        link_basis,
        claim_ceiling,
    }
}

fn sort_workload_references(references: &mut [WorkloadTraceReferenceV1]) {
    references.sort_by(|left, right| {
        (
            left.owner_id,
            left.schema_version.as_str(),
            left.record_id.as_str(),
        )
            .cmp(&(
                right.owner_id,
                right.schema_version.as_str(),
                right.record_id.as_str(),
            ))
    });
}

fn build_workload_trace(
    state_slice: &'static str,
    intent: WorkloadIntentV1,
    request_binding: WorkloadRequestBindingV1,
    references: Vec<WorkloadTraceReferenceV1>,
) -> Result<WorkloadTraceV1, InterfaceError> {
    let schema_version = "hyperai.interface.workload-trace.v1";
    let claim_ceiling = workload_trace_claim_ceiling(state_slice)
        .ok_or(MarketError::InvalidRecord("workload trace state slice"))?;
    let nonclaims = workload_trace_nonclaims(state_slice)
        .ok_or(MarketError::InvalidRecord("workload trace state slice"))?;
    let digest = domain_separated_digest(
        "hyperai.interface.workload-trace.v1",
        &WorkloadTraceDigestBodyV1 {
            schema_version,
            state_slice,
            intent: &intent,
            request_binding: &request_binding,
            references: &references,
            claim_ceiling,
            nonclaims,
        },
    )?;
    Ok(WorkloadTraceV1 {
        schema_version,
        state_slice,
        intent,
        request_binding,
        references,
        claim_ceiling,
        nonclaims,
        digest,
    })
}

/// Recheck the trace digest, intent, request-binding digest, and reference order.
///
/// # Errors
///
/// Returns an error if the projection or any embedded market identity record
/// was altered after construction.
pub fn verify_workload_trace_v1(trace: &WorkloadTraceV1) -> Result<(), InterfaceError> {
    hyperai_market::verify_workload_intent_v1(&trace.intent)?;
    if trace.schema_version != "hyperai.interface.workload-trace.v1"
        || workload_trace_claim_ceiling(trace.state_slice) != Some(trace.claim_ceiling)
        || workload_trace_nonclaims(trace.state_slice) != Some(trace.nonclaims)
        || !workload_trace_data_references_are_consistent(trace)
        || !workload_trace_data_status_preview_is_consistent(trace)
        || !workload_trace_failure_references_are_consistent(trace)
        || !workload_trace_duplicate_disposition_preview_is_consistent(trace)
        || trace.references.windows(2).any(|pair| {
            (
                pair[0].owner_id,
                pair[0].schema_version.as_str(),
                pair[0].record_id.as_str(),
            ) >= (
                pair[1].owner_id,
                pair[1].schema_version.as_str(),
                pair[1].record_id.as_str(),
            )
        })
        || trace.request_binding.body.workload_id != trace.intent.body.workload_id
        || trace.request_binding.body.intent_digest != trace.intent.digest
        || domain_separated_digest(
            "hyperai.compute-market.workload-request-binding.v1",
            &trace.request_binding.body,
        )? != trace.request_binding.digest
    {
        return Err(MarketError::InvalidRecord("workload trace fields or ordering").into());
    }
    let digest = domain_separated_digest(
        "hyperai.interface.workload-trace.v1",
        &WorkloadTraceDigestBodyV1 {
            schema_version: trace.schema_version,
            state_slice: trace.state_slice,
            intent: &trace.intent,
            request_binding: &trace.request_binding,
            references: &trace.references,
            claim_ceiling: trace.claim_ceiling,
            nonclaims: trace.nonclaims,
        },
    )?;
    if digest != trace.digest {
        return Err(MarketError::InvalidRecord("workload trace digest mismatch").into());
    }
    Ok(())
}

fn workload_trace_claim_ceiling(state_slice: &str) -> Option<&'static str> {
    match state_slice {
        WORKLOAD_TRACE_SUCCESS_SLICE => Some(WORKLOAD_TRACE_SUCCESS_CLAIM),
        WORKLOAD_TRACE_FAILURE_SLICE => Some(WORKLOAD_TRACE_FAILURE_CLAIM),
        WORKLOAD_TRACE_INCONCLUSIVE_SLICE => Some(WORKLOAD_TRACE_INCONCLUSIVE_CLAIM),
        WORKLOAD_TRACE_DATA_STATUS_SLICE => Some(WORKLOAD_TRACE_DATA_STATUS_CLAIM),
        WORKLOAD_TRACE_DUPLICATE_DISPOSITION_SLICE => {
            Some(WORKLOAD_TRACE_DUPLICATE_DISPOSITION_CLAIM)
        }
        _ => None,
    }
}

fn workload_trace_nonclaims(state_slice: &str) -> Option<&'static str> {
    match state_slice {
        WORKLOAD_TRACE_DATA_STATUS_SLICE => Some(WORKLOAD_TRACE_DATA_STATUS_NONCLAIMS),
        WORKLOAD_TRACE_DUPLICATE_DISPOSITION_SLICE => {
            Some(WORKLOAD_TRACE_DUPLICATE_DISPOSITION_NONCLAIMS)
        }
        WORKLOAD_TRACE_SUCCESS_SLICE
        | WORKLOAD_TRACE_FAILURE_SLICE
        | WORKLOAD_TRACE_INCONCLUSIVE_SLICE => Some(WORKLOAD_TRACE_NONCLAIMS),
        _ => None,
    }
}

fn is_workload_trace_data_status(status: &str) -> bool {
    matches!(status, "missing" | "conflicting" | "denied")
}

fn workload_trace_data_status_preview_is_consistent(trace: &WorkloadTraceV1) -> bool {
    if trace.state_slice != WORKLOAD_TRACE_DATA_STATUS_SLICE {
        return true;
    }

    let data_references = trace
        .references
        .iter()
        .filter(|reference| reference.owner_id == "hyperai-data")
        .collect::<Vec<_>>();
    let intent_references = &trace.intent.body.data_references;
    data_references.len() == 1
        && intent_references.len() == 1
        && is_workload_trace_data_status(data_references[0].status)
        && data_references[0].schema_version == "hyperai.data.manifest.v1"
        && data_references[0].record_id == "hyperai.synthetic_market_observations_json.v1"
        && data_references[0].digest.is_none()
        && data_references[0].link_basis == WORKLOAD_TRACE_DATA_STATUS_LINK_BASIS
        && data_references[0].claim_ceiling == WORKLOAD_TRACE_DATA_STATUS_CLAIM_CEILING
        && intent_references[0].owner_id == "hyperai-data"
        && intent_references[0].schema_version == "hyperai.data.manifest.v1"
        && intent_references[0].record_id == "hyperai.synthetic_market_observations_json.v1"
        && intent_references[0].digest == Sha256Digest::from_bytes([0x44; 32])
}

fn workload_trace_duplicate_disposition_preview_is_consistent(trace: &WorkloadTraceV1) -> bool {
    if trace.state_slice != WORKLOAD_TRACE_DUPLICATE_DISPOSITION_SLICE {
        return true;
    }

    let dispositions = trace
        .references
        .iter()
        .filter(|reference| reference.record_id == "synthetic-mock-disposition-v1")
        .collect::<Vec<_>>();
    let retry_attempts = trace
        .references
        .iter()
        .filter(|reference| reference.record_id == "synthetic-mock-disposition-replay-attempt-v1")
        .collect::<Vec<_>>();
    let receipts = trace
        .references
        .iter()
        .filter(|reference| reference.record_id == "synthetic-local-market-receipt-v1")
        .collect::<Vec<_>>();

    dispositions.len() == 1
        && dispositions[0].owner_id == "hyperai-market"
        && dispositions[0].schema_version == "hyperai.compute-market.mock-settlement.v1"
        && matches!(
            dispositions[0].status,
            "mock_disposition_release_to_seller"
                | "mock_disposition_refund_to_buyer"
                | "mock_disposition_hold_for_dispute"
        )
        && dispositions[0].digest.is_none()
        && dispositions[0].link_basis == "covered_by_the_enclosing_local_market_receipt_digest"
        && dispositions[0].claim_ceiling == "no_payment_or_entitlement_claim"
        && retry_attempts.len() == 1
        && retry_attempts[0].owner_id == "hyperai-market"
        && retry_attempts[0].schema_version == "hyperai.compute-market.mock-settlement.v1"
        && retry_attempts[0].status == "mock_settlement_replay_refused"
        && retry_attempts[0].digest.is_none()
        && retry_attempts[0].link_basis == "same_deal_retry_rejected_by_in_memory_mock_ledger"
        && retry_attempts[0].claim_ceiling
            == "replay_refusal_only_no_second_mock_receipt_or_funds_move"
        && receipts.len() == 1
        && receipts[0].owner_id == "hyperai-market"
        && receipts[0].schema_version == "hyperai.compute-market.local-receipt.v1"
        && receipts[0].status == "synthetic_source_record"
        && receipts[0].digest.is_some()
}

fn workload_trace_failure_references_are_consistent(trace: &WorkloadTraceV1) -> bool {
    let failure_references = trace
        .references
        .iter()
        .filter(|reference| {
            reference.owner_id == "hyperai-market"
                && reference.schema_version == "hyperai.compute-market.fulfillment-failure.v1"
        })
        .collect::<Vec<_>>();

    if trace.state_slice == WORKLOAD_TRACE_FAILURE_SLICE {
        failure_references.len() == 1
            && failure_references[0].status == "fulfillment_failed"
            && failure_references[0].digest.is_some()
            && failure_references[0].claim_ceiling
                == "seller_attributed_failure_report_only_not_independent_execution_or_payment_evidence"
            && trace.references.iter().any(|reference| {
                reference.record_id == "synthetic-approved-deal-v1"
                    && reference.owner_id == "hyperai-market"
            })
            && !trace.references.iter().any(|reference| {
                reference.record_id == "synthetic-fulfillment-v1"
                    || reference.record_id == "synthetic-mock-verifier-decision-v1"
                    || reference.record_id == "synthetic-mock-disposition-v1"
                    || reference.record_id == "synthetic-local-market-receipt-v1"
            })
    } else {
        failure_references.is_empty()
    }
}

fn workload_trace_data_references_are_consistent(trace: &WorkloadTraceV1) -> bool {
    let intent_references = &trace.intent.body.data_references;
    let data_references = trace
        .references
        .iter()
        .filter(|reference| reference.owner_id == "hyperai-data")
        .collect::<Vec<_>>();

    for intent_reference in intent_references {
        let Some(reference) = trace.references.iter().find(|reference| {
            reference.owner_id == intent_reference.owner_id
                && reference.schema_version == intent_reference.schema_version
                && reference.record_id == intent_reference.record_id
        }) else {
            return false;
        };

        match reference.status {
            "bound" | "stale" if reference.digest == Some(intent_reference.digest) => {}
            "missing" | "conflicting" | "denied" | "unavailable" if reference.digest.is_none() => {}
            _ => return false,
        }
    }

    data_references
        .into_iter()
        .all(|reference| match reference.status {
            "bound" | "stale" => intent_references.iter().any(|intent_reference| {
                reference.owner_id == intent_reference.owner_id
                    && reference.schema_version == intent_reference.schema_version
                    && reference.record_id == intent_reference.record_id
                    && reference.digest == Some(intent_reference.digest)
            }),
            "not_requested" | "not_seeded" | "missing" | "conflicting" | "denied"
            | "unavailable" => reference.digest.is_none(),
            _ => false,
        })
}

/// Return the deterministic synthetic quote preview without approving it.
///
/// # Errors
///
/// Returns a market contract error if the local preview cannot be built.
pub fn preview_local_market_quote_v1() -> Result<LocalMarketQuotePreviewV1, InterfaceError> {
    local_market_quote_preview().map_err(InterfaceError::from)
}

/// Build one member of the finite local synthetic quote fixture set.
///
/// # Errors
///
/// Returns a market contract error if the selected fixture cannot be
/// reconstructed.
pub fn preview_local_market_quote_scenario_v1(
    scenario: LocalMarketQuoteScenarioV1,
) -> Result<LocalMarketQuotePreviewV1, InterfaceError> {
    local_market_quote_preview_for_scenario(scenario).map_err(InterfaceError::from)
}

/// Approve only the exact deterministic quote preview identified by the
/// caller's digest. The market module reconstructs and validates the preview
/// before it produces a synthetic buyer signature.
///
/// # Errors
///
/// Returns a market contract error if the digest differs or approval fails.
pub fn approve_local_market_quote_v1(
    request: &LocalMarketQuoteApprovalRequestV1,
) -> Result<LocalMarketQuoteApprovalV1, InterfaceError> {
    approve_local_market_quote_preview_for_known_scenarios(request.preview_digest)
        .map_err(InterfaceError::from)
}

/// Complete only the synthetic deal identified by both reviewed digests.
///
/// The market module rebuilds the fixture, checks both digests, and verifies
/// the resulting receipt before the facade returns it.
///
/// # Errors
///
/// Returns a market contract error for a changed preview, changed approval, or
/// invalid generated lifecycle record.
pub fn complete_local_market_quote_v1(
    request: &LocalMarketQuoteCompletionRequestV1,
) -> Result<LocalMarketQuoteCompletionV1, InterfaceError> {
    let receipt = complete_local_market_quote_preview_for_known_scenarios(
        request.preview_digest,
        request.deal_digest,
    )
    .map_err(InterfaceError::from)?;
    verify_local_market_receipt(&receipt)?;
    let report = local_demo_report(&receipt, false);
    Ok(LocalMarketQuoteCompletionV1 {
        schema_version: "hyperai.interface.market-quote-completion.v1",
        state_slice: "hyperai.bazaar_interface.synthetic_quote_completion.v1",
        preview_digest: request.preview_digest,
        deal_digest: request.deal_digest,
        demo: LocalMarketDemoV1 {
            schema_version: "hyperai.interface.market-demo.v1",
            report,
            receipt,
            effect_boundary: "synthetic_in_memory_no_external_effects_no_funds_move",
        },
        effect_boundary: "local_synthetic_mock_fulfillment_verifier_and_settlement_only",
    })
}

/// Verify one receipt with the market module's existing offline contract check.
///
/// # Errors
///
/// Returns the market contract error if any nested digest, signature, or
/// lifecycle invariant is invalid.
pub fn verify_local_market_receipt_v1(
    receipt: &LocalMarketReceiptV1,
) -> Result<LocalMarketVerificationV1, InterfaceError> {
    verify_local_market_receipt(receipt)?;
    Ok(LocalMarketVerificationV1 {
        schema_version: "hyperai.interface.market-verification.v1",
        status: "local_contract_integrity_pass",
        receipt_digest: receipt.digest,
        claim_ceiling: receipt.body.claim_ceiling.clone(),
        nonclaims: receipt.body.nonclaims.clone(),
    })
}

/// Verify one imported `HyperIntel` V1–V4 market-evidence bundle offline.
///
/// Only the source-owned contract verifier is authoritative for bundle
/// reconstruction. Caller-supplied keys remain the trust root for signatures.
///
/// # Errors
///
/// Returns a closed error when the input, trust registry, schema, signature,
/// digest, or deterministic reconstruction is rejected.
pub fn verify_hyperintel_market_bundle_v1(
    request: &HyperIntelBundleRequestV1,
) -> Result<HyperIntelBundleVerificationV1, InterfaceError> {
    let input = hyperai_hyperintel_adapter::HyperIntelBundleInputV1 {
        evidence_input: request.evidence_input.clone(),
        trusted_keys_input: request.trusted_keys_input.clone(),
    };
    hyperai_hyperintel_adapter::verify_market_evidence_input_v1(&input)
        .map_err(InterfaceError::from)
}

/// Reconcile one caller-selected `HyperIntel` execution lineage offline.
///
/// The source-owned report is reconstructed and verified before it is returned.
/// Execution-stream data remains inside the request and is not included in the
/// result. This facade performs no provider, store, routing, execution, or
/// settlement operation.
///
/// # Errors
///
/// Returns a closed error when parsing, trust registration, or any
/// source-owned reconciliation invariant fails.
pub fn reconcile_hyperintel_execution_v1(
    request: &HyperIntelReconciliationRequestV1,
) -> Result<HyperIntelReconciliationV1, InterfaceError> {
    let input = hyperai_hyperintel_adapter::HyperIntelReconciliationInputV1 {
        execution_input: request.execution_input.clone(),
        trusted_keys_input: request.trusted_keys_input.clone(),
    };
    hyperai_hyperintel_adapter::reconcile_execution_input_v1(&input).map_err(InterfaceError::from)
}

/// Verify one caller-selected Simple Compute Market v2 listings capture offline.
///
/// Caller-supplied identity pins are displayed as caller-controlled. The
/// response does not cryptographically bind its query body, and the source
/// verifier does not maintain a replay store. The resulting listings cannot
/// enter offer selection or a deal through this facade.
///
/// # Errors
///
/// Returns a closed error when the capture, trust pins, source context,
/// timestamp, canonical body, or signature is rejected.
pub fn verify_simple_market_capture_v1(
    request: &SimpleMarketCaptureRequestV1,
) -> Result<ListingsCaptureVerificationV1, InterfaceError> {
    let input = hyperai_scm_adapter::ListingsCaptureInputV1 {
        capture_json: request.capture_json.clone(),
        trust_json: request.trust_json.clone(),
    };
    hyperai_scm_adapter::verify_listings_capture_now_v1(&input).map_err(InterfaceError::from)
}

#[cfg(test)]
mod workload_trace_tests {
    use std::fs;

    use super::{
        LocalWorkloadTraceFixtureV1, WORKLOAD_TRACE_DATA_STATUS_CLAIM,
        WORKLOAD_TRACE_DATA_STATUS_SLICE, WORKLOAD_TRACE_DUPLICATE_DISPOSITION_CLAIM,
        WORKLOAD_TRACE_DUPLICATE_DISPOSITION_SLICE, WORKLOAD_TRACE_FAILURE_CLAIM,
        WORKLOAD_TRACE_FAILURE_SLICE, WorkloadDataLinkV1, WorkloadTraceDataStatusPreviewV1,
        WorkloadTraceDigestBodyV1, WorkloadTraceV1, compose_failure_workload_trace,
        compose_workload_trace, domain_separated_digest, fail_local_market_quote_preview,
        fulfillment_failure_digest_v1, local_workload_trace_fixture, sort_workload_references,
        verify_workload_trace_v1, workload_trace_data_status_preview_v1,
        workload_trace_duplicate_disposition_preview_v1, workload_trace_inconclusive_preview_v1,
        workload_trace_seller_failure_preview_v1, workload_trace_v1,
    };
    use hyperai_market::{
        CompletionVerdict, MockDisposition, MockSettlementLedger, WorkloadDataReferenceV1,
        create_workload_intent_v1, mock_verifier_decision, verify_local_market_receipt,
    };

    fn trace_with_data_link(
        status: &'static str,
        linked_digest: Option<hyperai_foundation::Sha256Digest>,
    ) -> Result<WorkloadTraceV1, Box<dyn std::error::Error>> {
        let mut trace = workload_trace_v1(None)?;
        let data_reference = WorkloadDataReferenceV1 {
            owner_id: "hyperai-data".to_owned(),
            schema_version: "hyperai.data.manifest.v1".to_owned(),
            record_id: "hyperai.synthetic_market_observations_json.v1".to_owned(),
            digest: hyperai_foundation::Sha256Digest::from_bytes([0x44; 32]),
        };

        let mut intent_body = trace.intent.body.clone();
        intent_body.data_references = vec![data_reference.clone()];
        trace.intent = create_workload_intent_v1(intent_body)?;
        trace.request_binding.body.intent_digest = trace.intent.digest;
        trace.request_binding.digest = domain_separated_digest(
            "hyperai.compute-market.workload-request-binding.v1",
            &trace.request_binding.body,
        )?;

        let reference = trace
            .references
            .iter_mut()
            .find(|reference| reference.owner_id == "hyperai-data")
            .ok_or("synthetic trace has no Data context reference")?;
        reference.schema_version = data_reference.schema_version;
        reference.record_id = data_reference.record_id;
        reference.digest = linked_digest;
        reference.status = status;

        sort_workload_references(&mut trace.references);
        trace.digest = domain_separated_digest(
            "hyperai.interface.workload-trace.v1",
            &WorkloadTraceDigestBodyV1 {
                schema_version: trace.schema_version,
                state_slice: trace.state_slice,
                intent: &trace.intent,
                request_binding: &trace.request_binding,
                references: &trace.references,
                claim_ceiling: trace.claim_ceiling,
                nonclaims: trace.nonclaims,
            },
        )?;
        Ok(trace)
    }

    #[test]
    fn workload_trace_projects_only_owner_verified_data_digest()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "hyperai-workload-trace-data-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        let result = (|| {
            let unseeded = workload_trace_v1(Some(&root))?;
            verify_workload_trace_v1(&unseeded)?;
            let unseeded_reference = unseeded
                .references
                .iter()
                .find(|reference| reference.owner_id == "hyperai-data")
                .ok_or("unseeded trace omitted its Data reference")?;
            assert_eq!(unseeded_reference.status, "not_seeded");
            assert_eq!(unseeded_reference.digest, None);
            assert_eq!(
                unseeded_reference.link_basis,
                "local_catalog_does_not_contain_requested_dataset"
            );
            assert!(unseeded.intent.body.data_references.is_empty());

            let manifest = hyperai_data::seed_synthetic(&root)?;
            let bound = workload_trace_v1(Some(&root))?;
            verify_workload_trace_v1(&bound)?;
            assert_eq!(bound.intent.body.data_references.len(), 1);
            assert_eq!(
                bound.intent.body.data_references[0].digest,
                manifest.content_digest
            );
            let bound_reference = bound
                .references
                .iter()
                .find(|reference| reference.owner_id == "hyperai-data")
                .ok_or("bound trace omitted its Data reference")?;
            assert_eq!(bound_reference.status, "bound");
            assert_eq!(bound_reference.digest, Some(manifest.content_digest));
            assert_eq!(
                bound_reference.link_basis,
                "verified_local_manifest_and_snapshot_content_digest"
            );

            fs::remove_file(root.join("synthetic-market-observations.json"))?;
            let missing = workload_trace_v1(Some(&root))?;
            verify_workload_trace_v1(&missing)?;
            let missing_reference = missing
                .references
                .iter()
                .find(|reference| reference.owner_id == "hyperai-data")
                .ok_or("missing trace omitted its Data reference")?;
            assert_eq!(missing_reference.status, "missing");
            assert_eq!(missing_reference.digest, None);
            assert!(missing.intent.body.data_references.is_empty());

            hyperai_data::seed_synthetic(&root)?;
            fs::write(
                root.join("synthetic-market-observations.json"),
                b"changed local synthetic bytes\n",
            )?;
            let conflicting = workload_trace_v1(Some(&root))?;
            verify_workload_trace_v1(&conflicting)?;
            let conflict_reference = conflicting
                .references
                .iter()
                .find(|reference| reference.owner_id == "hyperai-data")
                .ok_or("conflicting trace omitted its Data reference")?;
            assert_eq!(conflict_reference.status, "conflicting");
            assert_eq!(conflict_reference.digest, None);
            assert!(conflicting.intent.body.data_references.is_empty());

            let unavailable_root = root.join("unavailable-data");
            let manifests = unavailable_root.join("manifests");
            fs::create_dir_all(&manifests)?;
            fs::write(
                manifests.join("hyperai.synthetic_market_observations_json.v1.json"),
                vec![b'x'; 16 * 1024 + 1],
            )?;
            let unavailable = workload_trace_v1(Some(&unavailable_root))?;
            verify_workload_trace_v1(&unavailable)?;
            let unavailable_reference = unavailable
                .references
                .iter()
                .find(|reference| reference.owner_id == "hyperai-data")
                .ok_or("unavailable trace omitted its Data reference")?;
            assert_eq!(unavailable_reference.status, "unavailable");
            assert_eq!(unavailable_reference.digest, None);
            assert!(unavailable.intent.body.data_references.is_empty());
            Ok(())
        })();
        let _ = fs::remove_dir_all(&root);
        result
    }

    fn reseal_trace(trace: &mut WorkloadTraceV1) -> Result<(), Box<dyn std::error::Error>> {
        trace.digest = domain_separated_digest(
            "hyperai.interface.workload-trace.v1",
            &WorkloadTraceDigestBodyV1 {
                schema_version: trace.schema_version,
                state_slice: trace.state_slice,
                intent: &trace.intent,
                request_binding: &trace.request_binding,
                references: &trace.references,
                claim_ceiling: trace.claim_ceiling,
                nonclaims: trace.nonclaims,
            },
        )?;
        Ok(())
    }

    #[test]
    fn trace_is_stable_digest_bound_and_keeps_context_advisory()
    -> Result<(), Box<dyn std::error::Error>> {
        let first = workload_trace_v1(None)?;
        let second = workload_trace_v1(None)?;
        verify_workload_trace_v1(&first)?;
        assert_eq!(first, second);
        assert_eq!(first.intent.body.data_references.len(), 0);
        assert!(first.references.iter().any(|reference| {
            reference.owner_id == "hyperai-data" && reference.status == "not_requested"
        }));
        assert!(first.references.iter().any(|reference| {
            reference.owner_id == "hyperai-eval"
                && reference.status == "advisory_only_not_run_for_this_workload"
        }));
        assert!(first.references.iter().any(|reference| {
            reference.owner_id == "hyperai-fleet"
                && reference.status == "advisory_only_not_workload_execution"
        }));
        assert!(first.references.iter().any(|reference| {
            reference.owner_id == "hyperai-trade" && reference.status == "advisory_market_data_only"
        }));
        Ok(())
    }

    #[test]
    fn trace_verifier_rejects_reference_tampering() -> Result<(), Box<dyn std::error::Error>> {
        let mut trace = workload_trace_v1(None)?;
        trace.references[0].record_id.push_str("-changed");
        assert!(verify_workload_trace_v1(&trace).is_err());
        Ok(())
    }

    #[test]
    fn trace_preserves_explicit_unresolved_data_links() -> Result<(), Box<dyn std::error::Error>> {
        for status in ["missing", "conflicting", "denied", "unavailable"] {
            let trace = trace_with_data_link(status, None)?;
            verify_workload_trace_v1(&trace)?;
            let reference = trace
                .references
                .iter()
                .find(|reference| reference.owner_id == "hyperai-data")
                .ok_or("Data reference disappeared from trace")?;
            assert_eq!(reference.status, status);
            assert_eq!(reference.digest, None);
        }

        let expected_digest = hyperai_foundation::Sha256Digest::from_bytes([0x44; 32]);
        let stale = trace_with_data_link("stale", Some(expected_digest))?;
        verify_workload_trace_v1(&stale)?;
        let bound = trace_with_data_link("bound", Some(expected_digest))?;
        verify_workload_trace_v1(&bound)?;
        Ok(())
    }

    #[test]
    fn duplicate_disposition_preview_binds_only_the_replay_refusal()
    -> Result<(), Box<dyn std::error::Error>> {
        let trace = workload_trace_duplicate_disposition_preview_v1()?;
        verify_workload_trace_v1(&trace)?;
        assert_eq!(
            trace.state_slice,
            WORKLOAD_TRACE_DUPLICATE_DISPOSITION_SLICE
        );
        assert_eq!(
            trace.claim_ceiling,
            WORKLOAD_TRACE_DUPLICATE_DISPOSITION_CLAIM
        );
        assert!(
            trace
                .nonclaims
                .contains("no second mock settlement receipt")
        );
        assert!(trace.nonclaims.contains("funds movement"));

        let disposition = trace
            .references
            .iter()
            .find(|reference| reference.record_id == "synthetic-mock-disposition-v1")
            .ok_or("first synthetic disposition was omitted")?;
        assert_eq!(disposition.status, "mock_disposition_release_to_seller");
        assert_eq!(disposition.digest, None);

        let retry = trace
            .references
            .iter()
            .find(|reference| reference.record_id == "synthetic-mock-disposition-replay-attempt-v1")
            .ok_or("duplicate retry result was omitted")?;
        assert_eq!(retry.status, "mock_settlement_replay_refused");
        assert_eq!(retry.digest, None);
        assert_eq!(
            retry.claim_ceiling,
            "replay_refusal_only_no_second_mock_receipt_or_funds_move"
        );
        assert_eq!(
            trace
                .references
                .iter()
                .filter(|reference| reference.record_id == "synthetic-local-market-receipt-v1")
                .count(),
            1
        );

        let mut conflicting = trace.clone();
        let replay = conflicting
            .references
            .iter_mut()
            .find(|reference| reference.record_id == "synthetic-mock-disposition-replay-attempt-v1")
            .ok_or("duplicate retry result was omitted")?;
        replay.status = "mock_disposition_refunded";
        assert!(verify_workload_trace_v1(&conflicting).is_err());
        Ok(())
    }

    #[test]
    fn trace_rejects_missing_or_conflicting_data_digest_claims()
    -> Result<(), Box<dyn std::error::Error>> {
        let absent_digest = trace_with_data_link("bound", None)?;
        assert!(verify_workload_trace_v1(&absent_digest).is_err());

        let conflicting_digest = trace_with_data_link(
            "bound",
            Some(hyperai_foundation::Sha256Digest::from_bytes([0x45; 32])),
        )?;
        assert!(verify_workload_trace_v1(&conflicting_digest).is_err());

        let mut unbound = workload_trace_v1(None)?;
        let data_reference = unbound
            .references
            .iter_mut()
            .find(|reference| reference.owner_id == "hyperai-data")
            .ok_or("synthetic trace has no Data context reference")?;
        data_reference.status = "bound";
        data_reference.digest = Some(hyperai_foundation::Sha256Digest::from_bytes([0x44; 32]));
        unbound.digest = domain_separated_digest(
            "hyperai.interface.workload-trace.v1",
            &WorkloadTraceDigestBodyV1 {
                schema_version: unbound.schema_version,
                state_slice: unbound.state_slice,
                intent: &unbound.intent,
                request_binding: &unbound.request_binding,
                references: &unbound.references,
                claim_ceiling: unbound.claim_ceiling,
                nonclaims: unbound.nonclaims,
            },
        )?;
        assert!(verify_workload_trace_v1(&unbound).is_err());
        Ok(())
    }

    #[test]
    fn inconclusive_mock_verdict_remains_visible_as_dispute_hold_in_trace()
    -> Result<(), Box<dyn std::error::Error>> {
        let trace = workload_trace_inconclusive_preview_v1(None)?;
        verify_workload_trace_v1(&trace)?;
        assert_eq!(
            trace.state_slice,
            "hyperai.bazaar.workload_trace_inconclusive_served.v1"
        );
        assert_eq!(
            trace.claim_ceiling,
            "digest-bound local synthetic inconclusive mock-outcome preview"
        );

        let verdict = trace
            .references
            .iter()
            .find(|reference| reference.record_id == "synthetic-mock-verifier-decision-v1")
            .ok_or("trace omitted the mock verifier decision")?;
        assert_eq!(verdict.status, "mock_verdict_inconclusive");
        assert_eq!(
            verdict.claim_ceiling,
            "local_fixture_verdict_only_not_independent_correctness_evidence"
        );

        let disposition = trace
            .references
            .iter()
            .find(|reference| reference.record_id == "synthetic-mock-disposition-v1")
            .ok_or("trace omitted the mock disposition")?;
        assert_eq!(disposition.status, "mock_disposition_hold_for_dispute");
        assert_eq!(disposition.digest, None);
        assert_eq!(disposition.claim_ceiling, "no_payment_or_entitlement_claim");
        Ok(())
    }

    #[test]
    fn data_status_previews_bind_expected_input_without_claiming_a_source_read()
    -> Result<(), Box<dyn std::error::Error>> {
        for (status, expected) in [
            (WorkloadTraceDataStatusPreviewV1::Missing, "missing"),
            (WorkloadTraceDataStatusPreviewV1::Conflicting, "conflicting"),
            (WorkloadTraceDataStatusPreviewV1::Denied, "denied"),
        ] {
            let trace = workload_trace_data_status_preview_v1(status)?;
            verify_workload_trace_v1(&trace)?;
            assert_eq!(trace.state_slice, WORKLOAD_TRACE_DATA_STATUS_SLICE);
            assert_eq!(trace.claim_ceiling, WORKLOAD_TRACE_DATA_STATUS_CLAIM);
            assert!(trace.nonclaims.contains("no Data source was read"));

            let intent_reference = trace
                .intent
                .body
                .data_references
                .first()
                .ok_or("synthetic intent omitted expected Data reference")?;
            assert_eq!(intent_reference.owner_id, "hyperai-data");
            assert_eq!(intent_reference.schema_version, "hyperai.data.manifest.v1");
            assert_eq!(
                intent_reference.record_id,
                "hyperai.synthetic_market_observations_json.v1"
            );
            assert_eq!(
                intent_reference.digest,
                hyperai_foundation::Sha256Digest::from_bytes([0x44; 32])
            );
            assert_eq!(
                trace.request_binding.body.intent_digest,
                trace.intent.digest
            );

            let data_references = trace
                .references
                .iter()
                .filter(|reference| reference.owner_id == "hyperai-data")
                .collect::<Vec<_>>();
            assert_eq!(data_references.len(), 1);
            assert_eq!(data_references[0].status, expected);
            assert_eq!(data_references[0].digest, None);
            assert_eq!(
                data_references[0].link_basis,
                "fixed_synthetic_status_fixture_no_data_source_read"
            );
            assert_eq!(
                data_references[0].claim_ceiling,
                "synthetic_data_status_fixture_only_no_source_read_conflict_or_access_decision"
            );
        }

        assert_eq!(
            WorkloadTraceDataStatusPreviewV1::from_status("missing"),
            Some(WorkloadTraceDataStatusPreviewV1::Missing)
        );
        assert_eq!(
            WorkloadTraceDataStatusPreviewV1::from_status("available"),
            None
        );
        Ok(())
    }

    #[test]
    fn rejected_mock_verdict_remains_visible_as_buyer_refund_in_trace()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut fixture: LocalWorkloadTraceFixtureV1 = local_workload_trace_fixture()?;
        let recorded_at = fixture.receipt.body.mock_settlement.recorded_at.clone();
        let decision = mock_verifier_decision(
            &fixture.receipt.body.approved_deal,
            &fixture.receipt.body.fulfillment,
            CompletionVerdict::Rejected,
            "synthetic_fixture_rejected".to_owned(),
        )?;
        let settlement = MockSettlementLedger::default().settle(
            &fixture.receipt.body.approved_deal,
            &fixture.receipt.body.fulfillment,
            &decision,
            recorded_at,
        )?;
        fixture.receipt.body.verifier_decision = decision;
        fixture.receipt.body.mock_settlement = settlement;
        fixture.receipt.digest = domain_separated_digest(
            "hyperai.compute-market.local-receipt.v1",
            &fixture.receipt.body,
        )?;
        verify_local_market_receipt(&fixture.receipt)?;

        let trace = compose_workload_trace(
            &fixture,
            &WorkloadDataLinkV1 {
                intent_reference: None,
                schema_version: "not_selected".to_owned(),
                status: "not_requested",
                link_basis: "no_local_data_root_selected",
                claim_ceiling: "no_local_data_source_was_requested_or_read",
            },
            "hyperai.bazaar.workload_trace_synthetic.v1",
        )?;
        verify_workload_trace_v1(&trace)?;

        let verdict = trace
            .references
            .iter()
            .find(|reference| reference.record_id == "synthetic-mock-verifier-decision-v1")
            .ok_or("trace omitted the mock verifier decision")?;
        assert_eq!(verdict.status, "mock_verdict_rejected");
        assert_eq!(
            verdict.claim_ceiling,
            "local_fixture_verdict_only_not_independent_correctness_evidence"
        );

        let disposition = trace
            .references
            .iter()
            .find(|reference| reference.record_id == "synthetic-mock-disposition-v1")
            .ok_or("trace omitted the mock disposition")?;
        assert_eq!(disposition.status, "mock_disposition_refund_to_buyer");
        assert_eq!(disposition.digest, None);
        assert_eq!(disposition.claim_ceiling, "no_payment_or_entitlement_claim");
        assert_eq!(
            fixture.receipt.body.mock_settlement.disposition,
            MockDisposition::RefundToBuyer
        );
        Ok(())
    }

    #[test]
    fn seller_failure_trace_binds_report_and_excludes_completion_records()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture: LocalWorkloadTraceFixtureV1 = local_workload_trace_fixture()?;
        let failure = fail_local_market_quote_preview(
            fixture.quote_preview.digest,
            fixture.report.deal_digest,
        )?;
        let trace = compose_failure_workload_trace(
            &fixture,
            &failure,
            &WorkloadDataLinkV1 {
                intent_reference: None,
                schema_version: "not_selected".to_owned(),
                status: "not_requested",
                link_basis: "no_local_data_root_selected",
                claim_ceiling: "no_local_data_source_was_requested_or_read",
            },
        )?;
        verify_workload_trace_v1(&trace)?;

        let failure_reference = trace
            .references
            .iter()
            .find(|reference| {
                reference.schema_version == "hyperai.compute-market.fulfillment-failure.v1"
            })
            .ok_or("failure trace omitted its seller report")?;
        assert_eq!(failure_reference.record_id, failure.attempt_id);
        assert_eq!(failure_reference.status, "fulfillment_failed");
        assert_eq!(
            failure_reference.digest,
            Some(fulfillment_failure_digest_v1(&failure)?)
        );
        assert_eq!(
            failure_reference.claim_ceiling,
            "seller_attributed_failure_report_only_not_independent_execution_or_payment_evidence"
        );
        assert!(
            trace
                .references
                .iter()
                .any(|reference| { reference.record_id == "synthetic-approved-deal-v1" })
        );
        for completion_only_record in [
            "synthetic-fulfillment-v1",
            "synthetic-mock-verifier-decision-v1",
            "synthetic-mock-disposition-v1",
            "synthetic-local-market-receipt-v1",
        ] {
            assert!(
                !trace
                    .references
                    .iter()
                    .any(|reference| { reference.record_id == completion_only_record })
            );
        }
        Ok(())
    }

    #[test]
    fn public_seller_failure_preview_returns_verified_synthetic_projection()
    -> Result<(), Box<dyn std::error::Error>> {
        let trace = workload_trace_seller_failure_preview_v1(None)?;
        verify_workload_trace_v1(&trace)?;

        assert_eq!(trace.state_slice, WORKLOAD_TRACE_FAILURE_SLICE);
        assert_eq!(trace.claim_ceiling, WORKLOAD_TRACE_FAILURE_CLAIM);
        assert!(trace.references.iter().any(|reference| {
            reference.schema_version == "hyperai.compute-market.fulfillment-failure.v1"
                && reference.status == "fulfillment_failed"
        }));
        assert!(
            trace
                .references
                .iter()
                .any(|reference| { reference.record_id == "synthetic-approved-deal-v1" })
        );
        Ok(())
    }

    #[test]
    fn failure_trace_rejects_missing_report_and_wrong_claim_pairing()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture: LocalWorkloadTraceFixtureV1 = local_workload_trace_fixture()?;
        let failure = fail_local_market_quote_preview(
            fixture.quote_preview.digest,
            fixture.report.deal_digest,
        )?;
        let mut trace = compose_failure_workload_trace(
            &fixture,
            &failure,
            &WorkloadDataLinkV1 {
                intent_reference: None,
                schema_version: "not_selected".to_owned(),
                status: "not_requested",
                link_basis: "no_local_data_root_selected",
                claim_ceiling: "no_local_data_source_was_requested_or_read",
            },
        )?;

        let mut wrong_claim_pairing = trace.clone();
        wrong_claim_pairing.claim_ceiling =
            "digest-bound local synthetic workload-to-receipt projection";
        reseal_trace(&mut wrong_claim_pairing)?;
        assert!(verify_workload_trace_v1(&wrong_claim_pairing).is_err());

        trace.references.retain(|reference| {
            reference.schema_version != "hyperai.compute-market.fulfillment-failure.v1"
        });
        sort_workload_references(&mut trace.references);
        reseal_trace(&mut trace)?;
        assert!(verify_workload_trace_v1(&trace).is_err());
        Ok(())
    }
}

#[cfg(test)]
mod connection_inventory_contract_tests {
    use super::{ConnectionModeV1, interface_connection_inventory_v1, interface_status};

    #[test]
    fn inventory_is_static_ordered_and_preserves_compatibility_projection() {
        let inventory = interface_connection_inventory_v1();
        assert_eq!(
            inventory.schema_version,
            "hyperai.interface.connection-inventory.v1"
        );
        assert_eq!(inventory.mode, "static_local_inventory");
        assert_eq!(inventory.connections.len(), 11);
        assert_eq!(
            inventory
                .connections
                .iter()
                .map(|item| item.module_id)
                .collect::<Vec<_>>(),
            [
                "compute_market",
                "simple_compute_market",
                "compute_bazaar",
                "hyperintel",
                "fleet",
                "eval",
                "outcomes",
                "justai",
                "esn_justnet",
                "trade",
                "data",
            ]
        );
        assert!(inventory.connections.iter().all(|item| {
            !item.owner.is_empty()
                && !item.state.is_empty()
                && !item.boundary.is_empty()
                && !item.next_blocked_gate.id.is_empty()
                && !item.next_blocked_gate.summary.is_empty()
        }));

        let bazaar = &inventory.connections[2];
        assert_eq!(bazaar.connection_mode, ConnectionModeV1::ReferenceOnly);
        assert!(bazaar.exposed_capability_ids.is_empty());
        assert_eq!(
            bazaar.source_revision,
            Some("d1809e5cfa843c087ebc4de9f00561bf0be67d55")
        );
        assert_eq!(
            bazaar.next_blocked_gate.id,
            "written_rights_for_exact_materials"
        );
        assert_eq!(
            inventory.connections[1].source_revision,
            Some("a1c46d20b1b2f1aabe53e1b052921a87cf9d7360")
        );
        assert_eq!(
            inventory.connections[3].source_revision,
            Some("d52111a07baa5fc4a7f6bfb4407731e2b9f609dc")
        );
        assert_eq!(
            inventory.connections[7].source_revision,
            Some("9a0e56536b968cbb2cb7173e142068b53c4f9fe3")
        );

        let status = interface_status();
        assert_eq!(status.schema_version, "hyperai.interface.status.v1");
        assert_eq!(status.mode, "local_synthetic_interface");
        assert_eq!(status.connections.len(), inventory.connections.len());
        for (legacy, current) in status.connections.iter().zip(inventory.connections.iter()) {
            assert_eq!(legacy.module_id, current.module_id);
            assert_eq!(legacy.name, current.name);
            assert_eq!(legacy.owner, current.owner);
            assert_eq!(legacy.state, current.state);
            assert_eq!(legacy.boundary, current.boundary);
        }
        assert_eq!(interface_connection_inventory_v1(), inventory);
    }
}
