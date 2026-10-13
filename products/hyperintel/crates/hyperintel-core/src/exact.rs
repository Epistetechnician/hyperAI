use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Checked exact-arithmetic failures.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ExactArithmeticError {
    /// A fixed-width operation overflowed.
    #[error("exact arithmetic overflow")]
    Overflow,
    /// A bounded rate exceeded its domain.
    #[error("rate exceeds its fixed domain")]
    RateOutOfRange,
    /// A division denominator was zero.
    #[error("division by zero")]
    DivisionByZero,
}

/// Atomic settlement-asset units.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AtomicAmount(pub u64);

impl AtomicAmount {
    /// Checked addition.
    ///
    /// # Errors
    ///
    /// Returns overflow rather than wrapping.
    pub fn checked_add(self, other: Self) -> Result<Self, ExactArithmeticError> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or(ExactArithmeticError::Overflow)
    }

    /// Checked subtraction.
    ///
    /// # Errors
    ///
    /// Returns overflow rather than underflowing.
    pub fn checked_sub(self, other: Self) -> Result<Self, ExactArithmeticError> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .ok_or(ExactArithmeticError::Overflow)
    }

    /// Compute `ceil(self * multiplier / denominator)` with a wide intermediate.
    ///
    /// # Errors
    ///
    /// Returns on zero division or a result outside `u64`.
    pub fn checked_ceil_mul_div(
        self,
        multiplier: u64,
        denominator: u64,
    ) -> Result<Self, ExactArithmeticError> {
        if denominator == 0 {
            return Err(ExactArithmeticError::DivisionByZero);
        }
        let numerator = u128::from(self.0)
            .checked_mul(u128::from(multiplier))
            .ok_or(ExactArithmeticError::Overflow)?;
        let adjusted = numerator
            .checked_add(u128::from(denominator - 1))
            .ok_or(ExactArithmeticError::Overflow)?;
        let result = adjusted / u128::from(denominator);
        Ok(Self(
            result
                .try_into()
                .map_err(|_| ExactArithmeticError::Overflow)?,
        ))
    }
}

/// Parts-per-million rate in the inclusive range `0..=1_000_000`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct PartsPerMillion(u32);

impl PartsPerMillion {
    /// Construct a bounded rate.
    ///
    /// # Errors
    ///
    /// Returns when the value exceeds one million.
    pub const fn new(value: u32) -> Result<Self, ExactArithmeticError> {
        if value <= 1_000_000 {
            Ok(Self(value))
        } else {
            Err(ExactArithmeticError::RateOutOfRange)
        }
    }

    /// Return the raw rate.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}
impl TryFrom<u32> for PartsPerMillion {
    type Error = ExactArithmeticError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<PartsPerMillion> for u32 {
    fn from(value: PartsPerMillion) -> Self {
        value.0
    }
}

/// Basis-point rate in the inclusive range `0..=10_000`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct BasisPoints(u16);

impl BasisPoints {
    /// Construct a bounded basis-point rate.
    ///
    /// # Errors
    ///
    /// Returns when the value exceeds ten thousand.
    pub const fn new(value: u16) -> Result<Self, ExactArithmeticError> {
        if value <= 10_000 {
            Ok(Self(value))
        } else {
            Err(ExactArithmeticError::RateOutOfRange)
        }
    }

    /// Return the raw rate.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}
impl TryFrom<u16> for BasisPoints {
    type Error = ExactArithmeticError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<BasisPoints> for u16 {
    fn from(value: BasisPoints) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_exact_amount_rounds_up_and_never_wraps() {
        assert_eq!(
            AtomicAmount(1).checked_ceil_mul_div(1, 1_000_000),
            Ok(AtomicAmount(1))
        );
        assert_eq!(
            AtomicAmount(u64::MAX).checked_add(AtomicAmount(1)),
            Err(ExactArithmeticError::Overflow)
        );
        assert!(PartsPerMillion::new(1_000_001).is_err());
        assert!(BasisPoints::new(10_001).is_err());
    }
}
