//! HSAI command-line entrypoint.

use std::{collections::HashMap, fs, path::PathBuf, process::ExitCode};

use clap::{Args, Parser, Subcommand, ValueEnum};
use hyperliquid_hip4::{Hip4ReadOnlyBackend, decode_asset_id, encode_asset_ids};
use outcomes_application::{LocalScenario, run_local_scenario};
use outcomes_blobstore::FileBlobStore;
use outcomes_corpus::{
    LocalMaterializationLimits, LocalMaterializationRequest, bundled_corpus_profiles,
    materialize_pinned_local_root,
};
use outcomes_domain::{
    ClaimSpecDraftV1, ClaimSpecV1, MarketBindingV1, OutcomeEvidenceBundleV1,
    ReconciliationResultV1, ResolutionRecordV1, SettlementNetwork, UnsignedSettlementIntentV1,
    UtcTimestamp,
};
use outcomes_evidence::verify_evidence_bundle;
use outcomes_settlement::{CapabilityMatrix, build_intent, preflight};
use serde_json::{Value, json};

const CAPABILITIES: &str = include_str!("../../../config/adapter-capabilities.json");
const CORPUS_PROFILES: &str = include_str!("../../../config/corpus-profiles.json");

#[derive(Debug, Parser)]
#[command(name = "hsai", version, about = "HSAI verifiable outcome protocol")]
struct Cli {
    #[command(subcommand)]
    command: RootCommand,
}

#[derive(Debug, Subcommand)]
enum RootCommand {
    /// Verifiable outcome protocol commands.
    Outcomes(OutcomesArgs),
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
    False,
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

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(value) => match serde_json::to_string_pretty(&value) {
            Ok(output) => {
                println!("{output}");
                ExitCode::SUCCESS
            }
            Err(error) => fail(5, "OUTPUT_ENCODING_FAILED", &error.to_string()),
        },
        Err(error) => fail(error.exit_code, error.code, &error.message),
    }
}

fn run(cli: Cli) -> Result<Value, CliError> {
    match cli.command {
        RootCommand::Outcomes(outcomes) => match outcomes.command {
            OutcomesCommand::Adapter(adapter) => run_adapter(adapter),
            OutcomesCommand::Claim(claim) => run_claim(claim),
            OutcomesCommand::Evidence(evidence) => run_evidence(evidence),
            OutcomesCommand::Demo(demo) => run_demo(demo),
            OutcomesCommand::Hip4(hip4) => run_hip4(hip4),
            OutcomesCommand::Settlement(settlement) => run_settlement(settlement),
            OutcomesCommand::Corpus(corpus) => run_corpus(corpus),
        },
    }
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
                ScenarioArg::False => LocalScenario::False,
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
    code: &'static str,
    message: String,
}

impl CliError {
    fn input(code: &'static str, message: &str) -> Self {
        Self {
            exit_code: 2,
            code,
            message: message.to_owned(),
        }
    }

    fn validation(message: &str) -> Self {
        Self {
            exit_code: 3,
            code: "VALIDATION_FAILED",
            message: message.to_owned(),
        }
    }

    fn verification(message: &str) -> Self {
        Self {
            exit_code: 4,
            code: "VERIFICATION_FAILED",
            message: message.to_owned(),
        }
    }

    fn operation(code: &'static str, message: &str) -> Self {
        Self {
            exit_code: 6,
            code,
            message: message.to_owned(),
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn internal(message: impl ToString) -> Self {
        Self {
            exit_code: 5,
            code: "INTERNAL_ERROR",
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
    use outcomes_domain::Sha256Digest;

    use super::*;

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
}
