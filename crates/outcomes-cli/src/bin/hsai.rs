//! Backward-compatible `hsai` entrypoint.

fn main() -> std::process::ExitCode {
    outcomes_cli::run_eval_scorer_process().unwrap_or_else(outcomes_cli::run_cli)
}
