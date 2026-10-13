//! Versioned MCP tool allowlists exposed through ACP v1 sessions.

use std::path::{Path, PathBuf};

use agent_client_protocol::schema::v1::{EnvVariable, McpServer, McpServerStdio};
use thiserror::Error;

/// Original fixed MCP tool names retained for compatibility checks.
pub const MCP_TOOL_NAMES_V1: &[&str] = &[
    "hyperai_data_catalog",
    "hyperai_data_query",
    "hyperai_fleet_inventory",
    "hyperai_fleet_doctor",
    "hyperai_fleet_operation_preview",
    "hyperai_eval_catalog",
    "hyperai_eval_reports",
    "hyperai_trade_catalog",
    "hyperai_trade_series",
    "hyperai_trade_scenario",
];

/// Fixed read-only tool names in the workload-trace toolset.
pub const MCP_TOOL_NAMES_V2: &[&str] = &[
    "hyperai_data_catalog",
    "hyperai_data_query",
    "hyperai_fleet_inventory",
    "hyperai_fleet_doctor",
    "hyperai_fleet_operation_preview",
    "hyperai_eval_catalog",
    "hyperai_eval_reports",
    "hyperai_trade_catalog",
    "hyperai_trade_series",
    "hyperai_trade_scenario",
    "hyperai_market_workload_trace",
];

/// Append-only read-only tool list including the fixed seller-failure preview.
pub const MCP_TOOL_NAMES_V3: &[&str] = &[
    "hyperai_data_catalog",
    "hyperai_data_query",
    "hyperai_fleet_inventory",
    "hyperai_fleet_doctor",
    "hyperai_fleet_operation_preview",
    "hyperai_eval_catalog",
    "hyperai_eval_reports",
    "hyperai_trade_catalog",
    "hyperai_trade_series",
    "hyperai_trade_scenario",
    "hyperai_market_workload_trace",
    "hyperai_market_workload_trace_seller_failure_preview",
];

/// Append-only read-only tool list including fixed seller-failure and
/// inconclusive-outcome previews.
pub const MCP_TOOL_NAMES_V4: &[&str] = &[
    "hyperai_data_catalog",
    "hyperai_data_query",
    "hyperai_fleet_inventory",
    "hyperai_fleet_doctor",
    "hyperai_fleet_operation_preview",
    "hyperai_eval_catalog",
    "hyperai_eval_reports",
    "hyperai_trade_catalog",
    "hyperai_trade_series",
    "hyperai_trade_scenario",
    "hyperai_market_workload_trace",
    "hyperai_market_workload_trace_seller_failure_preview",
    "hyperai_market_workload_trace_inconclusive_preview",
];

/// Append-only read-only tool list with the fixed buyer supply comparison.
pub const MCP_TOOL_NAMES_V5: &[&str] = &[
    "hyperai_data_catalog",
    "hyperai_data_query",
    "hyperai_fleet_inventory",
    "hyperai_fleet_doctor",
    "hyperai_fleet_operation_preview",
    "hyperai_eval_catalog",
    "hyperai_eval_reports",
    "hyperai_trade_catalog",
    "hyperai_trade_series",
    "hyperai_trade_scenario",
    "hyperai_market_workload_trace",
    "hyperai_market_workload_trace_seller_failure_preview",
    "hyperai_market_workload_trace_inconclusive_preview",
    "hyperai_market_supply_comparison",
];

/// Append-only read-only tool list binding the displayed supply comparison to its quote preview.
pub const MCP_TOOL_NAMES_V6: &[&str] = &[
    "hyperai_data_catalog",
    "hyperai_data_query",
    "hyperai_fleet_inventory",
    "hyperai_fleet_doctor",
    "hyperai_fleet_operation_preview",
    "hyperai_eval_catalog",
    "hyperai_eval_reports",
    "hyperai_trade_catalog",
    "hyperai_trade_series",
    "hyperai_trade_scenario",
    "hyperai_market_workload_trace",
    "hyperai_market_workload_trace_seller_failure_preview",
    "hyperai_market_workload_trace_inconclusive_preview",
    "hyperai_market_supply_comparison",
    "hyperai_market_supply_quote_preview",
];

/// Local endpoint and short-lived read-only capability passed to the MCP child.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentMcpConfigV1 {
    /// IPv4-loopback service base URL.
    pub endpoint: String,
    /// Random, per-session token scoped only to agent-tool routes.
    pub capability: String,
    /// Absolute path to the bundled `HyperAI` MCP executable.
    pub executable: PathBuf,
}

/// MCP configuration could not be built without widening the session scope.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum McpConfigError {
    /// Endpoint or executable did not meet the local-only contract.
    #[error("MCP endpoint must use IPv4 loopback and executable must be absolute")]
    InvalidEndpoint,
    /// Capability is empty or outside the expected short token bounds.
    #[error("scoped MCP capability is invalid")]
    InvalidCapability,
}

impl AgentMcpConfigV1 {
    /// Build the ACP v1 stdio server declaration after checking local scope.
    ///
    /// # Errors
    ///
    /// Returns `McpConfigError` when the endpoint or capability is outside
    /// the local-only contract.
    pub fn into_schema(self) -> Result<McpServer, McpConfigError> {
        let valid_port = self
            .endpoint
            .strip_prefix("http://127.0.0.1:")
            .and_then(|port| port.parse::<u16>().ok())
            .is_some_and(|port| port > 0);
        if !valid_port || !self.executable.is_absolute() {
            return Err(McpConfigError::InvalidEndpoint);
        }
        if self.capability.len() < 32 || self.capability.len() > 256 {
            return Err(McpConfigError::InvalidCapability);
        }
        let server = McpServerStdio::new("hyperai-readonly", self.executable)
            .args(vec!["stdio".to_owned()])
            .env(vec![
                EnvVariable::new("HYPERAI_MCP_ENDPOINT", self.endpoint),
                EnvVariable::new("HYPERAI_MCP_CAPABILITY", self.capability),
            ]);
        Ok(McpServer::Stdio(server))
    }
}

/// Find the bundled MCP executable next to a packaged `HyperAI` executable.
#[must_use]
pub fn sibling_mcp_executable(executable: &Path) -> Option<PathBuf> {
    let parent = executable.parent()?;
    let file_name = if cfg!(windows) {
        "hyperai-mcp.exe"
    } else {
        "hyperai-mcp"
    };
    Some(parent.join(file_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_only_the_fixed_loopback_stdio_bridge() -> Result<(), McpConfigError> {
        let mcp = AgentMcpConfigV1 {
            endpoint: "http://127.0.0.1:4312".to_owned(),
            capability: "a".repeat(48),
            executable: PathBuf::from("/opt/hyperai/bin/hyperai-mcp"),
        }
        .into_schema()?;
        let encoded = serde_json::to_value(mcp).map_err(|_| McpConfigError::InvalidEndpoint)?;
        assert_eq!(encoded["command"], "/opt/hyperai/bin/hyperai-mcp");
        assert_eq!(encoded["args"][0], "stdio");
        assert_eq!(encoded["env"].as_array().map(Vec::len), Some(2));
        Ok(())
    }

    #[test]
    fn rejects_non_loopback_endpoints() {
        let config = AgentMcpConfigV1 {
            endpoint: "https://example.com".to_owned(),
            capability: "a".repeat(48),
            executable: PathBuf::from("/opt/hyperai-mcp"),
        };
        assert_eq!(config.into_schema(), Err(McpConfigError::InvalidEndpoint));
    }
}
