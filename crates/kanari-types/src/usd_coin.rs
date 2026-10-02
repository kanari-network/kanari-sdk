// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! USD stablecoin (`0x2::usd::USD`) constants and exact decimal helpers.
//!
//! The coin has 6 decimals (USDC-style). All string <-> base-unit conversion
//! uses pure integer string math — never float — mirroring
//! [`crate::gas_coin::GasModule`] for KANARI.

use crate::address::Address;
use anyhow::{Context, Result};

/// USD stablecoin module constants and utilities.
pub struct UsdModule;

pub const USD_COIN: &str = "0x2::usd::USD";

impl UsdModule {
    pub const USD_MODULE: &'static str = "usd";
    pub const USD_STRUCT: &'static str = "USD";

    /// Base units per whole USD (10^6).
    pub const UNITS_PER_USD: u64 = 1_000_000;

    /// USD decimals.
    pub const USD_DECIMALS: u32 = 6;

    /// Fully qualified Move module path for `0x2::usd`.
    pub fn module_path() -> String {
        format!("{}::{}", Address::KANARI_SYSTEM_ADDRESS, Self::USD_MODULE)
    }

    /// Fully qualified `transfer_amount` entry point used by the faucet.
    pub fn transfer_entry() -> (String, String) {
        (Self::module_path(), "transfer_amount".to_string())
    }

    /// Parse a decimal USD amount (`"12.5"`) into exact base units.
    pub fn parse_usd_to_units(s: &str) -> Result<u64> {
        let trimmed = s.trim();
        if trimmed.is_empty() || trimmed.starts_with('-') {
            anyhow::bail!("invalid USD amount: {s:?}");
        }
        let mut parts = trimmed.split('.');
        let int_part = parts.next().unwrap_or("0");
        let frac_part = parts.next().unwrap_or("");
        if parts.next().is_some() {
            anyhow::bail!("invalid USD amount: {s:?}");
        }
        let int_digits = if int_part.is_empty() { "0" } else { int_part };
        if !int_digits.bytes().all(|b| b.is_ascii_digit())
            || !frac_part.bytes().all(|b| b.is_ascii_digit())
        {
            anyhow::bail!("invalid USD amount: {s:?}");
        }
        if frac_part.len() > Self::USD_DECIMALS as usize {
            anyhow::bail!(
                "too many fraction digits (max {}): {s:?}",
                Self::USD_DECIMALS
            );
        }
        let mut frac_padded = frac_part.to_string();
        while frac_padded.len() < Self::USD_DECIMALS as usize {
            frac_padded.push('0');
        }
        let combined = format!("{int_digits}{frac_padded}");
        let normalized = combined.trim_start_matches('0');
        let normalized = if normalized.is_empty() {
            "0"
        } else {
            normalized
        };
        normalized
            .parse::<u64>()
            .with_context(|| format!("USD amount overflows u64: {s:?}"))
    }

    /// Format base units into exact USD (`2500000` -> `"2.5"`), trimming
    /// trailing zeros.
    pub fn format_units_to_usd(units: u64) -> String {
        let digits = units.to_string();
        let scale = Self::USD_DECIMALS as usize;
        if digits.len() <= scale {
            let frac = format!("{:0>width$}", digits, width = scale);
            let trimmed = frac.trim_end_matches('0');
            if trimmed.is_empty() {
                return "0".to_string();
            }
            return format!("0.{trimmed}");
        }
        let split = digits.len() - scale;
        let (int_part, frac_part) = digits.split_at(split);
        let trimmed = frac_part.trim_end_matches('0');
        if trimmed.is_empty() {
            int_part.to_string()
        } else {
            format!("{int_part}.{trimmed}")
        }
    }

    /// Human display (`1500000` -> `"1.5 USD"`).
    pub fn format_units(units: u64) -> String {
        format!("{} USD", Self::format_units_to_usd(units))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usd_constants_match_move_module() {
        assert_eq!(UsdModule::UNITS_PER_USD, 1_000_000);
        assert_eq!(UsdModule::USD_DECIMALS, 6);
        assert_eq!(USD_COIN, "0x2::usd::USD");
    }

    #[test]
    fn usd_parsing_is_exact() {
        assert_eq!(UsdModule::parse_usd_to_units("0").unwrap(), 0);
        assert_eq!(UsdModule::parse_usd_to_units("1").unwrap(), 1_000_000);
        assert_eq!(UsdModule::parse_usd_to_units("10.5").unwrap(), 10_500_000);
        assert_eq!(UsdModule::parse_usd_to_units("0.000001").unwrap(), 1);
        assert_eq!(UsdModule::parse_usd_to_units("100").unwrap(), 100_000_000);
        assert!(UsdModule::parse_usd_to_units("").is_err());
        assert!(UsdModule::parse_usd_to_units("-1").is_err());
        assert!(UsdModule::parse_usd_to_units("1.0000001").is_err());
        assert!(UsdModule::parse_usd_to_units("abc").is_err());
    }

    #[test]
    fn usd_formatting_is_exact_and_trimmed() {
        assert_eq!(UsdModule::format_units_to_usd(0), "0");
        assert_eq!(UsdModule::format_units_to_usd(1), "0.000001");
        assert_eq!(UsdModule::format_units_to_usd(1_000_000), "1");
        assert_eq!(UsdModule::format_units_to_usd(2_500_000), "2.5");
        assert_eq!(UsdModule::format_units(1_500_000), "1.5 USD");
    }
}
