use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use thiserror::Error;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

/// Failure to parse the fixed UTC whole-second timestamp format.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TimestampError {
    /// The timestamp was not canonical UTC RFC 3339 at whole-second precision.
    #[error("timestamp must use canonical YYYY-MM-DDTHH:MM:SSZ form")]
    Invalid,
}

/// Canonical UTC RFC 3339 timestamp with whole-second precision.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcTimestamp(String);

impl UtcTimestamp {
    /// Return the canonical timestamp string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert to a timestamp for ordering and arithmetic.
    ///
    /// # Errors
    ///
    /// Returns an error if internal validation cannot be reproduced.
    pub fn to_datetime(&self) -> Result<OffsetDateTime, TimestampError> {
        OffsetDateTime::parse(&self.0, &Rfc3339).map_err(|_| TimestampError::Invalid)
    }

    /// Add whole seconds while preserving canonical form.
    ///
    /// # Errors
    ///
    /// Returns an error on range or format failure.
    pub fn checked_add_seconds(&self, seconds: u64) -> Result<Self, TimestampError> {
        let seconds = i64::try_from(seconds).map_err(|_| TimestampError::Invalid)?;
        let value = self
            .to_datetime()?
            .checked_add(time::Duration::seconds(seconds))
            .ok_or(TimestampError::Invalid)?;
        value
            .format(&Rfc3339)
            .map_err(|_| TimestampError::Invalid)?
            .parse()
    }
}
impl fmt::Debug for UtcTimestamp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("UtcTimestamp")
            .field(&self.0)
            .finish()
    }
}
impl fmt::Display for UtcTimestamp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl FromStr for UtcTimestamp {
    type Err = TimestampError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 20 || !value.ends_with('Z') {
            return Err(TimestampError::Invalid);
        }
        let parsed = OffsetDateTime::parse(value, &Rfc3339).map_err(|_| TimestampError::Invalid)?;
        if parsed.offset() != time::UtcOffset::UTC || parsed.nanosecond() != 0 {
            return Err(TimestampError::Invalid);
        }
        Ok(Self(value.to_owned()))
    }
}
impl Serialize for UtcTimestamp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for UtcTimestamp {
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
    fn contract_timestamp_is_canonical_and_checked() {
        let start = "2026-07-20T12:00:00Z".parse::<UtcTimestamp>();
        assert_eq!(
            start
                .and_then(|value| value.checked_add_seconds(300))
                .as_ref()
                .map(UtcTimestamp::as_str),
            Ok("2026-07-20T12:05:00Z")
        );
        assert!("2026-07-20T12:00:00.000Z".parse::<UtcTimestamp>().is_err());
    }
}
