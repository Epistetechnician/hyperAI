use std::{fmt, str::FromStr};

use data_encoding::HEXLOWER;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use subtle::ConstantTimeEq;
use thiserror::Error;

/// Identifier validation failures.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum IdentifierError {
    /// A SHA-256 digest was not canonical lowercase hexadecimal.
    #[error("SHA-256 digest must be 64 lowercase hexadecimal characters")]
    InvalidDigest,
    /// A protocol identifier violated the bounded canonical grammar.
    #[error("protocol ID must be 1-128 lowercase ASCII letters, digits, '.', '_', ':', or '-'")]
    InvalidProtocolId,
}

/// A SHA-256 digest serialized as 64 lowercase hexadecimal characters.
#[derive(Clone, Copy)]
pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    /// Construct a digest from raw bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Return raw digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Return true when every digest byte is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.0.ct_eq(&[0; 32]).into()
    }

    /// Return canonical lowercase hexadecimal.
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
impl PartialOrd for Sha256Digest {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Sha256Digest {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}
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
        Ok(Self(
            decoded
                .try_into()
                .map_err(|_| IdentifierError::InvalidDigest)?,
        ))
    }
}
impl Serialize for Sha256Digest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}
impl<'de> Deserialize<'de> for Sha256Digest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

/// Bounded lowercase identifier used for externally meaningful protocol keys.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ProtocolId(String);

impl ProtocolId {
    /// Return the canonical string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProtocolId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl FromStr for ProtocolId {
    type Err = IdentifierError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.len() > 128
            || value
                .bytes()
                .any(|byte| !matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b':' | b'-'))
        {
            return Err(IdentifierError::InvalidProtocolId);
        }
        Ok(Self(value.to_owned()))
    }
}
impl<'de> Deserialize<'de> for ProtocolId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_digest_and_protocol_id_round_trip() {
        let digest = Sha256Digest::from_bytes([0xab; 32]);
        assert_eq!(digest.to_string().parse::<Sha256Digest>(), Ok(digest));
        assert!("provider:alpha-1".parse::<ProtocolId>().is_ok());
        assert!("Provider Alpha".parse::<ProtocolId>().is_err());
    }
}
