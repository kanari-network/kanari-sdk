/// Defines a fixed-point numeric type with a 32-bit integer part and
/// a 32-bit fractional part.
use std::cmp::{max, min};

/// Maximum value for u64
const MAX_U64: u128 = 18446744073709551615;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FixedPoint32 {
    value: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FixedPointError {
    DenominatorIsZero,
    DivisionError,
    MultiplicationError,
    DivisionByZero,
    RatioOutOfRange,
}

impl FixedPoint32 {
    /// Multiply a u64 integer by a fixed-point number
    pub fn multiply_u64(val: u64, multiplier: FixedPoint32) -> Result<u64, FixedPointError> {
        let unscaled_product = (val as u128) * (multiplier.value as u128);
        let product = unscaled_product >> 32;
        if product > MAX_U64 {
            return Err(FixedPointError::MultiplicationError);
        }
        Ok(product as u64)
    }

    /// Divide a u64 integer by a fixed-point number
    pub fn divide_u64(val: u64, divisor: FixedPoint32) -> Result<u64, FixedPointError> {
        if divisor.value == 0 {
            return Err(FixedPointError::DivisionByZero);
        }
        let scaled_value = (val as u128) << 32;
        let quotient = scaled_value / (divisor.value as u128);
        if quotient > MAX_U64 {
            return Err(FixedPointError::DivisionError);
        }
        Ok(quotient as u64)
    }

    /// Create from rational number
    pub fn create_from_rational(numerator: u64, denominator: u64) -> Result<Self, FixedPointError> {
        if denominator == 0 {
            return Err(FixedPointError::DenominatorIsZero);
        }
        let scaled_numerator = (numerator as u128) << 64;
        let scaled_denominator = (denominator as u128) << 32;
        let quotient = scaled_numerator / scaled_denominator;
        if (quotient == 0 && numerator != 0) || quotient > MAX_U64 {
            return Err(FixedPointError::RatioOutOfRange);
        }
        Ok(FixedPoint32 { value: quotient as u64 })
    }

    /// Create from raw value
    pub fn create_from_raw_value(value: u64) -> Self {
        FixedPoint32 { value }
    }

    /// Get raw value
    pub fn get_raw_value(&self) -> u64 {
        self.value
    }

    /// Check if value is zero
    pub fn is_zero(&self) -> bool {
        self.value == 0
    }

    /// Get minimum of two values
    pub fn min(num1: FixedPoint32, num2: FixedPoint32) -> FixedPoint32 {
        FixedPoint32 { value: min(num1.value, num2.value) }
    }

    /// Get maximum of two values
    pub fn max(num1: FixedPoint32, num2: FixedPoint32) -> FixedPoint32 {
        FixedPoint32 { value: max(num1.value, num2.value) }
    }

    /// Create from u64
    pub fn create_from_u64(val: u64) -> Result<Self, FixedPointError> {
        let value = (val as u128) << 32;
        if value > MAX_U64 {
            return Err(FixedPointError::RatioOutOfRange);
        }
        Ok(FixedPoint32 { value: value as u64 })
    }

    /// Get floor value
    pub fn floor(&self) -> u64 {
        self.value >> 32
    }

    /// Get ceiling value
    pub fn ceil(&self) -> u64 {
        let floored_num = self.floor() << 32;
        if self.value == floored_num {
            return floored_num >> 32;
        }
        ((floored_num as u128 + (1 << 32)) >> 32) as u64
    }

    /// Round to nearest integer
    pub fn round(&self) -> u64 {
        let floored_num = self.floor() << 32;
        let boundary = floored_num + ((1 << 32) / 2);
        if self.value < boundary {
            floored_num >> 32
        } else {
            self.ceil()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let fp = FixedPoint32::create_from_u64(5).unwrap();
        assert_eq!(fp.floor(), 5);
        assert_eq!(fp.ceil(), 5);
        assert_eq!(fp.round(), 5);
    }

    #[test]
    fn test_rational() {
        let fp = FixedPoint32::create_from_rational(1, 2).unwrap();
        assert_eq!(fp.floor(), 0);
        assert_eq!(fp.ceil(), 1);
        assert_eq!(fp.round(), 1);
    }

    #[test]
    fn test_multiplication() {
        let fp = FixedPoint32::create_from_rational(3, 2).unwrap();
        let result = FixedPoint32::multiply_u64(10, fp).unwrap();
        assert_eq!(result, 15);
    }
}