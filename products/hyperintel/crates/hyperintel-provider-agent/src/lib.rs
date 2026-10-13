//! Signed provider-agent control assertions around the hermetic local harness.
//!
//! This crate contains no health observer, secret resolver, credential source,
//! socket, network client, external provider adapter, or deployment runtime.

mod control;
mod execution;

use hyperintel_core::{CanonicalizationError, TimestampError};
use thiserror::Error;

pub use control::{
    ProviderAgentArtifactKindV1, ProviderAgentAssertionRevocationV1, ProviderAgentBindingV1,
    ProviderAgentKeyPurposeV1, ProviderAgentKeyRegistryV1, ProviderAgentKeyRevocationV1,
    ProviderAgentReadinessClaimsV1, ProviderAgentReadinessStatusV1,
    ProviderAgentRevocationCheckpointV1, ProviderAgentRevocationSnapshotV1,
    ProviderAgentServiceManifestClaimsV1, ProviderTextStreamInterfaceV1,
    RegisteredProviderAgentKeyV1, SignedProviderAgentReadinessV1,
    SignedProviderAgentRevocationSnapshotV1, SignedProviderAgentServiceManifestV1,
};
pub use execution::{
    ProviderAgentExecutionContextV1, ProviderAgentExecutionV1, execute_provider_agent_v1,
};

/// Authorized repository state slice.
pub const PROVIDER_AGENT_STATE_SLICE_V1: &str = "hyperintel-provider-agent-v1";
/// Registered provider-agent key schema.
pub const PROVIDER_AGENT_KEY_SCHEMA_V1: &str = "hyperintel.provider-agent-key.v1";
/// Service-manifest claims schema.
pub const PROVIDER_AGENT_SERVICE_MANIFEST_CLAIMS_SCHEMA_V1: &str =
    "hyperintel.provider-agent-service-manifest-claims.v1";
/// Signed service-manifest wrapper schema.
pub const SIGNED_PROVIDER_AGENT_SERVICE_MANIFEST_SCHEMA_V1: &str =
    "hyperintel.signed-provider-agent-service-manifest.v1";
/// Readiness-assertion claims schema.
pub const PROVIDER_AGENT_READINESS_CLAIMS_SCHEMA_V1: &str =
    "hyperintel.provider-agent-readiness-claims.v1";
/// Signed readiness-assertion wrapper schema.
pub const SIGNED_PROVIDER_AGENT_READINESS_SCHEMA_V1: &str =
    "hyperintel.signed-provider-agent-readiness.v1";
/// Revocation-snapshot schema.
pub const PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1: &str =
    "hyperintel.provider-agent-revocation-snapshot.v1";
/// Signed revocation-snapshot wrapper schema.
pub const SIGNED_PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1: &str =
    "hyperintel.signed-provider-agent-revocation-snapshot.v1";
/// Fixed signature algorithm.
pub const ED25519_ALGORITHM_V1: &str = "ed25519";
/// Maximum service-manifest lifetime.
pub const MAX_SERVICE_MANIFEST_LIFETIME_SECONDS: u64 = 3_600;
/// Maximum readiness-assertion lifetime.
pub const MAX_READINESS_ASSERTION_LIFETIME_SECONDS: u64 = 300;
/// Maximum revocation-snapshot lifetime.
pub const MAX_REVOCATION_SNAPSHOT_LIFETIME_SECONDS: u64 = 300;

/// Closed provider-agent control and execution failures.
#[derive(Debug, Error)]
pub enum ProviderAgentError {
    /// A schema was not the exact supported V1 value.
    #[error("unsupported provider-agent schema")]
    UnsupportedSchema,
    /// The signature algorithm was not Ed25519.
    #[error("unsupported provider-agent signature algorithm")]
    UnsupportedAlgorithm,
    /// An identity appeared more than once.
    #[error("duplicate provider-agent identity")]
    DuplicateIdentity,
    /// A validity interval was inverted, stale, or too long.
    #[error("invalid provider-agent validity interval")]
    InvalidValidity,
    /// A required digest or sequence was zero.
    #[error("provider-agent identity must be nonzero")]
    ZeroIdentity,
    /// Key or signature bytes were malformed.
    #[error("invalid provider-agent Ed25519 encoding")]
    InvalidEncoding,
    /// Trust, signature, expectation, revocation, or checkpoint denied execution.
    #[error("provider-agent control assertion denied")]
    ControlDenied,
    /// Local execution inputs did not match the signed control lineage.
    #[error("provider-agent execution binding mismatch")]
    ExecutionBindingMismatch,
    /// Canonical bytes could not be produced.
    #[error("provider-agent canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
    /// Checked timestamp arithmetic failed.
    #[error("provider-agent timestamp arithmetic failed: {0}")]
    Timestamp(#[from] TimestampError),
    /// Existing local harness rejected or terminated the attempt.
    #[error("local execution harness failed: {0}")]
    LocalExecution(#[from] hyperintel_execution::LocalExecutionError),
    /// Route reconstruction failed before replay consumption.
    #[error("provider-agent route reconstruction failed: {0}")]
    Routing(#[from] hyperintel_market::RoutingError),
}
