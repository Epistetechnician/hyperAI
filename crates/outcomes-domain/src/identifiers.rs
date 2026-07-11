use std::{fmt, str::FromStr};

use data_encoding::{BASE32_NOPAD, HEXLOWER};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use subtle::ConstantTimeEq;
use thiserror::Error;

/// Validation failures for digest and derived claim identifiers.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum IdentifierError {
    /// A SHA-256 digest was not exactly 32 bytes of lowercase hexadecimal.
    #[error("SHA-256 digest must be 64 lowercase hexadecimal characters")]
    InvalidDigest,
    /// A claim identifier was not the lowercase, unpadded Base32 form.
    #[error("claim ID must be claim_ followed by lowercase unpadded Base32 SHA-256 bytes")]
    InvalidClaimId,
}

/// A SHA-256 digest serialized as 64 lowercase hexadecimal characters.
#[derive(Clone, Copy, JsonSchema)]
#[schemars(with = "String")]
pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    /// Construct a digest from its raw bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Return the raw digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Return the canonical lowercase hexadecimal representation.
    #[must_use]
    pub fn to_hex(self) -> String {
        HEXLOWER.encode(&self.0)
    }
}

impl PartialEq for Sha256Digest {
    fn eq(&self, other: &Self) -> bool {
        self.0.ct_eq(&other.0).into()
    }
}

impl Eq for Sha256Digest {}

impl std::hash::Hash for Sha256Digest {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.0);
    }
}

impl fmt::Debug for Sha256Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Sha256Digest")
            .field(&self.to_hex())
            .finish()
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

impl FromStr for Sha256Digest {
    type Err = IdentifierError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 64
            || value
                .bytes()
                .any(|byte| !matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Err(IdentifierError::InvalidDigest);
        }
        let decoded = HEXLOWER
            .decode(value.as_bytes())
            .map_err(|_| IdentifierError::InvalidDigest)?;
        let bytes = decoded
            .try_into()
            .map_err(|_| IdentifierError::InvalidDigest)?;
        Ok(Self(bytes))
    }
}

impl Serialize for Sha256Digest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Sha256Digest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(de::Error::custom)
    }
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
    fn contract_digest_round_trip() {
        let digest = Sha256Digest::from_bytes([0xab; 32]);
        let encoded = digest.to_string();
        assert_eq!(encoded.parse::<Sha256Digest>(), Ok(digest));
    }

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
