use std::fmt;

use hyperintel_core::{ProtocolId, Sha256Digest};
use serde::{Deserialize, Deserializer, Serialize, de};
use thiserror::Error;

/// Schema emitted by an opaque V1 secret reference.
pub const SECRET_REFERENCE_SCHEMA_V1: &str = "hyperintel.secret-reference.v1";

/// A logical, exact-version handle containing no secret material.
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecretReferenceV1 {
    schema_version: String,
    provider_id: ProtocolId,
    handle_id: ProtocolId,
    version_digest: Sha256Digest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SecretReferenceDocumentV1 {
    schema_version: String,
    provider_id: ProtocolId,
    handle_id: ProtocolId,
    version_digest: Sha256Digest,
}

/// Strict secret-reference validation failures.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum SecretReferenceError {
    /// Schema is not the exact V1 identifier.
    #[error("unsupported secret-reference schema")]
    WrongSchemaVersion,
    /// A zero digest cannot identify immutable version metadata.
    #[error("secret reference must name a nonzero immutable version-metadata digest")]
    ZeroVersionDigest,
}

impl SecretReferenceV1 {
    /// Construct an opaque exact-version reference.
    ///
    /// # Errors
    ///
    /// Returns when the version digest is zero.
    pub fn new(
        provider_id: ProtocolId,
        handle_id: ProtocolId,
        version_digest: Sha256Digest,
    ) -> Result<Self, SecretReferenceError> {
        if version_digest.is_zero() {
            return Err(SecretReferenceError::ZeroVersionDigest);
        }
        Ok(Self {
            schema_version: SECRET_REFERENCE_SCHEMA_V1.to_owned(),
            provider_id,
            handle_id,
            version_digest,
        })
    }

    /// Return the logical secret-provider identity.
    #[must_use]
    pub const fn provider_id(&self) -> &ProtocolId {
        &self.provider_id
    }

    /// Return the logical opaque handle identity.
    #[must_use]
    pub const fn handle_id(&self) -> &ProtocolId {
        &self.handle_id
    }

    /// Return the digest of an immutable provider-issued version identifier or metadata record.
    #[must_use]
    pub const fn version_digest(&self) -> Sha256Digest {
        self.version_digest
    }
}

impl fmt::Debug for SecretReferenceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SecretReferenceV1")
            .field("schema_version", &self.schema_version)
            .field("provider_id", &self.provider_id)
            .field("handle_id", &"[REDACTED]")
            .field("version_digest", &"[REDACTED]")
            .finish()
    }
}

impl<'de> Deserialize<'de> for SecretReferenceV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let document = SecretReferenceDocumentV1::deserialize(deserializer)?;
        if document.schema_version != SECRET_REFERENCE_SCHEMA_V1 {
            return Err(de::Error::custom(SecretReferenceError::WrongSchemaVersion));
        }
        Self::new(
            document.provider_id,
            document.handle_id,
            document.version_digest,
        )
        .map_err(de::Error::custom)
    }
}
