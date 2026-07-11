//! HSAI command-line entrypoint.

use std::{collections::HashMap, fs, path::PathBuf, process::ExitCode};

use clap::{Args, Parser, Subcommand, ValueEnum};
use outcomes_application::{LocalScenario, run_local_scenario};
use outcomes_blobstore::FileBlobStore;
use outcomes_domain::{ClaimSpecDraftV1, ClaimSpecV1, OutcomeEvidenceBundleV1};
use outcomes_evidence::verify_evidence_bundle;
use serde_json::{Value, json};

const CAPABILITIES: &str = include_str!("../../../config/adapter-capabilities.json");

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
        },
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
