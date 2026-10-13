//! Compatibility launcher for the authenticated `HyperAI` local service.

use std::{path::PathBuf, process::ExitCode};

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "hyperai-ui", about = "Run the local HyperAI interface")]
struct Args {
    /// Bind port on IPv4 loopback only.
    #[arg(long, default_value_t = 4317, value_parser = clap::value_parser!(u16).range(1..))]
    port: u16,
    /// Per-user `HyperAI` home. Defaults to the platform data directory.
    #[arg(long)]
    home: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(status) = outcomes_cli::run_eval_scorer_process() {
        return if status == ExitCode::SUCCESS {
            Ok(())
        } else {
            Err("The fixed synthetic evaluation scorer failed.".into())
        };
    }
    let args = Args::parse();
    hyperai_local_service::server::serve(args.home.as_deref(), args.port).await?;
    Ok(())
}
