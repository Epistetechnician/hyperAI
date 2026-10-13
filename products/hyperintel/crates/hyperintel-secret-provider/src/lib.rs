//! Signed authorization and bounded local injection of one exact secret.
//!
//! This crate invokes only a caller-supplied in-process resolver. It contains
//! no environment, file, keychain, network, KMS, HSM, cloud-secret-manager,
//! provider, wallet, chain, settlement, cache, retry, persistence, or logging
//! adapter.

mod authorization;
mod material;
mod resolver;

use hyperintel_core::{CanonicalizationError, TimestampError};
use thiserror::Error;

pub use authorization::{
    OperatorSecretAuthorizationClaimsV1, OperatorSecretAuthorizationExpectationV1,
    OperatorSecretAuthorizationRevocationV1, OperatorSecretAuthorizationVerificationContextV1,
    OperatorSecretKeyPurposeV1, OperatorSecretKeyRegistryV1, OperatorSecretKeyRevocationV1,
    OperatorSecretRevocationCheckpointV1, OperatorSecretRevocationStateV1,
    RegisteredOperatorSecretKeyV1, SecretPurposeV1, SignedOperatorSecretAuthorizationV1,
    VerifiedOperatorSecretAuthorizationV1, operator_secret_revocation_state_digest_v1,
    secret_reference_digest_v1,
};
pub use material::{MAX_SECRET_MATERIAL_BYTES, SecretMaterialV1};
pub use resolver::{
    InjectedSecretResolverErrorV1, InjectedSecretResolverV1, ResolvedSecretCandidateV1,
    ResolvedSecretV1, SecretResolutionRequestV1, resolve_authorized_secret_v1,
};

/// Authorized repository state slice.
pub const SECRET_PROVIDER_STATE_SLICE_V1: &str = "hyperintel-local-secret-injection-v1";
/// Exact registered operator-secret key schema.
pub const OPERATOR_SECRET_KEY_SCHEMA_V1: &str = "hyperintel.operator-secret-key.v1";
/// Exact signed authorization wrapper schema.
pub const SIGNED_OPERATOR_SECRET_AUTHORIZATION_SCHEMA_V1: &str =
    "hyperintel.signed-operator-secret-authorization.v1";
/// Exact operator-secret authorization claims schema.
pub const OPERATOR_SECRET_AUTHORIZATION_CLAIMS_SCHEMA_V1: &str =
    "hyperintel.operator-secret-authorization-claims.v1";
/// Exact caller-owned revocation-state schema.
pub const OPERATOR_SECRET_REVOCATION_STATE_SCHEMA_V1: &str =
    "hyperintel.operator-secret-revocation-state.v1";
/// Fixed Ed25519 algorithm label.
pub const ED25519_ALGORITHM_V1: &str = "ed25519";
/// Maximum lifetime of one operator-secret authorization.
pub const MAX_OPERATOR_SECRET_AUTHORIZATION_LIFETIME_SECONDS: u64 = 300;

/// Closed local secret authorization and resolution failures.
#[derive(Debug, Error)]
pub enum SecretProviderError {
    /// A schema identifier was not the exact V1 value.
    #[error("unsupported operator-secret schema")]
    UnsupportedSchema,
    /// The signature algorithm was not Ed25519.
    #[error("unsupported operator-secret signature algorithm")]
    UnsupportedAlgorithm,
    /// A caller-owned identity appeared more than once.
    #[error("duplicate operator-secret identity")]
    DuplicateIdentity,
    /// A time interval was inverted, stale, or too long.
    #[error("invalid operator-secret validity interval")]
    InvalidValidity,
    /// Key or signature bytes were malformed.
    #[error("invalid operator-secret Ed25519 encoding")]
    InvalidEncoding,
    /// Authorization, expectation, trust, signature, or revocation denied use.
    #[error("operator-secret authorization denied")]
    AuthorizationDenied,
    /// A required identity digest was zero.
    #[error("operator-secret identity digest must be nonzero")]
    ZeroDigest,
    /// A secret-material value was empty or exceeded the fixed bound.
    #[error("resolved secret material is invalid")]
    InvalidMaterial,
    /// The injected resolver failed without exposing provider detail.
    #[error("injected secret resolver is unavailable")]
    ResolverUnavailable,
    /// Resolver identity or returned metadata did not match exactly.
    #[error("injected secret resolver binding mismatch")]
    ResolverBindingMismatch,
    /// Canonical bytes could not be produced.
    #[error("operator-secret canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
    /// Checked timestamp arithmetic failed.
    #[error("operator-secret timestamp arithmetic failed: {0}")]
    Timestamp(#[from] TimestampError),
}
