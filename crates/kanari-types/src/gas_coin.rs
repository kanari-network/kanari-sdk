// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};

/// Gas module constants and utilities
pub struct GasModule;
pub const GAS_COIN: &str = "0x2::kanari::KANARI";

impl GasModule {
    pub const GAS_MODULE: &'static str = "kanari";

    /// The amount of Mist per Kanari token (10^-9 of a Kanari token)
    pub const MIST_PER_GAS: u64 = 1_000_000_000;

    /// Native token decimals.
    pub const KANARI_DECIMALS: u32 = 9;

    /// The total supply of Kanari denominated in whole Kanari tokens (11 Million)
    pub const TOTAL_SUPPLY_GAS: u64 = 11_000_000;

    /// The total supply of Kanari denominated in Mist (11 Million * 10^9)
    pub const TOTAL_SUPPLY_MIST: u64 = 11_000_000_000_000_000;

    /// Get the module ID for kanari_system::kanari
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;

        let module_name = Identifier::new(Self::GAS_MODULE).context("Invalid gas module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Fully qualified Move module path for `0x2::kanari`.
    pub fn module_path() -> String {
        format!("{}::{}", Address::KANARI_SYSTEM_ADDRESS, Self::GAS_MODULE)
    }

    /// Get function names used in gas module
    pub fn function_names() -> GasFunctions {
        GasFunctions {
            init: "init",
            transfer: "transfer",
            burn: "burn",
        }
    }
    /// Convert Gas to Mist
    pub fn gas_to_mist(gas: u64) -> u64 {
        gas.saturating_mul(Self::MIST_PER_GAS)
    }

    /// Convert Mist to gas (rounded down)
    pub fn mist_to_gas(mist: u64) -> u64 {
        mist / Self::MIST_PER_GAS
    }

    /// Format amount in Mist as gas string
    pub fn format_gas(mist: u64) -> String {
        let gas = mist / Self::MIST_PER_GAS;
        let remaining_mist = mist % Self::MIST_PER_GAS;
        if remaining_mist == 0 {
            format!("{} KANARI", gas)
        } else {
            format!("{}.{:09} KANARI", gas, remaining_mist)
        }
    }

    /// Parse a decimal KANARI amount (`"12.5"`) into exact Mist using pure
    /// integer string math. Rejects negatives, more than 9 fraction digits,
    /// malformed input, and values overflowing `u64`.
    ///
    /// Float parsing (`"12.5".parse::<f64>() * 1e9`) silently drops Mist past
    /// 2^53 — this is the only accepted string->Mist path for money movement.
    pub fn parse_kanari_to_mist(s: &str) -> Result<u64> {
        let trimmed = s.trim();
        if trimmed.is_empty() || trimmed.starts_with('-') {
            anyhow::bail!("invalid KANARI amount: {s:?}");
        }
        let mut parts = trimmed.split('.');
        let int_part = parts.next().unwrap_or("0");
        let frac_part = parts.next().unwrap_or("");
        if parts.next().is_some() {
            anyhow::bail!("invalid KANARI amount: {s:?}");
        }
        let int_digits = if int_part.is_empty() { "0" } else { int_part };
        if !int_digits.bytes().all(|b| b.is_ascii_digit())
            || !frac_part.bytes().all(|b| b.is_ascii_digit())
        {
            anyhow::bail!("invalid KANARI amount: {s:?}");
        }
        if frac_part.len() > Self::KANARI_DECIMALS as usize {
            anyhow::bail!(
                "too many fraction digits (max {}): {s:?}",
                Self::KANARI_DECIMALS
            );
        }
        let mut frac_padded = frac_part.to_string();
        while frac_padded.len() < Self::KANARI_DECIMALS as usize {
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
            .with_context(|| format!("KANARI amount overflows u64: {s:?}"))
    }

    /// Format Mist into exact KANARI (`12500000000` -> `"12.5"`), trimming
    /// trailing zeros. Pure integer string math — never float.
    pub fn format_mist_to_kanari(mist: u64) -> String {
        let digits = mist.to_string();
        let scale = Self::KANARI_DECIMALS as usize;
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
}

/// Gas module function names
pub struct GasFunctions {
    pub init: &'static str,
    pub transfer: &'static str,
    pub burn: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(GasModule::MIST_PER_GAS, 1_000_000_000);
        assert_eq!(GasModule::TOTAL_SUPPLY_GAS, 11_000_000);
        assert_eq!(
            GasModule::TOTAL_SUPPLY_MIST,
            GasModule::TOTAL_SUPPLY_GAS * GasModule::MIST_PER_GAS
        );
    }

    #[test]
    fn test_amount_conversion() {
        assert_eq!(GasModule::gas_to_mist(1), GasModule::MIST_PER_GAS);
        assert_eq!(GasModule::gas_to_mist(100), 100 * GasModule::MIST_PER_GAS);
        assert_eq!(GasModule::mist_to_gas(GasModule::MIST_PER_GAS), 1);
        assert_eq!(
            GasModule::mist_to_gas(GasModule::TOTAL_SUPPLY_MIST),
            GasModule::TOTAL_SUPPLY_GAS
        );
    }

    #[test]
    fn test_format_gas() {
        assert_eq!(GasModule::format_gas(1_000_000_000), "1 KANARI");
        assert_eq!(GasModule::format_gas(1_500_000_000), "1.500000000 KANARI");
        assert_eq!(GasModule::format_gas(1), "0.000000001 KANARI");
    }

    #[test]
    fn test_module_id() {
        let module_id = GasModule::get_module_id();
        assert!(module_id.is_ok());
    }

    #[test]
    fn mist_per_kanari_matches_chain_decimal_definition() {
        assert_eq!(GasModule::MIST_PER_GAS, 1_000_000_000);
        assert_eq!(GasModule::KANARI_DECIMALS, 9);
    }

    #[test]
    fn kanari_decimal_parsing_is_exact() {
        assert_eq!(GasModule::parse_kanari_to_mist("0").unwrap(), 0);
        assert_eq!(GasModule::parse_kanari_to_mist("1").unwrap(), 1_000_000_000);
        assert_eq!(
            GasModule::parse_kanari_to_mist("10.5").unwrap(),
            10_500_000_000
        );
        assert_eq!(GasModule::parse_kanari_to_mist("0.000000001").unwrap(), 1);
        // Past f64 precision (2^53 Mist): floats silently drop Mist here.
        assert_eq!(
            GasModule::parse_kanari_to_mist("10999989").unwrap(),
            10_999_989_000_000_000
        );
        assert_eq!(
            GasModule::parse_kanari_to_mist("10999989.000000001").unwrap(),
            10_999_989_000_000_001
        );
        assert!(GasModule::parse_kanari_to_mist("").is_err());
        assert!(GasModule::parse_kanari_to_mist("-1").is_err());
        assert!(GasModule::parse_kanari_to_mist("1.0000000001").is_err());
        assert!(GasModule::parse_kanari_to_mist("abc").is_err());
        assert!(GasModule::parse_kanari_to_mist("1.2.3").is_err());
        assert!(GasModule::parse_kanari_to_mist("18446744073.709551616").is_err());
    }

    #[test]
    fn mist_formatting_is_exact_and_trimmed() {
        assert_eq!(GasModule::format_mist_to_kanari(0), "0");
        assert_eq!(GasModule::format_mist_to_kanari(1), "0.000000001");
        assert_eq!(GasModule::format_mist_to_kanari(100), "0.0000001");
        assert_eq!(GasModule::format_mist_to_kanari(1_000_000_000), "1");
        assert_eq!(GasModule::format_mist_to_kanari(10_500_000_000), "10.5");
        assert_eq!(
            GasModule::format_mist_to_kanari(10_999_989_000_000_000),
            "10999989"
        );
        assert_eq!(
            GasModule::format_mist_to_kanari(u64::MAX),
            "18446744073.709551615"
        );
    }
}
