//! Pure-data tenant identity, authorization, and signed access-grant verification.
//!
//! The crate has no token issuance, ambient key source, persistence, replay store,
//! network client, provider integration, wallet, chain, or settlement authority.

mod authorization;
mod model;
mod token;

use hyperintel_core::{CanonicalizationError, TimestampError};
use thiserror::Error;

pub use authorization::{
    TenantResourceBindingV1, TenantResourceKindV1, TenantResourcePurposeV1,
    validate_resource_binding_v1,
};
pub use model::{
    TenantIamSnapshotV1, TenantLifecycleV1, TenantPrincipalKindV1, TenantPrincipalV1,
    TenantRoleBindingV1, TenantRoleV1, TenantV1, ValidatedTenantIamSnapshotV1,
};
pub use token::{
    RegisteredTenantIamKeyV1, SignedTenantAccessGrantV1, SignedTenantRevocationSnapshotV1,
    TenantAccessExpectationV1, TenantAccessGrantClaimsV1, TenantAccessVerificationContextV1,
    TenantGrantRevocationV1, TenantIamActionV1, TenantIamKeyPurposeV1, TenantIamKeyRegistryV1,
    TenantKeyRevocationV1, TenantRevocationCheckpointV1, TenantRevocationSnapshotV1,
    TenantRevocationVerificationContextV1, ValidatedTenantRevocationSnapshotV1,
    VerifiedTenantAccessV1, verify_tenant_access_grant_v1, verify_tenant_revocation_snapshot_v1,
};

/// Authorized repository state slice.
pub const TENANT_IAM_STATE_SLICE_V1: &str = "hyperintel-tenant-iam-v1";
/// Exact V1 tenant IAM snapshot schema.
pub const TENANT_IAM_SNAPSHOT_SCHEMA_V1: &str = "hyperintel.tenant-iam-snapshot.v1";
/// Exact V1 signed one-action grant schema.
pub const SIGNED_TENANT_ACCESS_GRANT_SCHEMA_V1: &str = "hyperintel.signed-tenant-access-grant.v1";
/// Exact V1 access-grant claims schema.
pub const TENANT_ACCESS_GRANT_CLAIMS_SCHEMA_V1: &str = "hyperintel.tenant-access-grant-claims.v1";
/// Exact V1 signed revocation snapshot schema.
pub const SIGNED_TENANT_REVOCATION_SNAPSHOT_SCHEMA_V1: &str =
    "hyperintel.signed-tenant-revocation-snapshot.v1";
/// Exact V1 revocation snapshot schema.
pub const TENANT_REVOCATION_SNAPSHOT_SCHEMA_V1: &str = "hyperintel.tenant-revocation-snapshot.v1";
/// Exact V1 registered-key schema.
pub const TENANT_IAM_KEY_SCHEMA_V1: &str = "hyperintel.tenant-iam-key.v1";
/// Exact V1 resource-binding schema.
pub const TENANT_RESOURCE_BINDING_SCHEMA_V1: &str = "hyperintel.tenant-resource-binding.v1";
/// Fixed Ed25519 algorithm label.
pub const ED25519_ALGORITHM_V1: &str = "ed25519";
/// Maximum lifetime of a signed access grant.
pub const MAX_ACCESS_GRANT_LIFETIME_SECONDS: u64 = 300;
/// Maximum lifetime of a signed revocation snapshot.
pub const MAX_REVOCATION_SNAPSHOT_LIFETIME_SECONDS: u64 = 300;

/// Closed tenant IAM validation and verification failures.
#[derive(Debug, Error)]
pub enum TenantIamError {
    /// One schema identifier was not the exact V1 value.
    #[error("unsupported tenant IAM schema")]
    UnsupportedSchema,
    /// An algorithm was not the fixed Ed25519 identifier.
    #[error("unsupported tenant IAM signature algorithm")]
    UnsupportedAlgorithm,
    /// A caller-owned identity appears more than once.
    #[error("duplicate tenant IAM identity")]
    DuplicateIdentity,
    /// An interval is inverted or exceeds its fixed lifetime.
    #[error("invalid tenant IAM validity interval")]
    InvalidValidity,
    /// An identity reference is absent or crosses a tenant boundary.
    #[error("tenant IAM identity relationship is invalid")]
    InvalidIdentityRelationship,
    /// A lifecycle or inclusive revocation boundary denies access.
    #[error("tenant IAM subject is inactive or revoked")]
    InactiveOrRevoked,
    /// No current fixed role authorizes the one requested action.
    #[error("tenant IAM action is not granted")]
    ActionNotGranted,
    /// The caller-owned resource binding is invalid for the action.
    #[error("tenant IAM resource binding is invalid")]
    InvalidResourceBinding,
    /// A required caller-owned expectation did not match exactly.
    #[error("tenant IAM caller expectation mismatch")]
    ExpectationMismatch,
    /// A verification key could not be found for the exact identity and purpose.
    #[error("tenant IAM verification key not found")]
    UnknownKey,
    /// Key or signature bytes are malformed.
    #[error("invalid tenant IAM Ed25519 key or signature encoding")]
    InvalidEncoding,
    /// A key is outside validity or has been inclusively revoked.
    #[error("tenant IAM verification key is outside validity or revoked")]
    KeyOutsideValidityOrRevoked,
    /// Ed25519 signature verification failed.
    #[error("tenant IAM signature verification failed")]
    InvalidSignature,
    /// The revocation snapshot is older than the caller's accepted checkpoint.
    #[error("tenant IAM revocation snapshot rollback detected")]
    RevocationRollback,
    /// The same revocation sequence has a different caller-pinned digest.
    #[error("tenant IAM revocation snapshot fork detected")]
    RevocationFork,
    /// The access grant or its signing key is inclusively revoked.
    #[error("tenant IAM grant or signing key is revoked")]
    GrantOrKeyRevoked,
    /// A digest field used as an identity was zero.
    #[error("tenant IAM identity digest must be nonzero")]
    ZeroDigest,
    /// Canonical bytes could not be produced.
    #[error("tenant IAM canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
    /// Checked timestamp arithmetic failed.
    #[error("tenant IAM timestamp arithmetic failed: {0}")]
    Timestamp(#[from] TimestampError),
}
