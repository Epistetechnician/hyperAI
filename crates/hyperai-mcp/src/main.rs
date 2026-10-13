//! `hyperai-mcp` stdio entrypoint started by the selected ACP agent.

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match hyperai_mcp::serve_stdio().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(_) => std::process::ExitCode::FAILURE,
    }
}
