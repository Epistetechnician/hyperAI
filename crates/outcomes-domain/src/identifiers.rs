use std::{fmt, str::FromStr};

use data_encoding::BASE32_NOPAD;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use thiserror::Error;

use crate::Sha256Digest;

/// Validation failures for digest and derived claim identifiers.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum IdentifierError {
    /// A claim identifier was not the lowercase, unpadded Base32 form.
    #[error("claim ID must be claim_ followed by lowercase unpadded Base32 SHA-256 bytes")]
    InvalidClaimId,
}

/// A claim ID derived from a claim-spec digest.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
#[schemars(with = "String")]
pub struct ClaimId(String);

impl ClaimId {
    /// Derive the claim ID from the canonical claim-spec digest.
    #[must_use]
    pub fn from_digest(digest: Sha256Digest) -> Self {
        let encoded = BASE32_NOPAD.encode(digest.as_bytes()).to_ascii_lowercase();
        Self(format!("claim_{encoded}"))
    }

    /// Return the canonical identifier string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ClaimId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("ClaimId").field(&self.0).finish()
    }
}

impl fmt::Display for ClaimId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for ClaimId {
    type Err = IdentifierError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let encoded = value
            .strip_prefix("claim_")
            .ok_or(IdentifierError::InvalidClaimId)?;
        if encoded.len() != 52
            || encoded
                .bytes()
                .any(|byte| !matches!(byte, b'a'..=b'z' | b'2'..=b'7'))
        {
            return Err(IdentifierError::InvalidClaimId);
        }
        let uppercase = encoded.to_ascii_uppercase();
        let decoded = BASE32_NOPAD
            .decode(uppercase.as_bytes())
            .map_err(|_| IdentifierError::InvalidClaimId)?;
        if decoded.len() != 32 || BASE32_NOPAD.encode(&decoded).to_ascii_lowercase() != encoded {
            return Err(IdentifierError::InvalidClaimId);
        }
        Ok(Self(value.to_owned()))
    }
}

impl Serialize for ClaimId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ClaimId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_claim_id_round_trip() {
        let id = ClaimId::from_digest(Sha256Digest::from_bytes([0x42; 32]));
        assert_eq!(id.as_str().parse::<ClaimId>(), Ok(id));
    }

    #[test]
    fn contract_claim_id_rejects_uppercase() {
        let id = ClaimId::from_digest(Sha256Digest::from_bytes([0x42; 32]));
        assert_eq!(
            id.as_str().to_ascii_uppercase().parse::<ClaimId>(),
            Err(IdentifierError::InvalidClaimId)
        );
    }
}
