//! HSAI command-line entrypoint.

mod beta_inventory;
mod beta_provenance;

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use hyperliquid_hip4::{Hip4ReadOnlyBackend, decode_asset_id, encode_asset_ids};
use outcomes_application::{LocalScenario, ScenarioReport, run_local_scenario};
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
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use beta_inventory::{finalize_run_inventory, verify_run_inventory};
use beta_provenance::{
    BetaProvenanceProvider, BetaRunProvenanceV1, CommandBetaProvenanceProvider,
    validate_recorded_provenance,
};

const CAPABILITIES: &str = include_str!("../../../config/adapter-capabilities.json");
const CORPUS_PROFILES: &str = include_str!("../../../config/corpus-profiles.json");
const LOCAL_BETA_PROFILE: &str = include_str!("../../../config/local-beta-profile.json");
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
            OutcomesCommand::Beta(beta) => run_beta(beta),
        },
    }
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

fn embedded_beta_profile() -> Result<(LocalBetaProfile, outcomes_domain::Sha256Digest), CliError> {
    let profile: LocalBetaProfile =
        serde_json::from_str(LOCAL_BETA_PROFILE).map_err(CliError::internal)?;
    validate_beta_profile(&profile)?;
    let digest = domain_separated_digest("hsai.local-beta-profile.v1", &profile)
        .map_err(CliError::internal)?;
    Ok((profile, digest))
}

fn finalize_beta_run(artifact_root: &Path, run_id: &str) -> Result<Value, CliError> {
    validate_run_id(run_id)?;
    let (profile, profile_digest) = embedded_beta_profile()?;
    let run_root = artifact_root.join(run_id);
    verify_beta_run_semantics(&run_root, run_id, &profile)?;
    let inventory =
        finalize_run_inventory(&run_root, run_id, profile_digest).map_err(map_inventory_error)?;
    verify_beta_run_semantics(&run_root, run_id, &profile)?;
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
    let (profile, profile_digest) = embedded_beta_profile()?;
    let run_root = artifact_root.join(run_id);
    verify_beta_run_semantics(&run_root, run_id, &profile)?;
    let inventory =
        verify_run_inventory(&run_root, run_id, profile_digest).map_err(map_inventory_error)?;
    serde_json::to_value(inventory).map_err(CliError::internal)
}

fn map_inventory_error(error: beta_inventory::InventoryError) -> CliError {
    let code = error.code;
    drop(error);
    CliError::operation(code, "beta retained-artifact inventory verification failed")
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
    let profile: LocalBetaProfile =
        serde_json::from_str(LOCAL_BETA_PROFILE).map_err(CliError::internal)?;
    validate_beta_profile(&profile)?;
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
    let profile_digest = domain_separated_digest("hsai.local-beta-profile.v1", &profile)
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

    run_exported_tamper_check(&run_root)?;
    let manifest = json!({
        "schema_version": "hsai.local-beta-run-manifest.v2",
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
    use beta_provenance::{
        BetaInputProvenanceV1, BetaRunProvenanceV1, BetaSourceProvenanceV1, BetaToolProvenanceV1,
        ProvenanceError,
    };
    use outcomes_domain::Sha256Digest;

    use super::*;

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
        let copied_profile: LocalBetaProfile =
            read_json_path(&run_root.join("local-beta-profile.json"))
                .map_err(|error| std::io::Error::other(error.message))?;
        let copied_profile_digest =
            domain_separated_digest("hsai.local-beta-profile.v1", &copied_profile)?;
        assert_eq!(
            output["profile_digest"],
            serde_json::to_value(copied_profile_digest)?
        );
        assert_eq!(output["profile_file"], "local-beta-profile.json");
        assert_eq!(output["schema_version"], "hsai.local-beta-run-manifest.v2");
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
        let mut profile: LocalBetaProfile = serde_json::from_str(LOCAL_BETA_PROFILE)?;
        profile.profile_id = "unapproved-local-profile".to_owned();
        let identity_error = validate_beta_profile(&profile)
            .err()
            .ok_or("profile ID drift must fail")?;
        assert_eq!(identity_error.code, "UNSAFE_LOCAL_BETA_PROFILE");
        profile.profile_id = "hermetic-trusted-operator-v1".to_owned();
        profile.live_writes = true;
        let live_error = validate_beta_profile(&profile)
            .err()
            .ok_or("live writes must fail")?;
        assert_eq!(live_error.code, "UNSAFE_LOCAL_BETA_PROFILE");
        profile.live_writes = false;
        profile.scenarios.swap(0, 1);
        let drift_error = validate_beta_profile(&profile)
            .err()
            .ok_or("scenario drift must fail")?;
        assert_eq!(drift_error.code, "UNSAFE_LOCAL_BETA_PROFILE");
        Ok(())
    }
}
