use std::{fmt, str::FromStr};

use data_encoding::HEXLOWER;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use subtle::ConstantTimeEq;
use thiserror::Error;

/// A SHA-256 digest was not exactly 32 bytes of lowercase hexadecimal.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("SHA-256 digest must be 64 lowercase hexadecimal characters")]
pub struct DigestParseError;

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
    type Err = DigestParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 64
            || value
                .bytes()
                .any(|byte| !matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Err(DigestParseError);
        }
        let decoded = HEXLOWER
            .decode(value.as_bytes())
            .map_err(|_| DigestParseError)?;
        let bytes = decoded.try_into().map_err(|_| DigestParseError)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_round_trip() {
        let digest = Sha256Digest::from_bytes([0xab; 32]);
        let encoded = digest.to_string();
        assert_eq!(encoded.parse::<Sha256Digest>(), Ok(digest));
    }

    #[test]
    fn digest_rejects_uppercase() {
        assert!("AB".repeat(32).parse::<Sha256Digest>().is_err());
    }
}
