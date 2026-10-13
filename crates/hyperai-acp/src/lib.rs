//! Stable ACP v1 session lifecycle for `HyperAI`'s local interface.
//!
//! This crate owns ACP wire types and process startup. It provides no file or
//! terminal tools and never treats the selected agent as an authority.

pub mod profile;
pub mod session;
pub mod tools;

pub use profile::{AgentProfileSummaryV1, AgentProfileV1, ProfileEnvironmentV1, ProfileError};
pub use session::{AgentSessionManager, SessionError, SessionEventV1, SessionSnapshotV1};
pub use tools::{
    AgentMcpConfigV1, MCP_TOOL_NAMES_V1, MCP_TOOL_NAMES_V2, MCP_TOOL_NAMES_V3, MCP_TOOL_NAMES_V4,
    MCP_TOOL_NAMES_V5, MCP_TOOL_NAMES_V6, sibling_mcp_executable,
};
