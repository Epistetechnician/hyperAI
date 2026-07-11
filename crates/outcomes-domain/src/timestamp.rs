use std::{fmt, str::FromStr};

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use thiserror::Error;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

/// Failure to parse the protocol's fixed UTC whole-second timestamp format.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TimestampError {
    /// The timestamp was not canonical UTC RFC 3339 at whole-second precision.
    #[error("timestamp must use canonical YYYY-MM-DDTHH:MM:SSZ form")]
    Invalid,
}

/// A canonical UTC RFC 3339 timestamp with whole-second precision.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
#[schemars(with = "String")]
pub struct UtcTimestamp(String);

impl UtcTimestamp {
    /// Return the canonical timestamp string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert the validated value to a timestamp for ordering and arithmetic.
    ///
    /// # Errors
    ///
    /// Returns [`TimestampError`] if the internally stored representation is not
    /// valid RFC 3339. Safe constructors prevent this during normal use.
    pub fn to_datetime(&self) -> Result<OffsetDateTime, TimestampError> {
        OffsetDateTime::parse(&self.0, &Rfc3339).map_err(|_| TimestampError::Invalid)
    }

    /// Add whole seconds while preserving the protocol timestamp format.
    ///
    /// # Errors
    ///
    /// Returns [`TimestampError`] if the duration is out of range or formatting
    /// cannot preserve canonical whole-second UTC form.
    pub fn checked_add_seconds(&self, seconds: u64) -> Result<Self, TimestampError> {
        let seconds = i64::try_from(seconds).map_err(|_| TimestampError::Invalid)?;
        let value = self
            .to_datetime()?
            .checked_add(time::Duration::seconds(seconds))
            .ok_or(TimestampError::Invalid)?;
        let formatted = value
            .format(&Rfc3339)
            .map_err(|_| TimestampError::Invalid)?;
        formatted.parse()
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
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_timestamp_accepts_whole_second_utc() {
        assert!("2026-07-10T12:34:56Z".parse::<UtcTimestamp>().is_ok());
    }

    #[test]
    fn contract_timestamp_rejects_fractional_or_offset_forms() {
        assert!("2026-07-10T12:34:56.000Z".parse::<UtcTimestamp>().is_err());
        assert!("2026-07-10T08:34:56-04:00".parse::<UtcTimestamp>().is_err());
    }

    #[test]
    fn contract_timestamp_adds_whole_seconds_canonically() {
        let start = "2026-07-10T12:00:00Z".parse::<UtcTimestamp>();
        assert_eq!(
            start
                .and_then(|value| value.checked_add_seconds(300))
                .as_ref()
                .map(UtcTimestamp::as_str),
            Ok("2026-07-10T12:05:00Z")
        );
    }
}
