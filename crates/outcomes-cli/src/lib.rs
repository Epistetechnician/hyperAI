//! Shared `HyperAI` command implementation for the `hyperai` and `hsai` binaries.

mod beta_inventory;
mod beta_provenance;

use std::{
    collections::{BTreeMap, HashMap},
    fs,
    io::{IsTerminal, Read},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use hyperai_acp::AgentProfileV1;
use hyperliquid_hip4::{Hip4ReadOnlyBackend, decode_asset_id, encode_asset_ids};
use outcomes_api::{
    LocalAuthorizationDenialReportV1, run_local_authorization_denial_acceptance,
    verify_local_authorization_denial_acceptance,
};
use outcomes_application::{
    LocalRecoveryReportV1, LocalScenario, ScenarioReport, run_local_lost_receipt_recovery,
    run_local_scenario, verify_local_lost_receipt_recovery,
};
use outcomes_blobstore::FileBlobStore;
use outcomes_corpus::{
    LocalMaterializationLimits, LocalMaterializationRequest, bundled_corpus_profiles,
    materialize_pinned_local_root,
};
use outcomes_domain::{
    ClaimSpecDraftV1, ClaimSpecV1, ClaimTruth, ExecutionOutcome, MarketBindingV1,
    OutcomeEvidenceBundleV1, ReconciliationResultV1, ReconciliationStatus, ResolutionFinality,
    ResolutionRecordV1, SettlementNetwork, UnsignedSettlementIntentV1, UtcTimestamp,
    domain_separated_digest,
};
use outcomes_evidence::verify_evidence_bundle;
use outcomes_settlement::{
    CapabilityMatrix, SettlementEligibilityBlocker, build_intent, preflight,
};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use beta_inventory::{finalize_run_inventory, verify_run_inventory};
use beta_provenance::{
    BetaProvenanceProvider, BetaRunProvenanceV1, CommandBetaProvenanceProvider,
    validate_recorded_provenance,
};

const CAPABILITIES: &str = include_str!("../../../config/adapter-capabilities.json");
const CORPUS_PROFILES: &str = include_str!("../../../config/corpus-profiles.json");
const LEGACY_LOCAL_BETA_PROFILE: &str = include_str!("../../../config/local-beta-profile.json");
const LOCAL_BETA_PROFILE_V2: &str = include_str!("../../../config/local-beta-profile-v2.json");
const LOCAL_BETA_PROFILE: &str = include_str!("../../../config/local-beta-profile-v3.json");
const BETA_SCENARIOS: [(&str, LocalScenario); 7] = [
    ("true", LocalScenario::True),
    ("false", LocalScenario::False),
    ("invalid-then-valid", LocalScenario::InvalidThenValid),
    (
        "disputed-false-counterexample-true",
        LocalScenario::DisputedFalseCounterexampleTrue,
    ),
    (
        "incomplete-observation",
        LocalScenario::IncompleteObservation,
    ),
    ("divergent-replays", LocalScenario::DivergentReplays),
    ("runner-error", LocalScenario::RunnerError),
];

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalBetaProfile {
    schema_version: String,
    profile_id: String,
    network: String,
    backend: String,
    live_writes: bool,
    serial_concurrency: u8,
    scenarios: Vec<String>,
    artifact_classification: String,
    claim_ceiling: String,
    stop_on_failure: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalBetaProfileV2 {
    schema_version: String,
    profile_id: String,
    network: String,
    backend: String,
    live_writes: bool,
    serial_concurrency: u8,
    scenarios: Vec<String>,
    operational_checks: Vec<String>,
    artifact_classification: String,
    claim_ceiling: String,
    stop_on_failure: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BetaScenarioResult {
    scenario: String,
    status: String,
    truth: ClaimTruth,
    finality: ResolutionFinality,
    execution_outcomes: Vec<ExecutionOutcome>,
    mock_effect_count: usize,
    reconciliation_status: Option<ReconciliationStatus>,
    evidence_bundles: Vec<BetaBundleResult>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BetaBundleResult {
    relative_path: String,
    bundle_digest: outcomes_domain::Sha256Digest,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BetaTamperCheck {
    status: String,
    expected_result: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BetaRunManifestV2 {
    schema_version: String,
    profile_schema_version: String,
    profile_id: String,
    profile_file: String,
    profile_digest: outcomes_domain::Sha256Digest,
    provenance_file: String,
    provenance_digest: outcomes_domain::Sha256Digest,
    run_id: String,
    status: String,
    claim_ceiling: String,
    artifact_classification: String,
    scenario_results: Vec<BetaScenarioResult>,
    tamper_check: BetaTamperCheck,
    external_effect: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BetaOperationalCheckResult {
    check: String,
    status: String,
    report_file: String,
    report_digest: outcomes_domain::Sha256Digest,
    database_file: String,
    database_digest: outcomes_domain::Sha256Digest,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BetaRunManifestV3 {
    schema_version: String,
    profile_schema_version: String,
    profile_id: String,
    profile_file: String,
    profile_digest: outcomes_domain::Sha256Digest,
    provenance_file: String,
    provenance_digest: outcomes_domain::Sha256Digest,
    run_id: String,
    status: String,
    claim_ceiling: String,
    artifact_classification: String,
    scenario_results: Vec<BetaScenarioResult>,
    operational_checks: Vec<BetaOperationalCheckResult>,
    tamper_check: BetaTamperCheck,
    external_effect: String,
}

type BetaRunManifestV4 = BetaRunManifestV3;

#[derive(Debug, Parser)]
#[command(
    name = "hyperai",
    version,
    about = "HyperAI modular compute and outcome protocols"
)]
struct Cli {
    #[command(subcommand)]
    command: RootCommand,
}

#[derive(Debug, Subcommand)]
enum RootCommand {
    /// Initialize a per-user local `HyperAI` home.
    Init(HomeArgs),
    /// Inspect local configuration and the loopback service without changing state.
    Doctor(HomeArgs),
    /// Start, inspect, or stop the local service.
    Service(ServiceArgs),
    /// Probe and report the configured local service connection.
    Connect(HomeArgs),
    /// Read the versioned local product connection inventory as JSON.
    Connections(ConnectionsArgs),
    /// Open the keyboard-driven local workspace.
    Desk(HomeArgs),
    /// Internal foreground service process entrypoint.
    #[command(name = "__serve", hide = true)]
    Serve(ServeArgs),
    /// Local-only compute market commands.
    Market(MarketArgs),
    /// Back up or restore this installation's local metadata.
    Storage(StorageArgs),
    /// Local, source-labelled dataset catalog and read-only query commands.
    Data(DataArgs),
    /// Local synthetic Fleet simulator commands over the authenticated service.
    Fleet(FleetArgs),
    /// Fixed synthetic evaluation harness catalog and fixture comparison.
    Eval(EvalArgs),
    /// Local research over the versioned synthetic compute-price series.
    Trade(TradeArgs),
    /// Manage one explicit local ACP agent session through the shared service.
    Agent(AgentArgs),
    /// Verifiable outcome protocol commands.
    Outcomes(OutcomesArgs),
}

#[derive(Debug, Args)]
struct HomeArgs {
    /// Per-user `HyperAI` home. Defaults to the platform data directory.
    #[arg(long)]
    home: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct ConnectionsArgs {
    /// Per-user `HyperAI` home. Defaults to the platform data directory.
    #[arg(long)]
    home: Option<PathBuf>,
    /// Emit compact versioned JSON without presentation whitespace.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct StorageArgs {
    #[command(subcommand)]
    command: StorageCommand,
}

#[derive(Debug, Subcommand)]
enum StorageCommand {
    /// Create a consistent, no-overwrite metadata backup.
    Backup(HomeArgs),
    /// Restore metadata from a generated backup in this home.
    Restore {
        #[arg(long)]
        home: Option<PathBuf>,
        #[arg(long)]
        backup_name: String,
    },
}

#[derive(Debug, Args)]
struct ServiceArgs {
    #[command(subcommand)]
    command: ServiceCommand,
}

#[derive(Debug, Subcommand)]
enum ServiceCommand {
    /// Start the authenticated IPv4 loopback service in the background.
    Start {
        #[arg(long)]
        home: Option<PathBuf>,
        #[arg(long, default_value_t = 4317, value_parser = clap::value_parser!(u16))]
        port: u16,
    },
    /// Check the live service instance recorded in the local home.
    Status(HomeArgs),
    /// Request a graceful local service shutdown.
    Stop(HomeArgs),
}

#[derive(Debug, Args)]
struct ServeArgs {
    #[arg(long)]
    home: Option<PathBuf>,
    #[arg(long, default_value_t = 4317, value_parser = clap::value_parser!(u16))]
    port: u16,
}

#[derive(Debug, Args)]
struct MarketArgs {
    #[command(subcommand)]
    command: MarketCommand,
}

#[derive(Debug, Subcommand)]
enum MarketCommand {
    /// Compare the fixed local buyer request with signed offers and advisory observations.
    Compare(MarketCompareArgs),
    /// Read the owner-only durable local synthetic deal history.
    History(MarketHistoryArgs),
    /// Preview the seller quote bound to one displayed supply comparison digest.
    QuotePreview(MarketSupplyQuotePreviewArgs),
    /// Create the synthetic buyer approval for the exact reviewed quote-preview digest.
    QuoteApprove(MarketQuoteApproveArgs),
    /// Create deterministic mock fulfillment for the exact approved quote and deal digests.
    QuoteComplete(MarketQuoteCompleteArgs),
    /// Show the local digest-bound workload-to-receipt projection.
    Trace(MarketTraceArgs),
    /// Run the deterministic synthetic catalog-to-mock-settlement path.
    Demo(MarketDemoArgs),
    /// Verify the structural integrity and signatures in a saved local receipt.
    Verify(MarketVerifyArgs),
    /// Recover a missing receipt export from a saved local mock journal.
    Recover(MarketRecoverArgs),
}

#[derive(Debug, Args)]
struct MarketCompareArgs {
    #[arg(long)]
    home: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct MarketHistoryArgs {
    /// Requested page size, bounded to 1–50 records.
    #[arg(long, default_value_t = 20)]
    limit: usize,
    /// Opaque continuation cursor returned by a previous history page.
    #[arg(long)]
    cursor: Option<String>,
    #[arg(long)]
    home: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct MarketSupplyQuotePreviewArgs {
    /// Exact digest returned by `hyperai market compare`.
    #[arg(long)]
    comparison_digest: String,
    #[arg(long)]
    home: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct MarketQuoteApproveArgs {
    /// Exact nested quote-preview digest returned by `hyperai market quote-preview`.
    #[arg(long)]
    preview_digest: String,
    #[arg(long)]
    home: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct MarketQuoteCompleteArgs {
    /// Exact nested quote-preview digest returned by `hyperai market quote-preview`.
    #[arg(long)]
    preview_digest: String,
    /// Exact deal digest returned by `hyperai market quote-approve`.
    #[arg(long)]
    deal_digest: String,
    #[arg(long)]
    home: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct MarketTraceArgs {
    /// Optional local `HyperAI` data root to include its verified synthetic input reference.
    #[arg(long)]
    data_root: Option<PathBuf>,
    /// Show the fixed synthetic seller-failure report instead of the success trace.
    #[arg(long, conflicts_with = "inconclusive_preview")]
    seller_failure_preview: bool,
    /// Show the fixed synthetic inconclusive-verdict and mock dispute-hold trace.
    #[arg(long, conflicts_with = "seller_failure_preview")]
    inconclusive_preview: bool,
}

#[derive(Debug, Args)]
struct DataArgs {
    #[arg(long)]
    home: Option<PathBuf>,
    #[command(subcommand)]
    command: DataCommand,
}

#[derive(Debug, Args)]
struct FleetArgs {
    #[arg(long)]
    home: Option<PathBuf>,
    #[command(subcommand)]
    command: FleetCommand,
}

#[derive(Debug, Subcommand)]
enum FleetCommand {
    /// Read the fixed synthetic Fleet inventory without probing a host.
    Inventory,
    /// Read fixture checks and explicitly unknown live Fleet checks.
    Doctor,
    /// Calculate a digest-bound advisory plan without invoking an operation.
    Preview {
        /// Fixed synthetic node ID from `fleet inventory`.
        #[arg(long)]
        node_id: String,
        /// Candidate intent to include in the non-executable preview.
        #[arg(long, value_enum)]
        operation: FleetPreviewOperationArg,
        /// Prior preview digest to detect a changed plan.
        #[arg(long)]
        expected_preview_digest: Option<String>,
    },
    /// Inspect or transition the local volatile virtual Fleet simulator.
    Simulation {
        #[command(subcommand)]
        command: FleetSimulationCommand,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FleetPreviewOperationArg {
    Attach,
    Launch,
    WorkloadStart,
    Cancel,
    Terminate,
}

impl FleetPreviewOperationArg {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Attach => "attach",
            Self::Launch => "launch",
            Self::WorkloadStart => "workload_start",
            Self::Cancel => "cancel",
            Self::Terminate => "terminate",
        }
    }
}

#[derive(Debug, Args)]
struct EvalArgs {
    #[arg(long)]
    home: Option<PathBuf>,
    #[command(subcommand)]
    command: EvalCommand,
}

#[derive(Debug, Subcommand)]
enum EvalCommand {
    /// Read the local synthetic task catalog and suite digest.
    Catalog,
    /// Run the fixed paired-answer harness qualification through the service.
    Run {
        #[arg(long)]
        expected_suite_digest: String,
        #[arg(long, default_value = "synthetic-pair-v1")]
        pair_id: String,
        #[arg(long)]
        idempotency_key: String,
    },
}

#[derive(Debug, Args)]
struct TradeArgs {
    #[arg(long)]
    home: Option<PathBuf>,
    #[command(subcommand)]
    command: TradeCommand,
}

#[derive(Debug, Subcommand)]
enum TradeCommand {
    /// Show fixed research instruments, source release, and method versions.
    Instruments,
    /// Show one gap-aware research series.
    Series {
        #[arg(long)]
        instrument_id: String,
    },
    /// Apply bounded synthetic price and availability assumptions.
    Scenario {
        #[arg(long)]
        primary_instrument_id: String,
        #[arg(long)]
        comparison_instrument_id: String,
        #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(i32).range(-5000..=5000))]
        price_shock_bps: i32,
        #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(i64).range(-10..=10))]
        availability_delta_units: i64,
    },
    /// Replay hourly asking-cost selection against a named baseline.
    Paper {
        #[arg(long, default_value = "a100-us-west")]
        baseline_instrument_id: String,
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u64).range(1..=8))]
        requested_units: u64,
    },
}

#[derive(Debug, Args)]
struct AgentArgs {
    /// Per-user `HyperAI` home. Defaults to the platform data directory.
    #[arg(long, global = true)]
    home: Option<PathBuf>,
    #[command(subcommand)]
    command: AgentCommand,
}

#[derive(Debug, Subcommand)]
enum AgentCommand {
    /// Validate and display a profile without starting its process.
    Profile(AgentProfileArgs),
    /// Explicitly start the selected local ACP v1 process.
    Connect(AgentProfileArgs),
    /// Read the current sanitized session snapshot.
    Status,
    /// Send one bounded prompt read from stdin; prompt text is not a CLI argument.
    Prompt,
    /// Cancel the active ACP request.
    Cancel,
    /// Disconnect and terminate the owned local process group.
    Disconnect,
}

#[derive(Debug, Args)]
struct AgentProfileArgs {
    /// Human-readable profile label.
    #[arg(long, default_value = "Local ACP agent")]
    name: String,
    /// Absolute path to the selected agent executable.
    #[arg(long)]
    executable: PathBuf,
    /// Absolute working directory passed to ACP session/new.
    #[arg(long)]
    working_directory: PathBuf,
    /// One executable argument. Repeat this option to add separate arguments.
    #[arg(long = "arg", action = clap::ArgAction::Append)]
    args: Vec<String>,
}

impl AgentProfileArgs {
    fn into_profile(self) -> AgentProfileV1 {
        AgentProfileV1 {
            name: self.name,
            executable: self.executable,
            args: self.args,
            working_directory: self.working_directory,
            environment: BTreeMap::default(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum FleetSimulationCommand {
    /// Read the current synthetic simulator snapshot.
    Show,
    /// Apply one explicit digest-bound simulator transition.
    Step {
        #[arg(long, value_enum)]
        operation: FleetSimulationOperationArg,
        #[arg(long)]
        expected_state_digest: String,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=4))]
        gpu_units: Option<u8>,
        #[arg(long, value_enum, default_value_t = FleetSimulationFaultArg::None)]
        fault: FleetSimulationFaultArg,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FleetSimulationOperationArg {
    Attach,
    Launch,
    WorkloadStart,
    Cancel,
    Terminate,
    Reconcile,
    Cleanup,
}

impl FleetSimulationOperationArg {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Attach => "attach",
            Self::Launch => "launch",
            Self::WorkloadStart => "workload_start",
            Self::Cancel => "cancel",
            Self::Terminate => "terminate",
            Self::Reconcile => "reconcile",
            Self::Cleanup => "cleanup",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum FleetSimulationFaultArg {
    #[default]
    None,
    TimeoutBeforeEffect,
    TimeoutAfterEffect,
    PartialEffect,
}

impl FleetSimulationFaultArg {
    const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::TimeoutBeforeEffect => "timeout_before_effect",
            Self::TimeoutAfterEffect => "timeout_after_effect",
            Self::PartialEffect => "partial_effect",
        }
    }
}

#[derive(Debug, Subcommand)]
enum DataCommand {
    /// Preview-bind and create the deterministic local JSON and Parquet sample.
    SeedSynthetic,
    /// List verified datasets beneath the local `HyperAI` home.
    Tables,
    /// Show one verified local dataset manifest.
    Describe { dataset_id: String },
    /// Run one bounded SELECT over a verified local dataset.
    Sql {
        dataset_id: String,
        #[arg(long)]
        query: String,
        #[arg(long, value_enum, default_value_t = DataOutputArg::Json)]
        format: DataOutputArg,
        #[arg(long)]
        max_rows: Option<usize>,
    },
    /// Preview or explicitly save a local query model from a JSON definition.
    Model {
        #[command(subcommand)]
        command: DataModelCommand,
    },
    /// Preview or explicitly save a local view from a JSON definition.
    View {
        #[command(subcommand)]
        command: DataViewCommand,
    },
}

#[derive(Debug, Subcommand)]
enum DataModelCommand {
    /// List saved models and their source-snapshot state.
    List,
    /// Run a saved model only while its input snapshot is current.
    Run { model_id: String },
    /// Ask the local service to validate without writing.
    Preview { definition: PathBuf },
    /// Re-preview and save with a fresh idempotency key.
    Save { definition: PathBuf },
}

#[derive(Debug, Subcommand)]
enum DataViewCommand {
    /// List saved views and their model-link state.
    List,
    /// Ask the local service to validate without writing.
    Preview { definition: PathBuf },
    /// Re-preview and save with a fresh idempotency key.
    Save { definition: PathBuf },
}

#[derive(Debug, Subcommand)]
enum DataDefinitionCommand {
    /// Ask the local service to validate a definition without writing it.
    Preview { definition: PathBuf },
    /// Re-preview and save a definition with a fresh idempotency key.
    Save { definition: PathBuf },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DataOutputArg {
    Json,
    Csv,
}

#[derive(Debug, Args)]
struct MarketDemoArgs {
    /// New directory for the receipt and recovery journal.
    #[arg(long)]
    output_dir: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct MarketVerifyArgs {
    /// Receipt JSON file to check without network access.
    #[arg(long)]
    receipt: PathBuf,
}

#[derive(Debug, Args)]
struct MarketRecoverArgs {
    /// Local one-deal journal JSON file.
    #[arg(long)]
    journal: PathBuf,
    /// New destination file for the recovered receipt.
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Args)]
struct OutcomesArgs {
    #[command(subcommand)]
    command: OutcomesCommand,
}

#[derive(Debug, Subcommand)]
enum OutcomesCommand {
    /// Adapter capability inspection.
    Adapter(AdapterArgs),
    /// Offline claim authoring and validation.
    Claim(ClaimArgs),
    /// Standalone evidence verification.
    Evidence(EvidenceArgs),
    /// Hermetic local demonstrations.
    Demo(DemoArgs),
    /// Safe HIP-4 read-only metadata and checked asset IDs.
    Hip4(Hip4Args),
    /// Safe artifact-driven settlement preparation and inspection.
    Settlement(SettlementArgs),
    /// Pinned corpus profile inspection and local materialization.
    Corpus(CorpusArgs),
    /// Controlled single-machine, mock-only beta operations.
    Beta(BetaArgs),
}

#[derive(Debug, Args)]
struct BetaArgs {
    #[command(subcommand)]
    command: BetaCommand,
}

#[derive(Debug, Subcommand)]
enum BetaCommand {
    /// Run the complete controlled local beta rehearsal without network writes.
    Rehearse {
        #[arg(long)]
        artifact_root: PathBuf,
        #[arg(long)]
        run_id: String,
    },
    /// Semantically verify and seal a completed beta run with an exact inventory.
    Finalize {
        #[arg(long)]
        artifact_root: PathBuf,
        #[arg(long)]
        run_id: String,
    },
    /// Read-only semantic and exact-inventory verification of a sealed beta run.
    Verify {
        #[arg(long)]
        artifact_root: PathBuf,
        #[arg(long)]
        run_id: String,
    },
}

#[derive(Debug, Args)]
struct AdapterArgs {
    #[command(subcommand)]
    command: AdapterCommand,
}

#[derive(Debug, Subcommand)]
enum AdapterCommand {
    /// Print the dated machine-readable capability matrix.
    Capabilities {
        /// Filter to one adapter ID.
        #[arg(long)]
        adapter: Option<String>,
    },
}

#[derive(Debug, Args)]
struct ClaimArgs {
    #[command(subcommand)]
    command: ClaimCommand,
}

#[derive(Debug, Subcommand)]
enum ClaimCommand {
    /// Validate a draft JSON claim without freezing it.
    Validate { claim: PathBuf },
    /// Freeze a draft JSON claim and print the derived immutable object.
    Freeze { claim: PathBuf },
    /// Verify a frozen JSON claim's derived identity.
    Show { claim: PathBuf },
}

#[derive(Debug, Args)]
struct EvidenceArgs {
    #[command(subcommand)]
    command: EvidenceCommand,
}

#[derive(Debug, Subcommand)]
enum EvidenceCommand {
    /// Verify a public bundle and local CAS blobs without a database.
    Verify {
        bundle: PathBuf,
        #[arg(long)]
        blob_root: PathBuf,
    },
}

#[derive(Debug, Args)]
struct DemoArgs {
    #[command(subcommand)]
    command: DemoCommand,
}

#[derive(Debug, Subcommand)]
enum DemoCommand {
    /// Run the local computational-market scenario.
    ComputationalMarket {
        #[arg(long, value_enum, default_value_t = ScenarioArg::True)]
        scenario: ScenarioArg,
        #[arg(long, default_value = "artifacts/demo")]
        artifact_root: PathBuf,
        /// Print JSON. The current CLI always emits machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ScenarioArg {
    True,
    InvalidThenValid,
    DisputedFalseCounterexampleTrue,
    False,
    IncompleteObservation,
    DivergentReplays,
    RunnerError,
}

#[derive(Debug, Args)]
struct Hip4Args {
    #[command(subcommand)]
    command: Hip4Command,
}

#[derive(Debug, Subcommand)]
enum Hip4Command {
    /// Parse a retained outcomeMeta fixture without network access.
    Discover {
        /// Network scope for the outcome ID.
        #[arg(long, value_enum)]
        network: Hip4NetworkArg,
        /// Sanitized retained outcomeMeta JSON.
        #[arg(long)]
        fixture: PathBuf,
    },
    /// Compute all checked HIP-4 identifiers.
    Encode { outcome_id: u64, side_index: u8 },
    /// Decode a `#coin` or `+token` identifier.
    Decode { asset_id: String },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Hip4NetworkArg {
    Mock,
    Testnet,
    MainnetReadonly,
}

#[derive(Debug, Args)]
struct SettlementArgs {
    #[command(subcommand)]
    command: SettlementCommand,
}

#[derive(Debug, Subcommand)]
enum SettlementCommand {
    /// Re-evaluate settlement readiness against retained current metadata.
    Preflight(SettlementInputArgs),
    /// Build a deterministic unsigned intent after successful preflight.
    BuildIntent {
        #[command(flatten)]
        inputs: SettlementInputArgs,
        /// Canonical whole-second UTC intent expiry.
        #[arg(long)]
        expires_at: String,
    },
    /// Inspect an intent through a no-effect dry run.
    Publish {
        intent: PathBuf,
        /// Required. This build has no mutation-capable publisher.
        #[arg(long)]
        dry_run: bool,
    },
    /// Inspect a retained reconciliation result.
    Reconcile { reconciliation: PathBuf },
}

#[derive(Debug, Args)]
struct SettlementInputArgs {
    /// Explicit digest-bound market binding JSON.
    #[arg(long)]
    binding: PathBuf,
    /// Final HSAI resolution JSON.
    #[arg(long)]
    resolution: PathBuf,
    /// Retained current outcomeMeta JSON.
    #[arg(long)]
    metadata: PathBuf,
    /// Dated backend capability snapshot JSON.
    #[arg(long)]
    capabilities: PathBuf,
}

#[derive(Debug, Args)]
struct CorpusArgs {
    #[command(subcommand)]
    command: CorpusCommand,
}

#[derive(Debug, Subcommand)]
enum CorpusCommand {
    /// Print the checked-in immutable corpus profile catalog.
    Profiles,
    /// Materialize safe files and inspect special metadata from a pinned checkout.
    MaterializeLocal {
        #[arg(long)]
        profile: String,
        #[arg(long)]
        checkout_root: PathBuf,
        #[arg(long)]
        materialization_root: String,
        #[arg(long)]
        expected_commit: String,
        #[arg(long)]
        observed_commit: String,
        #[arg(long, default_value_t = 10_000)]
        max_file_count: usize,
        #[arg(long, default_value_t = 16_777_216)]
        max_file_bytes: u64,
        #[arg(long, default_value_t = 268_435_456)]
        max_total_bytes: u64,
    },
}

/// Parses and executes the shared `HyperAI` command line.
#[must_use]
pub fn run_cli() -> ExitCode {
    let cli = Cli::parse();
    let compact_json = matches!(&cli.command, RootCommand::Connections(args) if args.json);
    match run(cli) {
        Ok(value) => match if compact_json {
            serde_json::to_string(&value)
        } else {
            serde_json::to_string_pretty(&value)
        } {
            Ok(output) => {
                println!("{output}");
                ExitCode::SUCCESS
            }
            Err(error) => fail(5, "OUTPUT_ENCODING_FAILED", &error.to_string()),
        },
        Err(error) => fail(error.exit_code, &error.code, &error.message),
    }
}

/// Run the private fixed-fixture scorer mode when invoked by the local service.
///
/// Returns `None` for ordinary CLI entry. The scorer accepts only its bounded
/// stdin protocol and never runs an agent or model.
#[must_use]
pub fn run_eval_scorer_process() -> Option<ExitCode> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("__internal_eval_scorer"))
        || args.next().is_some()
    {
        return None;
    }
    Some(if let Ok(()) = hyperai_eval::run_scorer_stdio_v1() {
        ExitCode::SUCCESS
    } else {
        eprintln!("The fixed synthetic evaluation scorer rejected its protocol input.");
        ExitCode::FAILURE
    })
}

fn run_market_trace(args: &MarketTraceArgs) -> Result<Value, CliError> {
    let trace = if args.inconclusive_preview {
        hyperai_application::workload_trace_inconclusive_preview_v1(args.data_root.as_deref())
    } else if args.seller_failure_preview {
        hyperai_application::workload_trace_seller_failure_preview_v1(args.data_root.as_deref())
    } else {
        hyperai_application::workload_trace_v1(args.data_root.as_deref())
    }
    .map_err(CliError::internal)?;
    serde_json::to_value(trace).map_err(CliError::internal)
}

fn run_market_compare(args: &MarketCompareArgs) -> Result<Value, CliError> {
    let (client, base_url, bearer) = local_service_client(args.home.as_deref())?;
    let response = client
        .get(format!("{base_url}/api/v1/market/supply-comparison"))
        .bearer_auth(bearer)
        .send()
        .map_err(CliError::internal)?;
    local_service_json_response(response)
}

fn run_market_history(args: &MarketHistoryArgs) -> Result<Value, CliError> {
    if !(1..=50).contains(&args.limit) {
        return Err(CliError::input(
            "MARKET_HISTORY_INVALID_QUERY",
            "history page limit must be between 1 and 50",
        ));
    }
    let (client, base_url, bearer) = local_service_client(args.home.as_deref())?;
    let mut url = reqwest::Url::parse(&format!("{base_url}/api/v1/market/deal-history"))
        .map_err(CliError::internal)?;
    url.query_pairs_mut()
        .append_pair("limit", &args.limit.to_string());
    if let Some(cursor) = args.cursor.as_deref() {
        url.query_pairs_mut().append_pair("cursor", cursor);
    }
    let payload = local_service_json_response(
        client
            .get(url)
            .bearer_auth(bearer)
            .send()
            .map_err(CliError::internal)?,
    )?;
    decode_market_history_page(payload)
}

fn decode_market_history_page(payload: Value) -> Result<Value, CliError> {
    let page = serde_json::from_value::<hyperai_market::LocalMarketHistoryPageV1>(payload)
        .map_err(CliError::internal)?;
    hyperai_market::verify_local_market_history_page_v1(&page).map_err(|_| {
        CliError::verification("local service returned an invalid Market history page")
    })?;
    serde_json::to_value(page).map_err(CliError::internal)
}

fn run_market_supply_quote_preview(args: &MarketSupplyQuotePreviewArgs) -> Result<Value, CliError> {
    let (client, base_url, bearer) = local_service_client(args.home.as_deref())?;
    let response = client
        .post(format!("{base_url}/api/v1/market/supply-quote-preview"))
        .bearer_auth(bearer)
        .json(&json!({ "comparison_digest": args.comparison_digest }))
        .send()
        .map_err(CliError::internal)?;
    local_service_json_response(response)
}

fn run_market_quote_approve(args: &MarketQuoteApproveArgs) -> Result<Value, CliError> {
    let (client, base_url, bearer) = local_service_client(args.home.as_deref())?;
    let response = client
        .post(format!("{base_url}/api/v1/market/quote-approve"))
        .bearer_auth(bearer)
        .json(&json!({ "preview_digest": args.preview_digest.as_str() }))
        .send()
        .map_err(CliError::internal)?;
    let approval = local_service_json_response(response)?;
    if approval["schema_version"] != "hyperai.compute-market.local-quote-approval.v1"
        || approval["preview_digest"] != args.preview_digest
        || approval["deal_digest"].as_str().is_none()
        || approval["approved_deal"].is_null()
        || !approval["effects"].as_str().is_some_and(|effects| {
            effects.contains("synthetic in-memory approval only")
                && effects.contains("no fulfillment")
                && effects.contains("funds effect")
        })
    {
        return Err(CliError::verification(
            "local service returned an approval that does not bind the reviewed synthetic quote",
        ));
    }
    Ok(approval)
}

fn run_market_quote_complete(args: &MarketQuoteCompleteArgs) -> Result<Value, CliError> {
    let (client, base_url, bearer) = local_service_client(args.home.as_deref())?;
    let response = client
        .post(format!("{base_url}/api/v1/market/quote-complete"))
        .bearer_auth(bearer)
        .json(&json!({
            "preview_digest": args.preview_digest.as_str(),
            "deal_digest": args.deal_digest.as_str(),
        }))
        .send()
        .map_err(CliError::internal)?;
    let completion = local_service_json_response(response)?;
    if completion["schema_version"] != "hyperai.interface.market-quote-completion.v1"
        || completion["preview_digest"] != args.preview_digest
        || completion["deal_digest"] != args.deal_digest
        || completion["demo"]["report"]["deal_digest"] != args.deal_digest
        || completion["demo"]["receipt"]["digest"].as_str().is_none()
        || completion["effect_boundary"]
            != "local_synthetic_mock_fulfillment_verifier_and_settlement_only"
        || completion["demo"]["receipt"]["body"]["effects"]
            .as_str()
            .is_none_or(|effects| {
                !effects.contains("no network") || !effects.contains("or funds moved")
            })
    {
        return Err(CliError::verification(
            "local service returned a completion that does not bind the reviewed quote and deal",
        ));
    }
    Ok(completion)
}

fn run(cli: Cli) -> Result<Value, CliError> {
    match cli.command {
        RootCommand::Init(args) => run_init(&args),
        RootCommand::Doctor(args) => {
            let home = hyperai_local_service::storage::resolve_home(args.home.as_deref())
                .map_err(CliError::internal)?;
            let home_report = hyperai_local_service::storage::inspect_home(Some(&home))
                .map_err(CliError::internal)?;
            let service = service_status(Some(&home))?;
            let initialized = home_report.initialized;
            let service_status = service["status"].clone();
            let overall_status = if initialized && service_status == "ready" {
                "ready"
            } else {
                "incomplete"
            };
            Ok(json!({
                "schema_version": "hyperai.local-doctor.v1",
                "status": overall_status,
                "home": home_report,
                "service": service,
                "checks": [
                    { "name": "local_home", "status": if initialized { "pass" } else { "unknown" } },
                    { "name": "loopback_service", "status": service_status },
                    { "name": "external_effects", "status": "closed" }
                ],
            }))
        }
        RootCommand::Service(args) => run_service(args),
        RootCommand::Connect(args) => service_status(args.home.as_deref()),
        RootCommand::Connections(args) => run_connections(&args),
        RootCommand::Desk(args) => run_desk(&args),
        RootCommand::Serve(args) => {
            let runtime = tokio::runtime::Runtime::new().map_err(CliError::internal)?;
            runtime
                .block_on(hyperai_local_service::server::serve(
                    args.home.as_deref(),
                    args.port,
                ))
                .map_err(CliError::internal)?;
            Ok(json!({
                "schema_version": "hyperai.local-service.exit.v1",
                "status": "stopped",
            }))
        }
        RootCommand::Market(market) => match market.command {
            MarketCommand::Compare(args) => run_market_compare(&args),
            MarketCommand::History(args) => run_market_history(&args),
            MarketCommand::QuotePreview(args) => run_market_supply_quote_preview(&args),
            MarketCommand::QuoteApprove(args) => run_market_quote_approve(&args),
            MarketCommand::QuoteComplete(args) => run_market_quote_complete(&args),
            MarketCommand::Trace(args) => run_market_trace(&args),
            MarketCommand::Demo(args) => {
                let receipt = hyperai_market::local_market_receipt().map_err(CliError::internal)?;
                let wrote_artifacts = if let Some(output_dir) = args.output_dir {
                    hyperai_market::write_local_market_run(&output_dir, &receipt)
                        .map_err(CliError::internal)?;
                    true
                } else {
                    false
                };
                let report = hyperai_market::local_demo_report(&receipt, wrote_artifacts);
                serde_json::to_value(report).map_err(CliError::internal)
            }
            MarketCommand::Verify(args) => {
                let receipt = hyperai_market::read_local_market_receipt(&args.receipt)
                    .map_err(CliError::internal)?;
                Ok(json!({
                    "schema_version": "hyperai.compute-market.offline-verification.v1",
                    "status": "OFFLINE_CONTRACT_CHECK_PASS",
                    "receipt_digest": receipt.digest,
                    "claim_ceiling": receipt.body.claim_ceiling,
                    "nonclaims": receipt.body.nonclaims,
                }))
            }
            MarketCommand::Recover(args) => {
                let receipt =
                    hyperai_market::recover_local_market_receipt(&args.journal, &args.output)
                        .map_err(CliError::internal)?;
                Ok(json!({
                    "schema_version": "hyperai.compute-market.local-recovery.v1",
                    "status": "RECOVERED_FROM_LOCAL_MOCK_JOURNAL",
                    "receipt_digest": receipt.digest,
                    "claim_ceiling": receipt.body.claim_ceiling,
                    "nonclaims": receipt.body.nonclaims,
                }))
            }
        },
        RootCommand::Data(data) => run_data(data),
        RootCommand::Storage(storage) => run_storage(storage),
        RootCommand::Fleet(fleet) => run_fleet(fleet),
        RootCommand::Eval(eval) => run_eval(eval),
        RootCommand::Trade(trade) => run_trade(trade),
        RootCommand::Agent(agent) => run_agent(agent),
        RootCommand::Outcomes(outcomes) => run_outcomes(outcomes),
    }
}

fn run_init(args: &HomeArgs) -> Result<Value, CliError> {
    let runtime = tokio::runtime::Runtime::new().map_err(CliError::internal)?;
    let (_, report) = runtime
        .block_on(hyperai_local_service::storage::initialize_home(
            args.home.as_deref(),
        ))
        .map_err(CliError::internal)?;
    serde_json::to_value(report).map_err(CliError::internal)
}

fn run_storage(args: StorageArgs) -> Result<Value, CliError> {
    let runtime = tokio::runtime::Runtime::new().map_err(CliError::internal)?;
    match args.command {
        StorageCommand::Backup(args) => serde_json::to_value(
            runtime
                .block_on(hyperai_local_service::storage::backup_metadata(
                    args.home.as_deref(),
                ))
                .map_err(storage_cli_error)?,
        )
        .map_err(CliError::internal),
        StorageCommand::Restore { home, backup_name } => serde_json::to_value(
            runtime
                .block_on(hyperai_local_service::storage::restore_metadata(
                    home.as_deref(),
                    &backup_name,
                ))
                .map_err(storage_cli_error)?,
        )
        .map_err(CliError::internal),
    }
}

fn storage_cli_error(error: hyperai_local_service::storage::StorageError) -> CliError {
    use hyperai_local_service::storage::StorageError;

    let (exit_code, code, message) = match error {
        StorageError::HomeNotInitialized => (
            2,
            "HOME_NOT_INITIALIZED",
            "initialize the selected HyperAI home before using storage backups",
        ),
        StorageError::InvalidBackupName => (
            2,
            "INVALID_BACKUP_NAME",
            "backup name must be a generated basename",
        ),
        StorageError::BackupNotFound => (
            2,
            "BACKUP_NOT_FOUND",
            "the selected backup was not found in this HyperAI home",
        ),
        StorageError::HomeBusy => (
            3,
            "HOME_BUSY",
            "stop the local HyperAI service before restoring metadata",
        ),
        StorageError::UnsupportedSchema => (
            4,
            "UNSUPPORTED_SCHEMA",
            "the local metadata schema is not supported by this command",
        ),
        StorageError::InvalidBackup => (
            4,
            "INVALID_BACKUP",
            "the selected local metadata backup is invalid",
        ),
        StorageError::RestoreFailed {
            safety_backup_name,
            outcome_unknown,
        } => {
            let detail = if outcome_unknown {
                "the restore outcome is uncertain"
            } else {
                "the current metadata was left unchanged"
            };
            return CliError {
                exit_code: 6,
                code: "STORAGE_OPERATION_FAILED".to_owned(),
                message: format!(
                    "local metadata restore failed; {detail}; recovery backup: {safety_backup_name}"
                ),
            };
        }
        _ => (
            6,
            "STORAGE_OPERATION_FAILED",
            "the local storage operation failed",
        ),
    };
    CliError {
        exit_code,
        code: code.to_owned(),
        message: message.to_owned(),
    }
}

#[allow(clippy::too_many_lines)] // Keep the explicit agent CLI lifecycle and stdin prompt handling together.
fn run_agent(args: AgentArgs) -> Result<Value, CliError> {
    match args.command {
        AgentCommand::Profile(profile_args) => {
            let profile = profile_args.into_profile();
            profile.validate().map_err(|_| {
                CliError::operation(
                    "AGENT_PROFILE_INVALID",
                    "profile does not meet local ACP limits",
                )
            })?;
            Ok(json!({
                "schema_version": "hyperai.acp.profile-check.v1",
                "status": "valid_not_started",
                "name": profile.name,
                "executable": profile.executable,
                "working_directory": profile.working_directory,
                "argument_count": profile.args.len(),
                "environment_names": [],
                "tool_scope": "fixed_read_only_hyperai_tools",
                "process_sandbox": false,
            }))
        }
        AgentCommand::Connect(profile_args) => {
            let profile = profile_args.into_profile();
            profile.validate().map_err(|_| {
                CliError::operation(
                    "AGENT_PROFILE_INVALID",
                    "profile does not meet local ACP limits",
                )
            })?;
            let (client, base_url, bearer) = local_agent_service_client(args.home.as_deref())?;
            agent_request(
                &client,
                &base_url,
                &bearer,
                Method::POST,
                "/api/v1/agent/connect",
                Some(json!({ "profile": profile })),
            )
        }
        AgentCommand::Status => {
            let (client, base_url, bearer) = local_agent_service_client(args.home.as_deref())?;
            agent_request(
                &client,
                &base_url,
                &bearer,
                Method::GET,
                "/api/v1/agent/status",
                None,
            )
        }
        AgentCommand::Prompt => {
            if std::io::stdin().is_terminal() {
                return Err(CliError::operation(
                    "AGENT_PROMPT_REQUIRES_STDIN",
                    "pipe prompt text to stdin so it is not placed in process arguments",
                ));
            }
            let mut prompt = Vec::new();
            std::io::stdin()
                .take(16_385)
                .read_to_end(&mut prompt)
                .map_err(CliError::internal)?;
            if prompt.is_empty() || prompt.len() > 16_384 {
                return Err(CliError::operation(
                    "AGENT_PROMPT_LIMIT_EXCEEDED",
                    "prompt must contain 1–16,384 bytes",
                ));
            }
            let prompt = String::from_utf8(prompt).map_err(|_| {
                CliError::operation("AGENT_PROMPT_INVALID", "prompt must be valid UTF-8")
            })?;
            let (client, base_url, bearer) = local_agent_service_client(args.home.as_deref())?;
            agent_request(
                &client,
                &base_url,
                &bearer,
                Method::POST,
                "/api/v1/agent/prompt",
                Some(json!({ "prompt": prompt })),
            )
        }
        AgentCommand::Cancel => {
            let (client, base_url, bearer) = local_agent_service_client(args.home.as_deref())?;
            agent_request(
                &client,
                &base_url,
                &bearer,
                Method::POST,
                "/api/v1/agent/cancel",
                None,
            )
        }
        AgentCommand::Disconnect => {
            let (client, base_url, bearer) = local_agent_service_client(args.home.as_deref())?;
            agent_request(
                &client,
                &base_url,
                &bearer,
                Method::POST,
                "/api/v1/agent/disconnect",
                None,
            )
        }
    }
}

fn agent_request(
    client: &reqwest::blocking::Client,
    base_url: &str,
    bearer: &str,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value, CliError> {
    local_service_request(client, base_url, bearer, method, path, body, None)
}

fn local_service_request(
    client: &reqwest::blocking::Client,
    base_url: &str,
    bearer: &str,
    method: Method,
    path: &str,
    body: Option<Value>,
    idempotency_key: Option<String>,
) -> Result<Value, CliError> {
    let mut request = client
        .request(method, format!("{base_url}{path}"))
        .bearer_auth(bearer);
    if let Some(body) = body {
        request = request.json(&body);
    }
    if let Some(key) = idempotency_key {
        request = request.header("idempotency-key", key);
    }
    let response = request.send().map_err(CliError::internal)?;
    local_service_json_response(response)
}

fn local_service_json_response(response: reqwest::blocking::Response) -> Result<Value, CliError> {
    let status = response.status();
    if status.is_success() {
        return response.json::<Value>().map_err(CliError::internal);
    }
    let body = response.json::<Value>().unwrap_or(Value::Null);
    let error = hyperai_application::normalize_interface_error_v1(status.as_u16(), &body);
    Err(CliError::operation(error.code, error.message))
}

fn run_outcomes(outcomes: OutcomesArgs) -> Result<Value, CliError> {
    match outcomes.command {
        OutcomesCommand::Adapter(adapter) => run_adapter(adapter),
        OutcomesCommand::Claim(claim) => run_claim(claim),
        OutcomesCommand::Evidence(evidence) => run_evidence(evidence),
        OutcomesCommand::Demo(demo) => run_demo(demo),
        OutcomesCommand::Hip4(hip4) => run_hip4(hip4),
        OutcomesCommand::Settlement(settlement) => run_settlement(settlement),
        OutcomesCommand::Corpus(corpus) => run_corpus(corpus),
        OutcomesCommand::Beta(beta) => run_beta(beta),
    }
}

fn run_service(args: ServiceArgs) -> Result<Value, CliError> {
    match args.command {
        ServiceCommand::Start { home, port } => start_service(home.as_deref(), port),
        ServiceCommand::Status(args) => service_status(args.home.as_deref()),
        ServiceCommand::Stop(args) => stop_service(args.home.as_deref()),
    }
}

fn run_connections(args: &ConnectionsArgs) -> Result<Value, CliError> {
    let (client, base_url, bearer) = local_service_client(args.home.as_deref())?;
    let response = client
        .get(format!("{base_url}/api/v1/connections"))
        .bearer_auth(bearer)
        .send()
        .map_err(CliError::internal)?;
    local_service_json_response(response)
}

fn service_status(home: Option<&Path>) -> Result<Value, CliError> {
    let paths = hyperai_local_service::storage::paths_for_home(home).map_err(CliError::internal)?;
    let record = match hyperai_local_service::lifecycle::read_record(&paths) {
        Ok(record) => record,
        Err(hyperai_local_service::lifecycle::LifecycleError::Io(error))
            if error.kind() == std::io::ErrorKind::NotFound =>
        {
            return Ok(json!({
                "schema_version": "hyperai.local-service-status.v1",
                "status": "stopped",
                "ready": false,
                "effect_boundary": "LOCAL_LOOPBACK_ONLY",
            }));
        }
        Err(error) => {
            return Ok(json!({
                "schema_version": "hyperai.local-service-status.v1",
                "status": "unknown",
                "ready": false,
                "error": error.to_string(),
            }));
        }
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(CliError::internal)?;
    let health = client
        .get(format!("http://{}/healthz", record.address))
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .and_then(reqwest::blocking::Response::json::<Value>);
    let ready = health.as_ref().is_ok_and(|body| {
        body["schema_version"] == "hyperai.local-service.health.v1"
            && body["status"] == "ready"
            && body["instance_id"] == record.instance_id
    });
    Ok(json!({
        "schema_version": "hyperai.local-service-status.v1",
        "status": if ready { "ready" } else { "not_ready" },
        "ready": ready,
        "address": record.address,
        "instance_id": record.instance_id,
        "process_id": record.process_id,
        "started_at_unix_seconds": record.started_at_unix_seconds,
        "effect_boundary": "LOCAL_LOOPBACK_ONLY_NO_PROVIDER_NO_FUNDS",
    }))
}

fn start_service(home: Option<&Path>, port: u16) -> Result<Value, CliError> {
    let existing = service_status(home)?;
    if existing["ready"] == true {
        return Ok(json!({
            "schema_version": "hyperai.local-service-start.v1",
            "status": "already_running",
            "service": existing,
        }));
    }

    let executable = std::env::current_exe().map_err(CliError::internal)?;
    let mut command = Command::new(executable);
    command
        .arg("__serve")
        .arg("--port")
        .arg(port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(home) = home {
        command.arg("--home").arg(home);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().map_err(CliError::internal)?;
    for _ in 0..50 {
        thread::sleep(Duration::from_millis(100));
        let service = service_status(home)?;
        if service["ready"] == true {
            return Ok(json!({
                "schema_version": "hyperai.local-service-start.v1",
                "status": "started",
                "service": service,
            }));
        }
        if let Some(status) = child.try_wait().map_err(CliError::internal)? {
            return Err(CliError::operation(
                "SERVICE_START_FAILED",
                &format!("local service process exited with {status}"),
            ));
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    Err(CliError::operation(
        "SERVICE_START_TIMEOUT",
        "local service did not become ready within five seconds",
    ))
}

fn stop_service(home: Option<&Path>) -> Result<Value, CliError> {
    let paths = hyperai_local_service::storage::paths_for_home(home).map_err(CliError::internal)?;
    let record =
        hyperai_local_service::lifecycle::read_record(&paths).map_err(CliError::internal)?;
    let bearer_path = paths.runtime.join("bearer.token");
    let bearer = hyperai_local_service::AuthState::read_bearer_file(&bearer_path)
        .map_err(CliError::internal)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(CliError::internal)?;
    let response = client
        .post(format!("http://{}/api/v1/service/stop", record.address))
        .bearer_auth(bearer)
        .send()
        .map_err(CliError::internal)?;
    let body = local_service_json_response(response)?;
    for _ in 0..30 {
        thread::sleep(Duration::from_millis(100));
        if service_status(home).is_ok_and(|status| status["ready"] != true) {
            return Ok(body);
        }
    }
    Err(CliError::operation(
        "SERVICE_STOP_TIMEOUT",
        "service accepted shutdown but did not stop within three seconds",
    ))
}

fn run_data(args: DataArgs) -> Result<Value, CliError> {
    let (client, base_url, bearer) = local_service_client(args.home.as_deref())?;
    match args.command {
        DataCommand::SeedSynthetic => {
            let preview = data_request(
                &client,
                &base_url,
                &bearer,
                Method::GET,
                "/api/v1/data/synthetic/preview",
                None,
                None,
            )?;
            let response = data_request(
                &client,
                &base_url,
                &bearer,
                Method::POST,
                "/api/v1/data/synthetic/seed",
                Some(json!({ "preview_digest": preview["preview_digest"] })),
                Some(fresh_local_key("seed")),
            )?;
            Ok(json!({ "preview": preview, "manifest": response }))
        }
        DataCommand::Tables => data_request(
            &client,
            &base_url,
            &bearer,
            Method::GET,
            "/api/v1/data/catalog",
            None,
            None,
        ),
        DataCommand::Describe { dataset_id } => {
            validate_dataset_id(&dataset_id)?;
            data_request(
                &client,
                &base_url,
                &bearer,
                Method::GET,
                &format!("/api/v1/data/describe/{dataset_id}"),
                None,
                None,
            )
        }
        DataCommand::Sql {
            dataset_id,
            query,
            format,
            max_rows,
        } => {
            validate_dataset_id(&dataset_id)?;
            data_request(
                &client,
                &base_url,
                &bearer,
                Method::POST,
                "/api/v1/data/query",
                Some(json!({
                    "query_id": fresh_local_key("query"),
                    "request": {
                        "dataset_id": dataset_id,
                        "sql": query,
                        "format": match format { DataOutputArg::Json => "json", DataOutputArg::Csv => "csv" },
                        "max_rows": max_rows,
                    }
                })),
                None,
            )
        }
        DataCommand::Model { command } => run_data_model(command, &client, &base_url, &bearer),
        DataCommand::View { command } => run_data_view(command, &client, &base_url, &bearer),
    }
}

fn run_fleet(args: FleetArgs) -> Result<Value, CliError> {
    let (client, base_url, bearer) = local_service_client(args.home.as_deref())?;
    match args.command {
        FleetCommand::Inventory => fleet_request(
            &client,
            &base_url,
            &bearer,
            Method::GET,
            "/api/v1/fleet/inventory",
            None,
        ),
        FleetCommand::Doctor => fleet_request(
            &client,
            &base_url,
            &bearer,
            Method::GET,
            "/api/v1/fleet/doctor",
            None,
        ),
        FleetCommand::Preview {
            node_id,
            operation,
            expected_preview_digest,
        } => fleet_request(
            &client,
            &base_url,
            &bearer,
            Method::POST,
            "/api/v1/fleet/operation-preview",
            Some(json!({
                "node_id": node_id,
                "operation": operation.as_str(),
                "expected_preview_digest": expected_preview_digest,
            })),
        ),
        FleetCommand::Simulation {
            command: FleetSimulationCommand::Show,
        } => fleet_request(
            &client,
            &base_url,
            &bearer,
            Method::GET,
            "/api/v1/fleet/simulation",
            None,
        ),
        FleetCommand::Simulation {
            command:
                FleetSimulationCommand::Step {
                    operation,
                    expected_state_digest,
                    idempotency_key,
                    gpu_units,
                    fault,
                },
        } => fleet_request(
            &client,
            &base_url,
            &bearer,
            Method::POST,
            "/api/v1/fleet/simulation/transition",
            Some(json!({
                "expected_state_digest": expected_state_digest,
                "idempotency_key": idempotency_key,
                "operation": operation.as_str(),
                "gpu_units": gpu_units,
                "fault": fault.as_str(),
            })),
        ),
    }
}

fn run_eval(args: EvalArgs) -> Result<Value, CliError> {
    let (client, base_url, bearer) = local_service_client(args.home.as_deref())?;
    match args.command {
        EvalCommand::Catalog => eval_request(
            &client,
            &base_url,
            &bearer,
            Method::GET,
            "/api/v1/eval/catalog",
            None,
        ),
        EvalCommand::Run {
            expected_suite_digest,
            pair_id,
            idempotency_key,
        } => eval_request(
            &client,
            &base_url,
            &bearer,
            Method::POST,
            "/api/v1/eval/run",
            Some(json!({
                "suite_id": hyperai_eval::EVAL_SUITE_ID_V1,
                "expected_suite_digest": expected_suite_digest,
                "pair_id": pair_id,
                "idempotency_key": idempotency_key,
            })),
        ),
    }
}

fn run_trade(args: TradeArgs) -> Result<Value, CliError> {
    let (client, base_url, bearer) = local_service_client(args.home.as_deref())?;
    match args.command {
        TradeCommand::Instruments => trade_request(
            &client,
            &base_url,
            &bearer,
            Method::GET,
            "/api/v1/trade/catalog",
            None,
        ),
        TradeCommand::Series { instrument_id } => {
            validate_trade_instrument_id(&instrument_id)?;
            trade_request(
                &client,
                &base_url,
                &bearer,
                Method::GET,
                &format!("/api/v1/trade/series/{instrument_id}"),
                None,
            )
        }
        TradeCommand::Scenario {
            primary_instrument_id,
            comparison_instrument_id,
            price_shock_bps,
            availability_delta_units,
        } => {
            validate_trade_instrument_id(&primary_instrument_id)?;
            validate_trade_instrument_id(&comparison_instrument_id)?;
            let catalog = trade_request(
                &client,
                &base_url,
                &bearer,
                Method::GET,
                "/api/v1/trade/catalog",
                None,
            )?;
            trade_request(
                &client,
                &base_url,
                &bearer,
                Method::POST,
                "/api/v1/trade/scenario",
                Some(json!({
                    "expected_dataset_digest": catalog["dataset_digest"],
                    "primary_instrument_id": primary_instrument_id,
                    "comparison_instrument_id": comparison_instrument_id,
                    "price_shock_bps": price_shock_bps,
                    "availability_delta_units": availability_delta_units,
                })),
            )
        }
        TradeCommand::Paper {
            baseline_instrument_id,
            requested_units,
        } => {
            validate_trade_instrument_id(&baseline_instrument_id)?;
            let catalog = trade_request(
                &client,
                &base_url,
                &bearer,
                Method::GET,
                "/api/v1/trade/catalog",
                None,
            )?;
            trade_request(
                &client,
                &base_url,
                &bearer,
                Method::POST,
                "/api/v1/trade/paper",
                Some(json!({
                    "expected_dataset_digest": catalog["dataset_digest"],
                    "baseline_instrument_id": baseline_instrument_id,
                    "requested_units": requested_units,
                })),
            )
        }
    }
}

fn eval_request(
    client: &reqwest::blocking::Client,
    base_url: &str,
    bearer: &str,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value, CliError> {
    local_service_request(client, base_url, bearer, method, path, body, None)
}

fn trade_request(
    client: &reqwest::blocking::Client,
    base_url: &str,
    bearer: &str,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value, CliError> {
    local_service_request(client, base_url, bearer, method, path, body, None)
}

fn validate_trade_instrument_id(instrument_id: &str) -> Result<(), CliError> {
    if instrument_id.is_empty()
        || instrument_id.len() > 64
        || !instrument_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(CliError::operation(
            "INVALID_TRADE_INSTRUMENT_ID",
            "instrument id contains unsupported path characters",
        ));
    }
    Ok(())
}

fn fleet_request(
    client: &reqwest::blocking::Client,
    base_url: &str,
    bearer: &str,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value, CliError> {
    local_service_request(client, base_url, bearer, method, path, body, None)
}

fn run_data_model(
    command: DataModelCommand,
    client: &reqwest::blocking::Client,
    base_url: &str,
    bearer: &str,
) -> Result<Value, CliError> {
    match command {
        DataModelCommand::List => data_request(
            client,
            base_url,
            bearer,
            Method::GET,
            "/api/v1/data/models",
            None,
            None,
        ),
        DataModelCommand::Run { model_id } => {
            let models = data_request(
                client,
                base_url,
                bearer,
                Method::GET,
                "/api/v1/data/models",
                None,
                None,
            )?;
            let model = models["models"]
                .as_array()
                .and_then(|records| {
                    records
                        .iter()
                        .find(|record| record["model"]["model_id"] == model_id)
                })
                .ok_or_else(|| {
                    CliError::operation("DATASET_NOT_FOUND", "saved model was not found")
                })?;
            if model["input_state"] != "current" {
                return Err(CliError::operation(
                    "DATA_INTEGRITY_MISMATCH",
                    "saved model input snapshot is stale or missing",
                ));
            }
            let definition = &model["model"];
            let result = data_request(
                client,
                base_url,
                bearer,
                Method::POST,
                "/api/v1/data/query",
                Some(json!({
                    "query_id": fresh_local_key("model-run"),
                    "request": {
                        "dataset_id": definition["dataset_id"],
                        "sql": definition["sql"],
                        "format": "json",
                        "max_rows": 100,
                    }
                })),
                None,
            )?;
            Ok(json!({ "model": model, "result": result }))
        }
        DataModelCommand::Preview { definition } => run_data_definition(
            DataDefinitionCommand::Preview { definition },
            client,
            base_url,
            bearer,
            "/api/v1/data/models/preview",
            "/api/v1/data/models/save",
            "model",
        ),
        DataModelCommand::Save { definition } => run_data_definition(
            DataDefinitionCommand::Save { definition },
            client,
            base_url,
            bearer,
            "/api/v1/data/models/preview",
            "/api/v1/data/models/save",
            "model",
        ),
    }
}

fn run_data_view(
    command: DataViewCommand,
    client: &reqwest::blocking::Client,
    base_url: &str,
    bearer: &str,
) -> Result<Value, CliError> {
    match command {
        DataViewCommand::List => data_request(
            client,
            base_url,
            bearer,
            Method::GET,
            "/api/v1/data/views",
            None,
            None,
        ),
        DataViewCommand::Preview { definition } => run_data_definition(
            DataDefinitionCommand::Preview { definition },
            client,
            base_url,
            bearer,
            "/api/v1/data/views/preview",
            "/api/v1/data/views/save",
            "view",
        ),
        DataViewCommand::Save { definition } => run_data_definition(
            DataDefinitionCommand::Save { definition },
            client,
            base_url,
            bearer,
            "/api/v1/data/views/preview",
            "/api/v1/data/views/save",
            "view",
        ),
    }
}

fn run_data_definition(
    command: DataDefinitionCommand,
    client: &reqwest::blocking::Client,
    base_url: &str,
    bearer: &str,
    preview_path: &str,
    save_path: &str,
    kind: &str,
) -> Result<Value, CliError> {
    let (definition, save) = match command {
        DataDefinitionCommand::Preview { definition } => {
            (read_data_definition(&definition)?, false)
        }
        DataDefinitionCommand::Save { definition } => (read_data_definition(&definition)?, true),
    };
    let preview = data_request(
        client,
        base_url,
        bearer,
        Method::POST,
        preview_path,
        Some(definition.clone()),
        None,
    )?;
    if !save {
        return Ok(json!({ "preview": preview, "definition": definition }));
    }
    let save_request = match kind {
        "model" => json!({ "model": definition, "preview_digest": preview["preview_digest"] }),
        "view" => json!({ "view": definition, "preview_digest": preview["preview_digest"] }),
        _ => return Err(CliError::internal("unknown local definition kind")),
    };
    let saved = data_request(
        client,
        base_url,
        bearer,
        Method::POST,
        save_path,
        Some(save_request),
        Some(fresh_local_key(kind)),
    )?;
    Ok(json!({ "preview": preview, "saved": saved }))
}

fn read_data_definition(path: &Path) -> Result<Value, CliError> {
    let bytes = fs::read(path).map_err(CliError::internal)?;
    if bytes.len() > 16 * 1024 {
        return Err(CliError::operation(
            "DATA_LIMIT_EXCEEDED",
            "definition JSON exceeds the 16 KiB local limit",
        ));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| CliError::operation("INVALID_DEFINITION", "definition must be valid JSON"))
}

fn local_service_client(
    home: Option<&Path>,
) -> Result<(reqwest::blocking::Client, String, String), CliError> {
    let paths = hyperai_local_service::storage::paths_for_home(home).map_err(CliError::internal)?;
    let record =
        hyperai_local_service::lifecycle::read_record(&paths).map_err(CliError::internal)?;
    let bearer =
        hyperai_local_service::AuthState::read_bearer_file(&paths.runtime.join("bearer.token"))
            .map_err(CliError::internal)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
        .map_err(CliError::internal)?;
    Ok((client, format!("http://{}", record.address), bearer))
}

fn local_agent_service_client(
    home: Option<&Path>,
) -> Result<(reqwest::blocking::Client, String, String), CliError> {
    let paths = hyperai_local_service::storage::paths_for_home(home).map_err(CliError::internal)?;
    let record =
        hyperai_local_service::lifecycle::read_record(&paths).map_err(CliError::internal)?;
    let bearer =
        hyperai_local_service::AuthState::read_bearer_file(&paths.runtime.join("bearer.token"))
            .map_err(CliError::internal)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(CliError::internal)?;
    Ok((client, format!("http://{}", record.address), bearer))
}

fn data_request(
    client: &reqwest::blocking::Client,
    base_url: &str,
    bearer: &str,
    method: Method,
    path: &str,
    body: Option<Value>,
    idempotency_key: Option<String>,
) -> Result<Value, CliError> {
    local_service_request(
        client,
        base_url,
        bearer,
        method,
        path,
        body,
        idempotency_key,
    )
}

fn validate_dataset_id(dataset_id: &str) -> Result<(), CliError> {
    if dataset_id.is_empty()
        || dataset_id.len() > 96
        || !dataset_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(CliError::operation(
            "INVALID_DATASET_ID",
            "dataset id contains unsupported path characters",
        ));
    }
    Ok(())
}

fn fresh_local_key(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("{prefix}-{}-{nanos}", std::process::id())
}

fn run_desk(args: &HomeArgs) -> Result<Value, CliError> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(CliError::operation(
            "TERMINAL_REQUIRED",
            "hyperai desk requires an interactive terminal",
        ));
    }
    let paths = hyperai_local_service::storage::paths_for_home(args.home.as_deref())
        .map_err(CliError::internal)?;
    let record =
        hyperai_local_service::lifecycle::read_record(&paths).map_err(CliError::internal)?;
    let token =
        hyperai_local_service::AuthState::read_bearer_file(&paths.runtime.join("bearer.token"))
            .map_err(CliError::internal)?;
    hyperai_tui::run(&format!("http://{}", record.address), &token).map_err(CliError::internal)?;
    Ok(json!({
        "schema_version": "hyperai.local-desk.exit.v1",
        "status": "closed",
        "service_address": record.address,
    }))
}

fn run_beta(args: BetaArgs) -> Result<Value, CliError> {
    match args.command {
        BetaCommand::Rehearse {
            artifact_root,
            run_id,
        } => run_beta_rehearsal_with_provider(
            &artifact_root,
            &run_id,
            &CommandBetaProvenanceProvider,
        ),
        BetaCommand::Finalize {
            artifact_root,
            run_id,
        } => finalize_beta_run(&artifact_root, &run_id),
        BetaCommand::Verify {
            artifact_root,
            run_id,
        } => verify_beta_run(&artifact_root, &run_id),
    }
}

fn embedded_legacy_beta_profile()
-> Result<(LocalBetaProfile, outcomes_domain::Sha256Digest), CliError> {
    let profile: LocalBetaProfile =
        serde_json::from_str(LEGACY_LOCAL_BETA_PROFILE).map_err(CliError::internal)?;
    validate_beta_profile(&profile)?;
    let digest = domain_separated_digest("hsai.local-beta-profile.v1", &profile)
        .map_err(CliError::internal)?;
    Ok((profile, digest))
}

fn embedded_beta_profile_v2()
-> Result<(LocalBetaProfileV2, outcomes_domain::Sha256Digest), CliError> {
    let profile: LocalBetaProfileV2 =
        serde_json::from_str(LOCAL_BETA_PROFILE_V2).map_err(CliError::internal)?;
    validate_beta_profile_v2(&profile)?;
    let digest = domain_separated_digest("hsai.local-beta-profile.v2", &profile)
        .map_err(CliError::internal)?;
    Ok((profile, digest))
}

fn embedded_beta_profile_v3()
-> Result<(LocalBetaProfileV2, outcomes_domain::Sha256Digest), CliError> {
    let profile: LocalBetaProfileV2 =
        serde_json::from_str(LOCAL_BETA_PROFILE).map_err(CliError::internal)?;
    validate_beta_profile_v3(&profile)?;
    let digest = domain_separated_digest("hsai.local-beta-profile.v3", &profile)
        .map_err(CliError::internal)?;
    Ok((profile, digest))
}

fn finalize_beta_run(artifact_root: &Path, run_id: &str) -> Result<Value, CliError> {
    validate_run_id(run_id)?;
    let run_root = artifact_root.join(run_id);
    let profile_digest = verify_beta_run_semantics(&run_root, run_id)?;
    let inventory =
        finalize_run_inventory(&run_root, run_id, profile_digest).map_err(map_inventory_error)?;
    let verified_profile_digest = verify_beta_run_semantics(&run_root, run_id)?;
    if verified_profile_digest != profile_digest {
        return Err(CliError::operation(
            "BETA_PROFILE_DIGEST_MISMATCH",
            "profile identity changed during beta finalization",
        ));
    }
    let verified =
        verify_run_inventory(&run_root, run_id, profile_digest).map_err(map_inventory_error)?;
    if inventory != verified {
        return Err(CliError::operation(
            "BETA_INVENTORY_VERIFICATION_FAILED",
            "created inventory differs from its read-only verification",
        ));
    }
    serde_json::to_value(inventory).map_err(CliError::internal)
}

fn verify_beta_run(artifact_root: &Path, run_id: &str) -> Result<Value, CliError> {
    validate_run_id(run_id)?;
    let run_root = artifact_root.join(run_id);
    let profile_digest = verify_beta_run_semantics(&run_root, run_id)?;
    let inventory =
        verify_run_inventory(&run_root, run_id, profile_digest).map_err(map_inventory_error)?;
    serde_json::to_value(inventory).map_err(CliError::internal)
}

fn map_inventory_error(error: beta_inventory::InventoryError) -> CliError {
    let code = error.code;
    drop(error);
    CliError::operation(code, "beta retained-artifact inventory verification failed")
}

fn beta_runtime() -> Result<tokio::runtime::Runtime, CliError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(CliError::internal)
}

fn raw_file_digest(path: &Path) -> Result<outcomes_domain::Sha256Digest, CliError> {
    let bytes =
        fs::read(path).map_err(|error| CliError::input("FILE_READ_FAILED", &error.to_string()))?;
    Ok(outcomes_domain::Sha256Digest::from_bytes(
        Sha256::digest(bytes).into(),
    ))
}

#[allow(clippy::too_many_lines)]
fn run_beta_rehearsal_with_provider(
    artifact_root: &Path,
    run_id: &str,
    provenance_provider: &impl BetaProvenanceProvider,
) -> Result<Value, CliError> {
    run_beta_rehearsal_with_provider_and_seal(artifact_root, run_id, provenance_provider, true)
}

#[allow(clippy::too_many_lines)]
fn run_beta_rehearsal_with_provider_and_seal(
    artifact_root: &Path,
    run_id: &str,
    provenance_provider: &impl BetaProvenanceProvider,
    seal: bool,
) -> Result<Value, CliError> {
    validate_run_id(run_id)?;
    let profile: LocalBetaProfileV2 =
        serde_json::from_str(LOCAL_BETA_PROFILE).map_err(CliError::internal)?;
    validate_beta_profile_v3(&profile)?;
    let provenance = provenance_provider
        .collect(Path::new("."))
        .map_err(|error| {
            CliError::operation(error.code, "bounded beta provenance collection failed")
        })?;
    validate_recorded_provenance(&provenance).map_err(|error| {
        CliError::operation(error.code, "bounded beta provenance validation failed")
    })?;
    let provenance_digest =
        domain_separated_digest("hsai.local-beta-run-provenance.v1", &provenance)
            .map_err(CliError::internal)?;

    fs::create_dir_all(artifact_root)
        .map_err(|error| CliError::input("ARTIFACT_ROOT_CREATE_FAILED", &error.to_string()))?;
    let run_root = artifact_root.join(run_id);
    fs::create_dir(&run_root).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            CliError::operation("BETA_RUN_COLLISION", "beta run directory already exists")
        } else {
            CliError::input("BETA_RUN_CREATE_FAILED", &error.to_string())
        }
    })?;
    let profile_digest = domain_separated_digest("hsai.local-beta-profile.v3", &profile)
        .map_err(CliError::internal)?;
    fs::write(run_root.join("local-beta-profile.json"), LOCAL_BETA_PROFILE)
        .map_err(|error| CliError::input("ARTIFACT_WRITE_FAILED", &error.to_string()))?;
    fs::write(
        run_root.join("run-provenance.json"),
        serde_json::to_vec_pretty(&provenance).map_err(CliError::internal)?,
    )
    .map_err(|error| CliError::input("ARTIFACT_WRITE_FAILED", &error.to_string()))?;

    let mut scenario_results = Vec::with_capacity(BETA_SCENARIOS.len());
    for (name, scenario) in BETA_SCENARIOS {
        let scenario_root = run_root.join(name);
        let report = run_local_scenario(scenario, &scenario_root)
            .map_err(|error| CliError::operation("BETA_SCENARIO_FAILED", &error.to_string()))?;
        validate_scenario_report(name, &report)?;
        let bundle_names: &[&str] = if scenario == LocalScenario::DisputedFalseCounterexampleTrue {
            &[
                "original-evidence-bundle.json",
                "replacement-evidence-bundle.json",
            ]
        } else {
            &["evidence-bundle.json"]
        };
        let mut bundles = Vec::with_capacity(bundle_names.len());
        for bundle_name in bundle_names {
            let bundle_path = scenario_root.join(bundle_name);
            let digest = verify_bundle_from_disk(&bundle_path, &scenario_root)?;
            bundles.push(BetaBundleResult {
                relative_path: format!("{name}/{bundle_name}"),
                bundle_digest: digest,
            });
        }
        if bundles.last().map(|bundle| bundle.bundle_digest) != Some(report.evidence_bundle_digest)
        {
            return Err(CliError::operation(
                "BETA_SCENARIO_MATRIX_MISMATCH",
                "primary disk-verified bundle digest differs from the scenario report",
            ));
        }
        fs::write(
            scenario_root.join("scenario-report.json"),
            serde_json::to_vec_pretty(&report).map_err(CliError::internal)?,
        )
        .map_err(|error| CliError::input("ARTIFACT_WRITE_FAILED", &error.to_string()))?;
        scenario_results.push(BetaScenarioResult {
            scenario: name.to_owned(),
            status: "PASS".to_owned(),
            truth: report.truth,
            finality: report.finality,
            execution_outcomes: report.execution_outcomes,
            mock_effect_count: report.mock_effect_count,
            reconciliation_status: report.reconciliation_status,
            evidence_bundles: bundles,
        });
    }

    let recovery_root = run_root.join("operational-checks/sqlite-settlement-recovery");
    let recovery_report = beta_runtime()?
        .block_on(run_local_lost_receipt_recovery(&recovery_root))
        .map_err(|error| {
            CliError::operation("BETA_OPERATIONAL_CHECK_FAILED", &error.to_string())
        })?;
    let recovery_report_path = recovery_root.join("local-recovery-report.json");
    let recovery_database_path = recovery_root.join("settlement-recovery.sqlite3");
    let mut operational_checks = vec![BetaOperationalCheckResult {
        check: "sqlite-settlement-recovery".to_owned(),
        status: "PASS".to_owned(),
        report_file: "operational-checks/sqlite-settlement-recovery/local-recovery-report.json"
            .to_owned(),
        report_digest: raw_file_digest(&recovery_report_path)?,
        database_file: "operational-checks/sqlite-settlement-recovery/settlement-recovery.sqlite3"
            .to_owned(),
        database_digest: raw_file_digest(&recovery_database_path)?,
    }];
    beta_runtime()?
        .block_on(verify_local_lost_receipt_recovery(
            &recovery_root,
            &recovery_report,
        ))
        .map_err(|error| {
            CliError::operation("BETA_OPERATIONAL_CHECK_FAILED", &error.to_string())
        })?;

    let authorization_root = run_root.join("operational-checks/authorization-denial");
    let authorization_report = beta_runtime()?
        .block_on(run_local_authorization_denial_acceptance(
            &authorization_root,
        ))
        .map_err(|error| {
            CliError::operation("BETA_OPERATIONAL_CHECK_FAILED", &error.to_string())
        })?;
    let authorization_report_path = authorization_root.join("authorization-denial-report.json");
    let authorization_database_path = authorization_root.join("authorization-denial.sqlite3");
    operational_checks.push(BetaOperationalCheckResult {
        check: "authorization-denial".to_owned(),
        status: "PASS".to_owned(),
        report_file: "operational-checks/authorization-denial/authorization-denial-report.json"
            .to_owned(),
        report_digest: raw_file_digest(&authorization_report_path)?,
        database_file: "operational-checks/authorization-denial/authorization-denial.sqlite3"
            .to_owned(),
        database_digest: raw_file_digest(&authorization_database_path)?,
    });
    beta_runtime()?
        .block_on(verify_local_authorization_denial_acceptance(
            &authorization_root,
            &authorization_report,
        ))
        .map_err(|error| {
            CliError::operation("BETA_OPERATIONAL_CHECK_FAILED", &error.to_string())
        })?;

    run_exported_tamper_check(&run_root)?;
    let manifest = json!({
        "schema_version": "hsai.local-beta-run-manifest.v4",
        "profile_schema_version": profile.schema_version,
        "profile_id": profile.profile_id,
        "profile_file": "local-beta-profile.json",
        "profile_digest": profile_digest,
        "provenance_file": "run-provenance.json",
        "provenance_digest": provenance_digest,
        "run_id": run_id,
        "status": "PASS",
        "claim_ceiling": profile.claim_ceiling,
        "artifact_classification": profile.artifact_classification,
        "scenario_results": scenario_results,
        "operational_checks": operational_checks,
        "tamper_check": {"status": "PASS", "expected_result": "VERIFICATION_REJECTED"},
        "external_effect": "mock only; no network writes",
    });
    fs::write(
        run_root.join("beta-run-manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(CliError::internal)?,
    )
    .map_err(|error| CliError::input("ARTIFACT_WRITE_FAILED", &error.to_string()))?;
    if seal {
        finalize_beta_run(artifact_root, run_id)?;
    }
    Ok(manifest)
}

fn validate_run_id(run_id: &str) -> Result<(), CliError> {
    let valid = !run_id.is_empty()
        && run_id.len() <= 64
        && run_id
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && run_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    if !valid {
        return Err(CliError::input(
            "UNSAFE_BETA_RUN_ID",
            "run ID must be 1-64 ASCII alphanumeric, hyphen, or underscore characters and start alphanumeric",
        ));
    }
    Ok(())
}

fn validate_beta_profile(profile: &LocalBetaProfile) -> Result<(), CliError> {
    let expected = BETA_SCENARIOS
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect::<Vec<_>>();
    if profile.schema_version != "hsai.local-beta-profile.v1"
        || profile.profile_id != "hermetic-trusted-operator-v1"
        || profile.network != "MOCK"
        || profile.backend != "mock-hip4"
        || profile.live_writes
        || profile.serial_concurrency != 1
        || profile.scenarios != expected
        || profile.artifact_classification != "PUBLIC"
        || !profile.stop_on_failure
    {
        return Err(CliError::operation(
            "UNSAFE_LOCAL_BETA_PROFILE",
            "local beta profile violates the frozen mock-only serial rehearsal contract",
        ));
    }
    Ok(())
}

fn validate_beta_profile_v2(profile: &LocalBetaProfileV2) -> Result<(), CliError> {
    let expected = BETA_SCENARIOS
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect::<Vec<_>>();
    if profile.schema_version != "hsai.local-beta-profile.v2"
        || profile.profile_id != "hermetic-trusted-operator-v2"
        || profile.network != "MOCK"
        || profile.backend != "mock-hip4"
        || profile.live_writes
        || profile.serial_concurrency != 1
        || profile.scenarios != expected
        || profile.operational_checks != ["sqlite-settlement-recovery"]
        || profile.artifact_classification != "PUBLIC"
        || !profile.stop_on_failure
    {
        return Err(CliError::operation(
            "UNSAFE_LOCAL_BETA_PROFILE",
            "local beta profile violates the frozen mock-only serial rehearsal contract",
        ));
    }
    Ok(())
}

fn validate_beta_profile_v3(profile: &LocalBetaProfileV2) -> Result<(), CliError> {
    let expected = BETA_SCENARIOS
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect::<Vec<_>>();
    if profile.schema_version != "hsai.local-beta-profile.v3"
        || profile.profile_id != "hermetic-trusted-operator-v3"
        || profile.network != "MOCK"
        || profile.backend != "mock-hip4"
        || profile.live_writes
        || profile.serial_concurrency != 1
        || profile.scenarios != expected
        || profile.operational_checks != ["sqlite-settlement-recovery", "authorization-denial"]
        || profile.artifact_classification != "PUBLIC"
        || !profile.stop_on_failure
    {
        return Err(CliError::operation(
            "UNSAFE_LOCAL_BETA_PROFILE",
            "local beta profile violates the frozen mock-only serial rehearsal contract",
        ));
    }
    Ok(())
}

fn validate_scenario_report(name: &str, report: &ScenarioReport) -> Result<(), CliError> {
    let (truth, finality, outcomes, effects, reconciliation, requires_ineligibility) = match name {
        "true" | "disputed-false-counterexample-true" => (
            ClaimTruth::True,
            ResolutionFinality::Final,
            vec![ExecutionOutcome::Pass, ExecutionOutcome::Pass],
            1,
            Some(ReconciliationStatus::Confirmed),
            false,
        ),
        "false" => (
            ClaimTruth::False,
            ResolutionFinality::Final,
            vec![],
            0,
            None,
            false,
        ),
        "invalid-then-valid" => (
            ClaimTruth::True,
            ResolutionFinality::Final,
            vec![
                ExecutionOutcome::Fail,
                ExecutionOutcome::Pass,
                ExecutionOutcome::Pass,
            ],
            1,
            Some(ReconciliationStatus::Confirmed),
            false,
        ),
        "incomplete-observation" => (
            ClaimTruth::Unresolved,
            ResolutionFinality::None,
            vec![],
            0,
            None,
            true,
        ),
        "divergent-replays" => (
            ClaimTruth::Unresolved,
            ResolutionFinality::None,
            vec![ExecutionOutcome::Pass, ExecutionOutcome::Fail],
            0,
            None,
            true,
        ),
        "runner-error" => (
            ClaimTruth::Unresolved,
            ResolutionFinality::None,
            vec![ExecutionOutcome::Error],
            0,
            None,
            false,
        ),
        _ => {
            return Err(CliError::operation(
                "BETA_SCENARIO_MATRIX_MISMATCH",
                "unknown beta scenario",
            ));
        }
    };
    let ineligibility_matches = if requires_ineligibility {
        report
            .settlement_eligibility
            .as_ref()
            .is_some_and(|assessment| {
                !assessment.eligible_for_preflight
                    && assessment.blockers
                        == [
                            SettlementEligibilityBlocker::ResolutionRequired,
                            SettlementEligibilityBlocker::FinalResolutionRequired,
                        ]
            })
    } else {
        report.settlement_eligibility.is_none()
    };
    if report.truth != truth
        || report.finality != finality
        || report.execution_outcomes != outcomes
        || report.mock_effect_count != effects
        || report.reconciliation_status != reconciliation
        || !report.evidence_verified
        || !ineligibility_matches
    {
        return Err(CliError::operation(
            "BETA_SCENARIO_MATRIX_MISMATCH",
            "scenario report violated the frozen truth/finality/outcome/effect matrix",
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn verify_beta_run_semantics(
    run_root: &Path,
    run_id: &str,
) -> Result<outcomes_domain::Sha256Digest, CliError> {
    let manifest: Value = read_json_path(&run_root.join("beta-run-manifest.json"))?;
    match manifest.get("schema_version").and_then(Value::as_str) {
        Some("hsai.local-beta-run-manifest.v2") => {
            let (profile, digest) = embedded_legacy_beta_profile()?;
            verify_beta_run_semantics_v2(run_root, run_id, &profile)?;
            Ok(digest)
        }
        Some("hsai.local-beta-run-manifest.v3") => {
            let (profile, digest) = embedded_beta_profile_v2()?;
            verify_beta_run_semantics_v3(run_root, run_id, &profile)?;
            Ok(digest)
        }
        Some("hsai.local-beta-run-manifest.v4") => {
            let (profile, digest) = embedded_beta_profile_v3()?;
            verify_beta_run_semantics_v4(run_root, run_id, &profile)?;
            Ok(digest)
        }
        _ => Err(CliError::operation(
            "BETA_MANIFEST_UNSUPPORTED_SCHEMA",
            "sealed beta verification requires run manifest v2, v3, or v4",
        )),
    }
}

#[allow(clippy::too_many_lines)]
fn verify_beta_run_semantics_v2(
    run_root: &Path,
    run_id: &str,
    profile: &LocalBetaProfile,
) -> Result<(), CliError> {
    let manifest_value: Value = read_json_path(&run_root.join("beta-run-manifest.json"))?;
    if manifest_value.get("schema_version").and_then(Value::as_str)
        != Some("hsai.local-beta-run-manifest.v2")
    {
        return Err(CliError::operation(
            "BETA_MANIFEST_UNSUPPORTED_SCHEMA",
            "sealed beta verification requires run manifest v2",
        ));
    }
    let manifest: BetaRunManifestV2 = serde_json::from_value(manifest_value).map_err(|_| {
        CliError::operation(
            "BETA_MANIFEST_INVALID",
            "run manifest does not match the strict v2 contract",
        )
    })?;
    let expected_profile_digest = domain_separated_digest("hsai.local-beta-profile.v1", profile)
        .map_err(CliError::internal)?;
    if manifest.schema_version != "hsai.local-beta-run-manifest.v2"
        || manifest.run_id != run_id
        || manifest.status != "PASS"
        || manifest.profile_schema_version != profile.schema_version
        || manifest.profile_id != profile.profile_id
        || manifest.profile_file != "local-beta-profile.json"
        || manifest.profile_digest != expected_profile_digest
        || manifest.provenance_file != "run-provenance.json"
        || manifest.claim_ceiling != profile.claim_ceiling
        || manifest.artifact_classification != profile.artifact_classification
        || manifest.external_effect != "mock only; no network writes"
        || manifest.tamper_check.status != "PASS"
        || manifest.tamper_check.expected_result != "VERIFICATION_REJECTED"
    {
        return Err(CliError::operation(
            "BETA_MANIFEST_SEMANTICS_MISMATCH",
            "beta manifest identity, profile, status, or claim boundary differs from the frozen contract",
        ));
    }

    let copied_profile: LocalBetaProfile =
        read_json_path(&run_root.join("local-beta-profile.json"))?;
    validate_beta_profile(&copied_profile)?;
    let copied_profile_digest =
        domain_separated_digest("hsai.local-beta-profile.v1", &copied_profile)
            .map_err(CliError::internal)?;
    if copied_profile_digest != expected_profile_digest {
        return Err(CliError::operation(
            "BETA_PROFILE_DIGEST_MISMATCH",
            "copied profile differs from the embedded frozen profile",
        ));
    }

    let provenance: BetaRunProvenanceV1 = read_json_path(&run_root.join("run-provenance.json"))?;
    validate_recorded_provenance(&provenance).map_err(|error| {
        CliError::operation(error.code, "recorded beta provenance validation failed")
    })?;
    let provenance_digest =
        domain_separated_digest("hsai.local-beta-run-provenance.v1", &provenance)
            .map_err(CliError::internal)?;
    if provenance_digest != manifest.provenance_digest {
        return Err(CliError::operation(
            "BETA_PROVENANCE_DIGEST_MISMATCH",
            "run provenance digest differs from the manifest binding",
        ));
    }

    if manifest.scenario_results.len() != BETA_SCENARIOS.len() {
        return Err(CliError::operation(
            "BETA_SCENARIO_MATRIX_MISMATCH",
            "manifest does not contain exactly the frozen scenario matrix",
        ));
    }
    let mut verified_bundle_count = 0_usize;
    for ((expected_name, scenario), result) in BETA_SCENARIOS.iter().zip(&manifest.scenario_results)
    {
        if result.scenario != *expected_name || result.status != "PASS" {
            return Err(CliError::operation(
                "BETA_SCENARIO_MATRIX_MISMATCH",
                "manifest scenario order, identity, or status differs from the frozen matrix",
            ));
        }
        let scenario_root = run_root.join(expected_name);
        let report: ScenarioReport = read_json_path(&scenario_root.join("scenario-report.json"))?;
        validate_scenario_report(expected_name, &report)?;
        if result.truth != report.truth
            || result.finality != report.finality
            || result.execution_outcomes != report.execution_outcomes
            || result.mock_effect_count != report.mock_effect_count
            || result.reconciliation_status != report.reconciliation_status
        {
            return Err(CliError::operation(
                "BETA_SCENARIO_MATRIX_MISMATCH",
                "manifest scenario result differs from its validated report",
            ));
        }
        let expected_bundle_names: &[&str] =
            if *scenario == LocalScenario::DisputedFalseCounterexampleTrue {
                &[
                    "original-evidence-bundle.json",
                    "replacement-evidence-bundle.json",
                ]
            } else {
                &["evidence-bundle.json"]
            };
        if result.evidence_bundles.len() != expected_bundle_names.len() {
            return Err(CliError::operation(
                "BETA_SCENARIO_MATRIX_MISMATCH",
                "manifest bundle count differs from the frozen matrix",
            ));
        }
        for (bundle, expected_bundle_name) in result
            .evidence_bundles
            .iter()
            .zip(expected_bundle_names.iter())
        {
            let expected_relative_path = format!("{expected_name}/{expected_bundle_name}");
            if bundle.relative_path != expected_relative_path {
                return Err(CliError::operation(
                    "BETA_SCENARIO_MATRIX_MISMATCH",
                    "manifest bundle path differs from the frozen matrix",
                ));
            }
            let observed_digest =
                verify_bundle_from_disk(&run_root.join(&bundle.relative_path), &scenario_root)?;
            if observed_digest != bundle.bundle_digest {
                return Err(CliError::operation(
                    "BETA_SCENARIO_MATRIX_MISMATCH",
                    "disk-verified bundle digest differs from the manifest binding",
                ));
            }
            verified_bundle_count += 1;
        }
        if result
            .evidence_bundles
            .last()
            .map(|bundle| bundle.bundle_digest)
            != Some(report.evidence_bundle_digest)
        {
            return Err(CliError::operation(
                "BETA_SCENARIO_MATRIX_MISMATCH",
                "primary bundle digest differs from the scenario report binding",
            ));
        }
    }
    if verified_bundle_count != 8 {
        return Err(CliError::operation(
            "BETA_SCENARIO_MATRIX_MISMATCH",
            "beta run must contain exactly eight verified evidence bundles",
        ));
    }
    verify_exported_tamper_artifact(run_root)
}

#[allow(clippy::too_many_lines)]
fn verify_beta_run_semantics_v3(
    run_root: &Path,
    run_id: &str,
    profile: &LocalBetaProfileV2,
) -> Result<(), CliError> {
    let manifest: BetaRunManifestV3 = read_json_path(&run_root.join("beta-run-manifest.json"))?;
    let expected_profile_digest = domain_separated_digest("hsai.local-beta-profile.v2", profile)
        .map_err(CliError::internal)?;
    if manifest.schema_version != "hsai.local-beta-run-manifest.v3"
        || manifest.run_id != run_id
        || manifest.status != "PASS"
        || manifest.profile_schema_version != profile.schema_version
        || manifest.profile_id != profile.profile_id
        || manifest.profile_file != "local-beta-profile.json"
        || manifest.profile_digest != expected_profile_digest
        || manifest.provenance_file != "run-provenance.json"
        || manifest.claim_ceiling != profile.claim_ceiling
        || manifest.artifact_classification != profile.artifact_classification
        || manifest.external_effect != "mock only; no network writes"
        || manifest.tamper_check.status != "PASS"
        || manifest.tamper_check.expected_result != "VERIFICATION_REJECTED"
    {
        return Err(CliError::operation(
            "BETA_MANIFEST_SEMANTICS_MISMATCH",
            "beta manifest identity, profile, status, or claim boundary differs from the frozen contract",
        ));
    }
    let copied_profile: LocalBetaProfileV2 =
        read_json_path(&run_root.join("local-beta-profile.json"))?;
    validate_beta_profile_v2(&copied_profile)?;
    let copied_profile_digest =
        domain_separated_digest("hsai.local-beta-profile.v2", &copied_profile)
            .map_err(CliError::internal)?;
    if copied_profile_digest != expected_profile_digest {
        return Err(CliError::operation(
            "BETA_PROFILE_DIGEST_MISMATCH",
            "copied profile differs from the embedded frozen profile",
        ));
    }
    let provenance: BetaRunProvenanceV1 = read_json_path(&run_root.join("run-provenance.json"))?;
    validate_recorded_provenance(&provenance).map_err(|error| {
        CliError::operation(error.code, "recorded beta provenance validation failed")
    })?;
    let provenance_digest =
        domain_separated_digest("hsai.local-beta-run-provenance.v1", &provenance)
            .map_err(CliError::internal)?;
    if provenance_digest != manifest.provenance_digest {
        return Err(CliError::operation(
            "BETA_PROVENANCE_DIGEST_MISMATCH",
            "run provenance digest differs from the manifest binding",
        ));
    }
    verify_beta_scenario_results(run_root, &manifest.scenario_results)?;
    let [check] = manifest.operational_checks.as_slice() else {
        return Err(CliError::operation(
            "BETA_OPERATIONAL_CHECK_MISMATCH",
            "beta manifest must contain exactly the frozen operational check",
        ));
    };
    let expected_root = "operational-checks/sqlite-settlement-recovery";
    if check.check != "sqlite-settlement-recovery"
        || check.status != "PASS"
        || check.report_file != format!("{expected_root}/local-recovery-report.json")
        || check.database_file != format!("{expected_root}/settlement-recovery.sqlite3")
    {
        return Err(CliError::operation(
            "BETA_OPERATIONAL_CHECK_MISMATCH",
            "operational check identity, status, or artifact path differs from the frozen contract",
        ));
    }
    if raw_file_digest(&run_root.join(&check.report_file))? != check.report_digest
        || raw_file_digest(&run_root.join(&check.database_file))? != check.database_digest
    {
        return Err(CliError::operation(
            "BETA_OPERATIONAL_CHECK_MISMATCH",
            "operational check artifact digest differs from the manifest binding",
        ));
    }
    let recovery_root = run_root.join(expected_root);
    let report: LocalRecoveryReportV1 =
        read_json_path(&recovery_root.join("local-recovery-report.json"))?;
    beta_runtime()?
        .block_on(verify_local_lost_receipt_recovery(&recovery_root, &report))
        .map_err(|error| {
            CliError::operation("BETA_OPERATIONAL_CHECK_FAILED", &error.to_string())
        })?;
    verify_exported_tamper_artifact(run_root)
}

#[allow(clippy::too_many_lines)]
fn verify_beta_run_semantics_v4(
    run_root: &Path,
    run_id: &str,
    profile: &LocalBetaProfileV2,
) -> Result<(), CliError> {
    let manifest: BetaRunManifestV4 = read_json_path(&run_root.join("beta-run-manifest.json"))?;
    let expected_profile_digest = domain_separated_digest("hsai.local-beta-profile.v3", profile)
        .map_err(CliError::internal)?;
    if manifest.schema_version != "hsai.local-beta-run-manifest.v4"
        || manifest.run_id != run_id
        || manifest.status != "PASS"
        || manifest.profile_schema_version != profile.schema_version
        || manifest.profile_id != profile.profile_id
        || manifest.profile_file != "local-beta-profile.json"
        || manifest.profile_digest != expected_profile_digest
        || manifest.provenance_file != "run-provenance.json"
        || manifest.claim_ceiling != profile.claim_ceiling
        || manifest.artifact_classification != profile.artifact_classification
        || manifest.external_effect != "mock only; no network writes"
        || manifest.tamper_check.status != "PASS"
        || manifest.tamper_check.expected_result != "VERIFICATION_REJECTED"
    {
        return Err(CliError::operation(
            "BETA_MANIFEST_SEMANTICS_MISMATCH",
            "beta manifest identity, profile, status, or claim boundary differs from the frozen contract",
        ));
    }
    let copied_profile: LocalBetaProfileV2 =
        read_json_path(&run_root.join("local-beta-profile.json"))?;
    validate_beta_profile_v3(&copied_profile)?;
    let copied_profile_digest =
        domain_separated_digest("hsai.local-beta-profile.v3", &copied_profile)
            .map_err(CliError::internal)?;
    if copied_profile_digest != expected_profile_digest {
        return Err(CliError::operation(
            "BETA_PROFILE_DIGEST_MISMATCH",
            "copied profile differs from the embedded frozen profile",
        ));
    }
    let provenance: BetaRunProvenanceV1 = read_json_path(&run_root.join("run-provenance.json"))?;
    validate_recorded_provenance(&provenance).map_err(|error| {
        CliError::operation(error.code, "recorded beta provenance validation failed")
    })?;
    let provenance_digest =
        domain_separated_digest("hsai.local-beta-run-provenance.v1", &provenance)
            .map_err(CliError::internal)?;
    if provenance_digest != manifest.provenance_digest {
        return Err(CliError::operation(
            "BETA_PROVENANCE_DIGEST_MISMATCH",
            "run provenance digest differs from the manifest binding",
        ));
    }
    verify_beta_scenario_results(run_root, &manifest.scenario_results)?;
    let [recovery_check, authorization_check] = manifest.operational_checks.as_slice() else {
        return Err(CliError::operation(
            "BETA_OPERATIONAL_CHECK_MISMATCH",
            "beta manifest must contain exactly the two frozen operational checks",
        ));
    };
    verify_operational_check_binding(
        run_root,
        recovery_check,
        "sqlite-settlement-recovery",
        "operational-checks/sqlite-settlement-recovery/local-recovery-report.json",
        "operational-checks/sqlite-settlement-recovery/settlement-recovery.sqlite3",
    )?;
    let recovery_root = run_root.join("operational-checks/sqlite-settlement-recovery");
    let recovery_report: LocalRecoveryReportV1 =
        read_json_path(&recovery_root.join("local-recovery-report.json"))?;
    beta_runtime()?
        .block_on(verify_local_lost_receipt_recovery(
            &recovery_root,
            &recovery_report,
        ))
        .map_err(|error| {
            CliError::operation("BETA_OPERATIONAL_CHECK_FAILED", &error.to_string())
        })?;

    verify_operational_check_binding(
        run_root,
        authorization_check,
        "authorization-denial",
        "operational-checks/authorization-denial/authorization-denial-report.json",
        "operational-checks/authorization-denial/authorization-denial.sqlite3",
    )?;
    let authorization_root = run_root.join("operational-checks/authorization-denial");
    let authorization_report: LocalAuthorizationDenialReportV1 =
        read_json_path(&authorization_root.join("authorization-denial-report.json"))?;
    beta_runtime()?
        .block_on(verify_local_authorization_denial_acceptance(
            &authorization_root,
            &authorization_report,
        ))
        .map_err(|error| {
            CliError::operation("BETA_OPERATIONAL_CHECK_FAILED", &error.to_string())
        })?;
    verify_exported_tamper_artifact(run_root)
}

fn verify_operational_check_binding(
    run_root: &Path,
    check: &BetaOperationalCheckResult,
    expected_check: &str,
    expected_report_file: &str,
    expected_database_file: &str,
) -> Result<(), CliError> {
    if check.check != expected_check
        || check.status != "PASS"
        || check.report_file != expected_report_file
        || check.database_file != expected_database_file
    {
        return Err(CliError::operation(
            "BETA_OPERATIONAL_CHECK_MISMATCH",
            "operational check identity, status, or artifact path differs from the frozen contract",
        ));
    }
    if raw_file_digest(&run_root.join(&check.report_file))? != check.report_digest
        || raw_file_digest(&run_root.join(&check.database_file))? != check.database_digest
    {
        return Err(CliError::operation(
            "BETA_OPERATIONAL_CHECK_MISMATCH",
            "operational check artifact digest differs from the manifest binding",
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn verify_beta_scenario_results(
    run_root: &Path,
    scenario_results: &[BetaScenarioResult],
) -> Result<(), CliError> {
    if scenario_results.len() != BETA_SCENARIOS.len() {
        return Err(CliError::operation(
            "BETA_SCENARIO_MATRIX_MISMATCH",
            "manifest does not contain exactly the frozen scenario matrix",
        ));
    }
    let mut verified_bundle_count = 0_usize;
    for ((expected_name, scenario), result) in BETA_SCENARIOS.iter().zip(scenario_results) {
        if result.scenario != *expected_name || result.status != "PASS" {
            return Err(CliError::operation(
                "BETA_SCENARIO_MATRIX_MISMATCH",
                "manifest scenario order, identity, or status differs from the frozen matrix",
            ));
        }
        let scenario_root = run_root.join(expected_name);
        let report: ScenarioReport = read_json_path(&scenario_root.join("scenario-report.json"))?;
        validate_scenario_report(expected_name, &report)?;
        if result.truth != report.truth
            || result.finality != report.finality
            || result.execution_outcomes != report.execution_outcomes
            || result.mock_effect_count != report.mock_effect_count
            || result.reconciliation_status != report.reconciliation_status
        {
            return Err(CliError::operation(
                "BETA_SCENARIO_MATRIX_MISMATCH",
                "manifest scenario result differs from its validated report",
            ));
        }
        let expected_bundle_names: &[&str] =
            if *scenario == LocalScenario::DisputedFalseCounterexampleTrue {
                &[
                    "original-evidence-bundle.json",
                    "replacement-evidence-bundle.json",
                ]
            } else {
                &["evidence-bundle.json"]
            };
        if result.evidence_bundles.len() != expected_bundle_names.len() {
            return Err(CliError::operation(
                "BETA_SCENARIO_MATRIX_MISMATCH",
                "manifest bundle count differs from the frozen matrix",
            ));
        }
        for (bundle, expected_bundle_name) in
            result.evidence_bundles.iter().zip(expected_bundle_names)
        {
            let expected_relative_path = format!("{expected_name}/{expected_bundle_name}");
            if bundle.relative_path != expected_relative_path {
                return Err(CliError::operation(
                    "BETA_SCENARIO_MATRIX_MISMATCH",
                    "manifest bundle path differs from the frozen matrix",
                ));
            }
            let observed_digest =
                verify_bundle_from_disk(&run_root.join(&bundle.relative_path), &scenario_root)?;
            if observed_digest != bundle.bundle_digest {
                return Err(CliError::operation(
                    "BETA_SCENARIO_MATRIX_MISMATCH",
                    "disk-verified bundle digest differs from the manifest binding",
                ));
            }
            verified_bundle_count += 1;
        }
        if result
            .evidence_bundles
            .last()
            .map(|bundle| bundle.bundle_digest)
            != Some(report.evidence_bundle_digest)
        {
            return Err(CliError::operation(
                "BETA_SCENARIO_MATRIX_MISMATCH",
                "primary bundle digest differs from the scenario report binding",
            ));
        }
    }
    if verified_bundle_count != 8 {
        return Err(CliError::operation(
            "BETA_SCENARIO_MATRIX_MISMATCH",
            "beta run must contain exactly eight verified evidence bundles",
        ));
    }
    Ok(())
}

fn verify_exported_tamper_artifact(run_root: &Path) -> Result<(), CliError> {
    let tampered = run_root.join("tamper-check/evidence-bundle.json");
    match verify_bundle_from_disk(&tampered, &run_root.join("true")) {
        Err(error) if error.code == "VERIFICATION_FAILED" => Ok(()),
        Err(error) => Err(error),
        Ok(_) => Err(CliError::operation(
            "BETA_TAMPER_CHECK_FAILED",
            "retained tampered evidence was accepted",
        )),
    }
}

fn verify_bundle_from_disk(
    bundle_path: &Path,
    blob_root: &Path,
) -> Result<outcomes_domain::Sha256Digest, CliError> {
    let bundle: OutcomeEvidenceBundleV1 = read_json_path(bundle_path)?;
    let store = FileBlobStore::new(blob_root);
    let mut blobs = HashMap::new();
    for reference in &bundle.blobs {
        let bytes = store
            .get(reference)
            .map_err(|error| CliError::verification(&error.to_string()))?;
        blobs.insert(reference.digest, bytes);
    }
    verify_evidence_bundle(&bundle, &blobs)
        .map_err(|error| CliError::verification(&error.to_string()))?;
    Ok(bundle.bundle_digest)
}

fn run_exported_tamper_check(run_root: &Path) -> Result<(), CliError> {
    let source = run_root.join("true/evidence-bundle.json");
    let tamper_root = run_root.join("tamper-check");
    fs::create_dir(&tamper_root)
        .map_err(|error| CliError::input("ARTIFACT_WRITE_FAILED", &error.to_string()))?;
    let mut value: Value = read_json_path(&source)?;
    let byte_length = value["blobs"][0]["byte_length"]
        .as_u64()
        .ok_or_else(|| CliError::internal("true bundle has no blob byte length"))?;
    value["blobs"][0]["byte_length"] = Value::from(byte_length.saturating_add(1));
    let tampered = tamper_root.join("evidence-bundle.json");
    fs::write(
        &tampered,
        serde_json::to_vec_pretty(&value).map_err(CliError::internal)?,
    )
    .map_err(|error| CliError::input("ARTIFACT_WRITE_FAILED", &error.to_string()))?;
    match verify_bundle_from_disk(&tampered, &run_root.join("true")) {
        Err(error) if error.code == "VERIFICATION_FAILED" => Ok(()),
        Err(error) => Err(error),
        Ok(_) => Err(CliError::operation(
            "BETA_TAMPER_CHECK_FAILED",
            "tampered exported evidence was accepted",
        )),
    }
}

fn read_json_path<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, CliError> {
    let bytes =
        fs::read(path).map_err(|error| CliError::input("FILE_READ_FAILED", &error.to_string()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| CliError::input("INVALID_JSON", &error.to_string()))
}

fn run_corpus(args: CorpusArgs) -> Result<Value, CliError> {
    match args.command {
        CorpusCommand::Profiles => {
            serde_json::from_str(CORPUS_PROFILES).map_err(CliError::internal)
        }
        CorpusCommand::MaterializeLocal {
            profile,
            checkout_root,
            materialization_root,
            expected_commit,
            observed_commit,
            max_file_count,
            max_file_bytes,
            max_total_bytes,
        } => {
            let catalog = bundled_corpus_profiles()
                .map_err(|error| CliError::validation(&error.to_string()))?;
            let manifest = materialize_pinned_local_root(
                &catalog,
                &LocalMaterializationRequest {
                    profile_id: &profile,
                    checkout_root: &checkout_root,
                    materialization_root: &materialization_root,
                    expected_commit: &expected_commit,
                    observed_checkout_commit: &observed_commit,
                    limits: LocalMaterializationLimits {
                        max_file_count,
                        max_file_bytes,
                        max_total_bytes,
                    },
                },
            )
            .map_err(|error| CliError::validation(&error.to_string()))?;
            serde_json::to_value(manifest).map_err(CliError::internal)
        }
    }
}

fn run_settlement(args: SettlementArgs) -> Result<Value, CliError> {
    match args.command {
        SettlementCommand::Preflight(inputs) => {
            let (_, _, readiness) = settlement_preflight(&inputs)?;
            serde_json::to_value(readiness).map_err(CliError::internal)
        }
        SettlementCommand::BuildIntent { inputs, expires_at } => {
            let (binding, resolution, readiness) = settlement_preflight(&inputs)?;
            let expires_at: UtcTimestamp =
                expires_at
                    .parse()
                    .map_err(|error: outcomes_domain::TimestampError| {
                        CliError::validation(&error.to_string())
                    })?;
            let intent = build_intent(&binding, &resolution, &readiness, expires_at)
                .map_err(|error| CliError::validation(&error.to_string()))?;
            serde_json::to_value(intent).map_err(CliError::internal)
        }
        SettlementCommand::Publish { intent, dry_run } => {
            let intent: UnsignedSettlementIntentV1 = read_json(&intent)?;
            if intent.network == SettlementNetwork::HyperliquidMainnetReadonly {
                return Err(CliError::operation(
                    "MAINNET_MUTATION_DISABLED",
                    "mainnet mutation is structurally disabled",
                ));
            }
            if !dry_run {
                return Err(CliError::operation(
                    "DRY_RUN_REQUIRED",
                    "this build has no mutation-capable publisher; --dry-run is required",
                ));
            }
            Ok(json!({
                "intent_id": intent.intent_id,
                "network": intent.network,
                "dry_run": true,
                "external_effect": "none",
                "publisher_status": if intent.network == SettlementNetwork::Mock {
                    "MOCK_DRY_RUN_ONLY"
                } else {
                    "UNSUPPORTED_UNVERIFIED"
                },
            }))
        }
        SettlementCommand::Reconcile { reconciliation } => {
            let result: ReconciliationResultV1 = read_json(&reconciliation)?;
            Ok(json!({
                "status": result.status,
                "settlement_lifecycle": result.settlement_lifecycle,
                "binding_digest": result.binding_digest,
                "final_resolution_digest": result.final_resolution_digest,
                "exact_reason": result.exact_reason,
                "external_effect": "none; retained reconciliation inspection only",
            }))
        }
    }
}

fn settlement_preflight(
    inputs: &SettlementInputArgs,
) -> Result<
    (
        MarketBindingV1,
        ResolutionRecordV1,
        outcomes_settlement::SettlementPreflight,
    ),
    CliError,
> {
    let binding: MarketBindingV1 = read_json(&inputs.binding)?;
    let resolution: ResolutionRecordV1 = read_json(&inputs.resolution)?;
    let capabilities: CapabilityMatrix = read_json(&inputs.capabilities)?;
    if capabilities.network != binding.network || capabilities.backend != binding.backend {
        return Err(CliError::validation(
            "capability snapshot does not match binding backend and network",
        ));
    }
    let raw = fs::read(&inputs.metadata)
        .map_err(|error| CliError::input("FILE_READ_FAILED", &error.to_string()))?;
    let market = Hip4ReadOnlyBackend {
        network: binding.network,
        info_url: String::new(),
    }
    .parse_recorded(&raw)
    .map_err(|error| CliError::validation(&error.to_string()))?;
    if market.market_id != binding.external_market_id {
        return Err(CliError::validation(
            "network-scoped outcome ID does not match binding",
        ));
    }
    let readiness = preflight(&binding, &resolution, market.metadata_digest, &capabilities);
    Ok((binding, resolution, readiness))
}

fn run_hip4(args: Hip4Args) -> Result<Value, CliError> {
    match args.command {
        Hip4Command::Discover { network, fixture } => {
            let network = match network {
                Hip4NetworkArg::Mock => SettlementNetwork::Mock,
                Hip4NetworkArg::Testnet => SettlementNetwork::HyperliquidTestnet,
                Hip4NetworkArg::MainnetReadonly => SettlementNetwork::HyperliquidMainnetReadonly,
            };
            let raw = fs::read(fixture)
                .map_err(|error| CliError::input("FILE_READ_FAILED", &error.to_string()))?;
            let backend = Hip4ReadOnlyBackend {
                network,
                info_url: String::new(),
            };
            let market = backend
                .parse_recorded(&raw)
                .map_err(|error| CliError::validation(&error.to_string()))?;
            serde_json::to_value(market).map_err(CliError::internal)
        }
        Hip4Command::Encode {
            outcome_id,
            side_index,
        } => serde_json::to_value(
            encode_asset_ids(outcome_id, side_index)
                .map_err(|error| CliError::validation(&error.to_string()))?,
        )
        .map_err(CliError::internal),
        Hip4Command::Decode { asset_id } => serde_json::to_value(
            decode_asset_id(&asset_id).map_err(|error| CliError::validation(&error.to_string()))?,
        )
        .map_err(CliError::internal),
    }
}

fn run_adapter(args: AdapterArgs) -> Result<Value, CliError> {
    match args.command {
        AdapterCommand::Capabilities { adapter } => {
            let mut matrix: Value =
                serde_json::from_str(CAPABILITIES).map_err(CliError::internal)?;
            if let Some(adapter) = adapter {
                let entries = matrix["adapters"]
                    .as_array()
                    .ok_or_else(|| CliError::internal("capability matrix has no adapters array"))?;
                let selected = entries
                    .iter()
                    .find(|entry| entry["id"] == adapter)
                    .cloned()
                    .ok_or_else(|| CliError::input("UNKNOWN_ADAPTER", "adapter ID not found"))?;
                matrix = selected;
            }
            Ok(matrix)
        }
    }
}

fn run_claim(args: ClaimArgs) -> Result<Value, CliError> {
    match args.command {
        ClaimCommand::Validate { claim } => {
            let draft: ClaimSpecDraftV1 = read_json(&claim)?;
            draft
                .validate()
                .map_err(|error| CliError::validation(&error.to_string()))?;
            Ok(json!({"valid": true, "schema_version": draft.schema_version}))
        }
        ClaimCommand::Freeze { claim } => {
            let draft: ClaimSpecDraftV1 = read_json(&claim)?;
            let frozen = draft
                .freeze()
                .map_err(|error| CliError::validation(&error.to_string()))?;
            serde_json::to_value(frozen).map_err(CliError::internal)
        }
        ClaimCommand::Show { claim } => {
            let frozen: ClaimSpecV1 = read_json(&claim)?;
            frozen
                .verify_identity()
                .map_err(|error| CliError::verification(&error.to_string()))?;
            let digest = frozen
                .claim_spec_digest()
                .map_err(|error| CliError::verification(&error.to_string()))?;
            Ok(json!({
                "valid": true,
                "claim_id": frozen.claim_id,
                "claim_spec_digest": digest,
            }))
        }
    }
}

fn run_evidence(args: EvidenceArgs) -> Result<Value, CliError> {
    match args.command {
        EvidenceCommand::Verify { bundle, blob_root } => {
            let bundle: OutcomeEvidenceBundleV1 = read_json(&bundle)?;
            let store = FileBlobStore::new(blob_root);
            let mut blobs = HashMap::new();
            for reference in &bundle.blobs {
                let bytes = store
                    .get(reference)
                    .map_err(|error| CliError::verification(&error.to_string()))?;
                blobs.insert(reference.digest, bytes);
            }
            verify_evidence_bundle(&bundle, &blobs)
                .map_err(|error| CliError::verification(&error.to_string()))?;
            Ok(json!({"valid": true, "bundle_digest": bundle.bundle_digest}))
        }
    }
}

fn run_demo(args: DemoArgs) -> Result<Value, CliError> {
    match args.command {
        DemoCommand::ComputationalMarket {
            scenario,
            artifact_root,
            json: _,
        } => {
            let scenario = match scenario {
                ScenarioArg::True => LocalScenario::True,
                ScenarioArg::InvalidThenValid => LocalScenario::InvalidThenValid,
                ScenarioArg::DisputedFalseCounterexampleTrue => {
                    LocalScenario::DisputedFalseCounterexampleTrue
                }
                ScenarioArg::False => LocalScenario::False,
                ScenarioArg::IncompleteObservation => LocalScenario::IncompleteObservation,
                ScenarioArg::DivergentReplays => LocalScenario::DivergentReplays,
                ScenarioArg::RunnerError => LocalScenario::RunnerError,
            };
            let report = run_local_scenario(scenario, &artifact_root)
                .map_err(|error| CliError::internal(error.to_string()))?;
            serde_json::to_value(report).map_err(CliError::internal)
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T, CliError> {
    let bytes =
        fs::read(path).map_err(|error| CliError::input("FILE_READ_FAILED", &error.to_string()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| CliError::input("INVALID_JSON", &error.to_string()))
}

#[derive(Debug)]
struct CliError {
    exit_code: u8,
    code: String,
    message: String,
}

impl CliError {
    fn input(code: &'static str, message: &str) -> Self {
        Self {
            exit_code: 2,
            code: code.to_owned(),
            message: message.to_owned(),
        }
    }

    fn validation(message: &str) -> Self {
        Self {
            exit_code: 3,
            code: "VALIDATION_FAILED".to_owned(),
            message: message.to_owned(),
        }
    }

    fn verification(message: &str) -> Self {
        Self {
            exit_code: 4,
            code: "VERIFICATION_FAILED".to_owned(),
            message: message.to_owned(),
        }
    }

    fn operation(code: impl Into<String>, message: &str) -> Self {
        Self {
            exit_code: 6,
            code: code.into(),
            message: message.to_owned(),
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn internal(message: impl ToString) -> Self {
        Self {
            exit_code: 5,
            code: "INTERNAL_ERROR".to_owned(),
            message: message.to_string(),
        }
    }
}

fn fail(code: u8, error_code: &str, message: &str) -> ExitCode {
    let output = json!({"error_code": error_code, "message": message});
    eprintln!("{output}");
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use beta_provenance::{
        BetaInputProvenanceV1, BetaRunProvenanceV1, BetaSourceProvenanceV1, BetaToolProvenanceV1,
        ProvenanceError,
    };
    use outcomes_domain::Sha256Digest;

    use super::*;

    #[test]
    fn market_compare_is_a_read_only_local_service_command() {
        let parsed = Cli::try_parse_from([
            "hyperai",
            "market",
            "compare",
            "--home",
            "/tmp/hyperai-home",
        ]);
        assert!(parsed.is_ok(), "{parsed:?}");
        let Ok(parsed) = parsed else {
            return;
        };
        assert!(matches!(&parsed.command, RootCommand::Market(_)));
        let RootCommand::Market(market) = parsed.command else {
            return;
        };
        assert!(matches!(&market.command, MarketCommand::Compare(_)));
        let MarketCommand::Compare(args) = market.command else {
            return;
        };
        assert_eq!(args.home.as_deref(), Some(Path::new("/tmp/hyperai-home")));
    }

    #[test]
    fn market_history_cli_defaults_and_bounds_its_page_size() -> Result<(), Box<dyn Error>> {
        let parsed = Cli::try_parse_from(["hyperai", "market", "history"]);
        assert!(parsed.is_ok(), "{parsed:?}");
        let parsed = parsed?;
        let RootCommand::Market(market) = parsed.command else {
            return Err("market history did not parse as a Market command".into());
        };
        let MarketCommand::History(args) = market.command else {
            return Err("history did not parse as a history subcommand".into());
        };
        assert_eq!(args.limit, 20);

        let result = run_market_history(&MarketHistoryArgs {
            limit: 51,
            cursor: None,
            home: None,
        });
        assert!(matches!(
            result,
            Err(CliError { code, .. }) if code == "MARKET_HISTORY_INVALID_QUERY"
        ));
        Ok(())
    }

    #[test]
    fn market_history_cli_recomputes_typed_page_digest() -> Result<(), Box<dyn Error>> {
        let page = hyperai_market::local_market_history_page_v1(Vec::new(), None)?;
        let valid = serde_json::to_value(&page)?;
        let decoded = decode_market_history_page(valid.clone())
            .map_err(|error| std::io::Error::other(error.message))?;
        assert_eq!(decoded, valid);

        let mut tampered = valid;
        tampered["body"]["claim_ceiling"] = json!("a different claim ceiling");
        let result = decode_market_history_page(tampered);
        assert!(matches!(
            result,
            Err(CliError { code, .. }) if code == "VERIFICATION_FAILED"
        ));
        Ok(())
    }

    #[test]
    fn trade_cli_rejects_assumptions_outside_local_bounds() {
        let valid = Cli::try_parse_from([
            "hyperai",
            "trade",
            "scenario",
            "--primary-instrument-id",
            "a100-us-east",
            "--comparison-instrument-id",
            "a100-us-west",
            "--price-shock-bps=-5000",
            "--availability-delta-units=10",
        ]);
        assert!(valid.is_ok(), "{valid:?}");

        let unbounded_price = Cli::try_parse_from([
            "hyperai",
            "trade",
            "scenario",
            "--primary-instrument-id",
            "a100-us-east",
            "--comparison-instrument-id",
            "a100-us-west",
            "--price-shock-bps=5001",
        ]);
        assert!(unbounded_price.is_err());

        let unbounded_capacity = Cli::try_parse_from([
            "hyperai",
            "trade",
            "scenario",
            "--primary-instrument-id",
            "a100-us-east",
            "--comparison-instrument-id",
            "a100-us-west",
            "--availability-delta-units=-11",
        ]);
        assert!(unbounded_capacity.is_err());
    }

    struct FakeProvenanceProvider {
        result: Result<BetaRunProvenanceV1, ProvenanceError>,
    }

    impl BetaProvenanceProvider for FakeProvenanceProvider {
        fn collect(&self, _repository_root: &Path) -> Result<BetaRunProvenanceV1, ProvenanceError> {
            self.result.clone()
        }
    }

    fn test_provenance() -> BetaRunProvenanceV1 {
        BetaRunProvenanceV1 {
            schema_version: "hsai.local-beta-run-provenance.v1".to_owned(),
            source: BetaSourceProvenanceV1 {
                git_commit: "0123456789abcdef0123456789abcdef01234567".to_owned(),
                tree_state: "CLEAN".to_owned(),
            },
            inputs: BetaInputProvenanceV1 {
                cargo_lock_sha256: Sha256Digest::from_bytes([1; 32]),
                pnpm_lock_sha256: Sha256Digest::from_bytes([2; 32]),
                rust_toolchain_sha256: Sha256Digest::from_bytes([3; 32]),
                package_json_sha256: Sha256Digest::from_bytes([4; 32]),
            },
            tools: BetaToolProvenanceV1 {
                rustc: "1.93.1".to_owned(),
                cargo: "1.93.1".to_owned(),
                pnpm: "11.7.0".to_owned(),
            },
        }
    }

    fn beta_rehearsal(artifact_root: &Path, run_id: &str) -> Result<Value, CliError> {
        run_beta_rehearsal_with_provider(
            artifact_root,
            run_id,
            &FakeProvenanceProvider {
                result: Ok(test_provenance()),
            },
        )
    }

    fn beta_rehearsal_unsealed(artifact_root: &Path, run_id: &str) -> Result<Value, CliError> {
        run_beta_rehearsal_with_provider_and_seal(
            artifact_root,
            run_id,
            &FakeProvenanceProvider {
                result: Ok(test_provenance()),
            },
            false,
        )
    }

    fn intent(
        network: SettlementNetwork,
    ) -> Result<UnsignedSettlementIntentV1, Box<dyn std::error::Error>> {
        Ok(UnsignedSettlementIntentV1 {
            schema_version: "hsai.unsigned-settlement-intent.v1".to_owned(),
            intent_id: "intent-cli-test".to_owned(),
            backend: "hyperliquid-hip4".to_owned(),
            network,
            binding_digest: Sha256Digest::from_bytes([1; 32]),
            final_resolution_digest: Sha256Digest::from_bytes([2; 32]),
            action_type: "ATTESTED_RESOLUTION".to_owned(),
            canonical_payload: "{}".to_owned(),
            payload_digest: Sha256Digest::from_bytes([3; 32]),
            idempotency_key: "settlement-cli-test".to_owned(),
            expires_at: "2026-07-12T00:00:00Z".parse()?,
        })
    }

    fn write_intent(
        network: SettlementNetwork,
    ) -> Result<(tempfile::TempDir, PathBuf), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("intent.json");
        fs::write(&path, serde_json::to_vec(&intent(network)?)?)?;
        Ok((directory, path))
    }

    #[test]
    fn contract_cli_mainnet_publish_refuses_even_dry_run() -> Result<(), Box<dyn std::error::Error>>
    {
        let (_directory, path) = write_intent(SettlementNetwork::HyperliquidMainnetReadonly)?;
        let error = run(Cli {
            command: RootCommand::Outcomes(OutcomesArgs {
                command: OutcomesCommand::Settlement(SettlementArgs {
                    command: SettlementCommand::Publish {
                        intent: path,
                        dry_run: true,
                    },
                }),
            }),
        })
        .err()
        .ok_or("mainnet publish must fail")?;
        assert_eq!(error.code, "MAINNET_MUTATION_DISABLED");
        assert_eq!(error.exit_code, 6);
        Ok(())
    }

    #[test]
    fn contract_cli_publish_requires_dry_run_and_never_effects()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_directory, path) = write_intent(SettlementNetwork::Mock)?;
        let denied = run(Cli {
            command: RootCommand::Outcomes(OutcomesArgs {
                command: OutcomesCommand::Settlement(SettlementArgs {
                    command: SettlementCommand::Publish {
                        intent: path.clone(),
                        dry_run: false,
                    },
                }),
            }),
        })
        .err()
        .ok_or("non-dry-run publish must fail")?;
        assert_eq!(denied.code, "DRY_RUN_REQUIRED");

        let output = run(Cli {
            command: RootCommand::Outcomes(OutcomesArgs {
                command: OutcomesCommand::Settlement(SettlementArgs {
                    command: SettlementCommand::Publish {
                        intent: path,
                        dry_run: true,
                    },
                }),
            }),
        })
        .map_err(|error| std::io::Error::other(error.message))?;
        assert_eq!(output["external_effect"], "none");
        assert_eq!(output["dry_run"], true);
        Ok(())
    }

    #[test]
    fn contract_cli_local_corpus_materialization_is_pinned_and_non_executing()
    -> Result<(), Box<dyn std::error::Error>> {
        let checkout_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("fixtures/corpora/local-materialization")
            .canonicalize()?;
        let commit = "4c5744f55a31a58e03d6118c9dbb1c18e5bbdfa9".to_owned();
        let output = run(Cli {
            command: RootCommand::Outcomes(OutcomesArgs {
                command: OutcomesCommand::Corpus(CorpusArgs {
                    command: CorpusCommand::MaterializeLocal {
                        profile: "nyu-ctf-bench".to_owned(),
                        checkout_root,
                        materialization_root: "development".to_owned(),
                        expected_commit: commit.clone(),
                        observed_commit: commit,
                        max_file_count: 10,
                        max_file_bytes: 1_024,
                        max_total_bytes: 4_096,
                    },
                }),
            }),
        })
        .map_err(|error| std::io::Error::other(error.message))?;
        assert_eq!(
            output["schema_version"],
            "hsai.local-corpus-materialization.v3"
        );
        assert_eq!(output["executed_files"], false);
        assert_ne!(output["settlement_readiness"], "SETTLEMENT_READY");
        assert!(
            output["files"]
                .as_array()
                .is_some_and(|files| !files.is_empty())
        );
        assert!(
            output["special_objects"]
                .as_array()
                .is_some_and(|objects| !objects.is_empty())
        );
        assert!(
            output["readiness_blockers"]
                .as_array()
                .is_some_and(|blockers| !blockers.is_empty())
        );
        Ok(())
    }

    #[test]
    fn contract_cli_invalid_then_valid_reports_all_execution_outcomes()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let output = run(Cli {
            command: RootCommand::Outcomes(OutcomesArgs {
                command: OutcomesCommand::Demo(DemoArgs {
                    command: DemoCommand::ComputationalMarket {
                        scenario: ScenarioArg::InvalidThenValid,
                        artifact_root: directory.path().to_path_buf(),
                        json: true,
                    },
                }),
            }),
        })
        .map_err(|error| std::io::Error::other(error.message))?;
        assert_eq!(
            output["execution_outcomes"],
            json!(["FAIL", "PASS", "PASS"])
        );
        assert_eq!(output["truth"], "TRUE");
        assert_eq!(output["mock_effect_count"], 1);
        Ok(())
    }

    #[test]
    fn contract_cli_disputed_false_counterexample_reports_superseding_true()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let output = run(Cli {
            command: RootCommand::Outcomes(OutcomesArgs {
                command: OutcomesCommand::Demo(DemoArgs {
                    command: DemoCommand::ComputationalMarket {
                        scenario: ScenarioArg::DisputedFalseCounterexampleTrue,
                        artifact_root: directory.path().to_path_buf(),
                        json: true,
                    },
                }),
            }),
        })
        .map_err(|error| std::io::Error::other(error.message))?;
        assert_eq!(output["execution_outcomes"], json!(["PASS", "PASS"]));
        assert_eq!(output["truth"], "TRUE");
        assert_eq!(output["finality"], "FINAL");
        assert_eq!(output["mock_effect_count"], 1);
        Ok(())
    }

    #[test]
    fn contract_cli_unresolved_scenarios_refuse_settlement_eligibility()
    -> Result<(), Box<dyn std::error::Error>> {
        for (scenario, outcomes) in [
            (ScenarioArg::IncompleteObservation, json!([])),
            (ScenarioArg::DivergentReplays, json!(["PASS", "FAIL"])),
        ] {
            let directory = tempfile::tempdir()?;
            let output = run(Cli {
                command: RootCommand::Outcomes(OutcomesArgs {
                    command: OutcomesCommand::Demo(DemoArgs {
                        command: DemoCommand::ComputationalMarket {
                            scenario,
                            artifact_root: directory.path().to_path_buf(),
                            json: true,
                        },
                    }),
                }),
            })
            .map_err(|error| std::io::Error::other(error.message))?;
            assert_eq!(output["execution_outcomes"], outcomes);
            assert_eq!(output["truth"], "UNRESOLVED");
            assert_eq!(output["finality"], "NONE");
            assert_eq!(output["mock_effect_count"], 0);
            assert_eq!(
                output["settlement_eligibility"]["blockers"],
                json!(["RESOLUTION_REQUIRED", "FINAL_RESOLUTION_REQUIRED"])
            );
            assert_eq!(
                output["settlement_eligibility"]["eligible_for_preflight"],
                false
            );
        }
        Ok(())
    }

    #[test]
    fn contract_beta_rehearsal_verifies_matrix_bundles_and_tamper_rejection()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let output = beta_rehearsal(directory.path(), "beta-001")
            .map_err(|error| std::io::Error::other(error.message))?;
        assert_eq!(output["status"], "PASS");
        assert_eq!(output["external_effect"], "mock only; no network writes");
        assert_eq!(output["tamper_check"]["status"], "PASS");
        let scenarios = output["scenario_results"]
            .as_array()
            .ok_or("scenario results missing")?;
        assert_eq!(scenarios.len(), 7);
        assert_eq!(
            scenarios
                .iter()
                .map(|value| value["mock_effect_count"].as_u64().unwrap_or_default())
                .collect::<Vec<_>>(),
            vec![1, 0, 1, 1, 0, 0, 0]
        );
        assert_eq!(
            scenarios[3]["evidence_bundles"].as_array().map(Vec::len),
            Some(2)
        );
        let run_root = directory.path().join("beta-001");
        assert!(run_root.join("beta-run-manifest.json").is_file());
        let copied_profile: LocalBetaProfileV2 =
            read_json_path(&run_root.join("local-beta-profile.json"))
                .map_err(|error| std::io::Error::other(error.message))?;
        let copied_profile_digest =
            domain_separated_digest("hsai.local-beta-profile.v3", &copied_profile)?;
        assert_eq!(
            output["profile_digest"],
            serde_json::to_value(copied_profile_digest)?
        );
        assert_eq!(output["profile_file"], "local-beta-profile.json");
        assert_eq!(output["schema_version"], "hsai.local-beta-run-manifest.v4");
        assert_eq!(
            output["operational_checks"].as_array().map(Vec::len),
            Some(2)
        );
        assert_eq!(
            output["operational_checks"][0]["check"],
            "sqlite-settlement-recovery"
        );
        assert_eq!(
            output["operational_checks"][1]["check"],
            "authorization-denial"
        );
        assert!(
            run_root
                .join("operational-checks/sqlite-settlement-recovery/settlement-recovery.sqlite3")
                .is_file()
        );
        assert!(
            run_root
                .join("operational-checks/authorization-denial/authorization-denial.sqlite3")
                .is_file()
        );
        assert_eq!(output["provenance_file"], "run-provenance.json");
        let provenance: BetaRunProvenanceV1 = read_json_path(&run_root.join("run-provenance.json"))
            .map_err(|error| std::io::Error::other(error.message))?;
        let provenance_digest =
            domain_separated_digest("hsai.local-beta-run-provenance.v1", &provenance)?;
        assert_eq!(
            output["provenance_digest"],
            serde_json::to_value(provenance_digest)?
        );
        let serialized = serde_json::to_string(&provenance)?;
        for forbidden in [
            "cwd",
            "hostname",
            "username",
            "environment",
            "secret",
            "/Users/",
        ] {
            assert!(!serialized.contains(forbidden));
        }
        assert!(run_root.join("tamper-check/evidence-bundle.json").is_file());
        assert!(run_root.join("beta-artifact-inventory.json").is_file());
        verify_beta_run(directory.path(), "beta-001")
            .map_err(|error| std::io::Error::other(error.message))?;
        for scenario in scenarios {
            for bundle in scenario["evidence_bundles"]
                .as_array()
                .ok_or("bundle results missing")?
            {
                assert!(
                    run_root
                        .join(bundle["relative_path"].as_str().ok_or("path")?)
                        .is_file()
                );
            }
        }
        Ok(())
    }

    #[test]
    fn contract_beta_finalize_then_read_only_verify_and_collision()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        beta_rehearsal_unsealed(directory.path(), "sealed-001")
            .map_err(|error| std::io::Error::other(error.message))?;
        let finalized = finalize_beta_run(directory.path(), "sealed-001")
            .map_err(|error| std::io::Error::other(error.message))?;
        assert_eq!(
            finalized["schema_version"],
            "hsai.local-beta-artifact-inventory.v1"
        );
        let inventory_path = directory
            .path()
            .join("sealed-001/beta-artifact-inventory.json");
        let before = fs::read(&inventory_path)?;
        let verified = verify_beta_run(directory.path(), "sealed-001")
            .map_err(|error| std::io::Error::other(error.message))?;
        assert_eq!(finalized, verified);
        assert_eq!(before, fs::read(&inventory_path)?);
        let collision = finalize_beta_run(directory.path(), "sealed-001")
            .err()
            .ok_or("second finalization must fail")?;
        assert_eq!(collision.code, "BETA_ALREADY_FINALIZED");
        Ok(())
    }

    #[test]
    fn contract_beta_semantics_reject_bundle_mutation_before_seal()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        beta_rehearsal_unsealed(directory.path(), "tampered-001")
            .map_err(|error| std::io::Error::other(error.message))?;
        let bundle_path = directory
            .path()
            .join("tampered-001/true/evidence-bundle.json");
        let mut bundle: Value = serde_json::from_slice(&fs::read(&bundle_path)?)?;
        bundle["schema_version"] = json!("hsai.outcome-evidence-bundle.tampered");
        fs::write(bundle_path, serde_json::to_vec(&bundle)?)?;
        assert!(finalize_beta_run(directory.path(), "tampered-001").is_err());
        assert!(
            !directory
                .path()
                .join("tampered-001/beta-artifact-inventory.json")
                .exists()
        );
        Ok(())
    }

    #[test]
    fn contract_beta_verify_rejects_legacy_manifest_with_stable_code()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let run_root = directory.path().join("legacy-001");
        fs::create_dir(&run_root)?;
        fs::write(
            run_root.join("beta-run-manifest.json"),
            serde_json::to_vec(&json!({
                "schema_version": "hsai.local-beta-run-manifest.v1",
                "run_id": "legacy-001",
                "status": "PASS"
            }))?,
        )?;
        let error = verify_beta_run(directory.path(), "legacy-001")
            .err()
            .ok_or("legacy manifest must fail")?;
        assert_eq!(error.code, "BETA_MANIFEST_UNSUPPORTED_SCHEMA");
        Ok(())
    }

    #[test]
    fn contract_beta_v2_legacy_profile_run_remains_verifiable()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let run_id = "legacy-v2-sealed";
        beta_rehearsal_unsealed(directory.path(), run_id)
            .map_err(|error| std::io::Error::other(error.message))?;
        let run_root = directory.path().join(run_id);
        fs::remove_dir_all(run_root.join("operational-checks"))?;
        let legacy_profile: LocalBetaProfile = serde_json::from_str(LEGACY_LOCAL_BETA_PROFILE)?;
        let legacy_profile_digest =
            domain_separated_digest("hsai.local-beta-profile.v1", &legacy_profile)?;
        fs::write(
            run_root.join("local-beta-profile.json"),
            LEGACY_LOCAL_BETA_PROFILE,
        )?;
        let mut manifest: Value =
            serde_json::from_slice(&fs::read(run_root.join("beta-run-manifest.json"))?)?;
        manifest["schema_version"] = json!("hsai.local-beta-run-manifest.v2");
        manifest["profile_schema_version"] = json!(legacy_profile.schema_version);
        manifest["profile_id"] = json!(legacy_profile.profile_id);
        manifest["profile_digest"] = serde_json::to_value(legacy_profile_digest)?;
        manifest["claim_ceiling"] = json!(legacy_profile.claim_ceiling);
        manifest
            .as_object_mut()
            .ok_or("manifest must be an object")?
            .remove("operational_checks");
        fs::write(
            run_root.join("beta-run-manifest.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        finalize_beta_run(directory.path(), run_id)
            .map_err(|error| std::io::Error::other(error.message))?;
        verify_beta_run(directory.path(), run_id)
            .map_err(|error| std::io::Error::other(error.message))?;
        Ok(())
    }

    #[test]
    fn contract_beta_operational_report_and_database_tamper_fail_closed()
    -> Result<(), Box<dyn std::error::Error>> {
        for (run_id, relative_path) in [
            (
                "recovery-report-tamper",
                "operational-checks/sqlite-settlement-recovery/local-recovery-report.json",
            ),
            (
                "recovery-database-tamper",
                "operational-checks/sqlite-settlement-recovery/settlement-recovery.sqlite3",
            ),
            (
                "authorization-report-tamper",
                "operational-checks/authorization-denial/authorization-denial-report.json",
            ),
            (
                "authorization-database-tamper",
                "operational-checks/authorization-denial/authorization-denial.sqlite3",
            ),
        ] {
            let directory = tempfile::tempdir()?;
            beta_rehearsal(directory.path(), run_id)
                .map_err(|error| std::io::Error::other(error.message))?;
            let path = directory.path().join(run_id).join(relative_path);
            let mut bytes = fs::read(&path)?;
            bytes.push(b'!');
            fs::write(path, bytes)?;
            assert!(verify_beta_run(directory.path(), run_id).is_err());
        }

        let directory = tempfile::tempdir()?;
        beta_rehearsal(directory.path(), "authorization-report-delete")
            .map_err(|error| std::io::Error::other(error.message))?;
        fs::remove_file(
            directory
                .path()
                .join("authorization-report-delete/operational-checks/authorization-denial/authorization-denial-report.json"),
        )?;
        assert!(verify_beta_run(directory.path(), "authorization-report-delete").is_err());
        Ok(())
    }

    #[test]
    fn contract_beta_v4_rejects_operational_check_order_removal_and_addition()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let run_id = "operational-matrix-drift";
        beta_rehearsal_unsealed(directory.path(), run_id)
            .map_err(|error| std::io::Error::other(error.message))?;
        let manifest_path = directory.path().join(run_id).join("beta-run-manifest.json");
        let original: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
        for mutation in ["order", "remove", "add"] {
            let mut changed = original.clone();
            let checks = changed["operational_checks"]
                .as_array_mut()
                .ok_or("operational checks missing")?;
            match mutation {
                "order" => checks.swap(0, 1),
                "remove" => {
                    checks.pop();
                }
                "add" => checks.push(checks[0].clone()),
                _ => unreachable!(),
            }
            fs::write(&manifest_path, serde_json::to_vec_pretty(&changed)?)?;
            assert!(
                verify_beta_run_semantics(&directory.path().join(run_id), run_id).is_err(),
                "{mutation} mutation must fail"
            );
        }
        Ok(())
    }

    #[test]
    fn contract_beta_verify_rejects_post_seal_addition_mutation_and_deletion()
    -> Result<(), Box<dyn std::error::Error>> {
        for (run_id, change) in [
            ("changed-add", "add"),
            ("changed-mutate", "mutate"),
            ("changed-delete", "delete"),
        ] {
            let directory = tempfile::tempdir()?;
            beta_rehearsal(directory.path(), run_id)
                .map_err(|error| std::io::Error::other(error.message))?;
            let run_root = directory.path().join(run_id);
            match change {
                "add" => fs::write(run_root.join("unlisted-artifact"), b"unexpected")?,
                "mutate" => fs::write(run_root.join("local-beta-profile.json"), b"{}")?,
                "delete" => fs::remove_file(run_root.join("run-provenance.json"))?,
                _ => unreachable!(),
            }
            assert!(verify_beta_run(directory.path(), run_id).is_err());
        }
        Ok(())
    }

    #[test]
    fn contract_beta_rehearsal_rejects_unsafe_id_and_existing_run()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let unsafe_error = beta_rehearsal(directory.path(), "../escape")
            .err()
            .ok_or("unsafe ID must fail")?;
        assert_eq!(unsafe_error.code, "UNSAFE_BETA_RUN_ID");
        fs::create_dir(directory.path().join("existing"))?;
        let collision = beta_rehearsal(directory.path(), "existing")
            .err()
            .ok_or("existing run must fail")?;
        assert_eq!(collision.code, "BETA_RUN_COLLISION");

        let dirty_root = tempfile::tempdir()?;
        let dirty = run_beta_rehearsal_with_provider(
            dirty_root.path(),
            "dirty-run",
            &FakeProvenanceProvider {
                result: Err(ProvenanceError {
                    code: "BETA_SOURCE_DIRTY",
                }),
            },
        )
        .err()
        .ok_or("dirty source must fail")?;
        assert_eq!(dirty.code, "BETA_SOURCE_DIRTY");
        assert!(!dirty_root.path().join("dirty-run").exists());
        Ok(())
    }

    #[test]
    fn contract_beta_profile_rejects_live_writes_and_scenario_drift()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut profile: LocalBetaProfileV2 = serde_json::from_str(LOCAL_BETA_PROFILE)?;
        profile.profile_id = "unapproved-local-profile".to_owned();
        let identity_error = validate_beta_profile_v3(&profile)
            .err()
            .ok_or("profile ID drift must fail")?;
        assert_eq!(identity_error.code, "UNSAFE_LOCAL_BETA_PROFILE");
        profile.profile_id = "hermetic-trusted-operator-v3".to_owned();
        profile.live_writes = true;
        let live_error = validate_beta_profile_v3(&profile)
            .err()
            .ok_or("live writes must fail")?;
        assert_eq!(live_error.code, "UNSAFE_LOCAL_BETA_PROFILE");
        profile.live_writes = false;
        profile.scenarios.swap(0, 1);
        let drift_error = validate_beta_profile_v3(&profile)
            .err()
            .ok_or("scenario drift must fail")?;
        assert_eq!(drift_error.code, "UNSAFE_LOCAL_BETA_PROFILE");
        profile.scenarios.swap(0, 1);
        profile.operational_checks.swap(0, 1);
        let check_order_error = validate_beta_profile_v3(&profile)
            .err()
            .ok_or("operational check order drift must fail")?;
        assert_eq!(check_order_error.code, "UNSAFE_LOCAL_BETA_PROFILE");
        Ok(())
    }
}
