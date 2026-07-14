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
}
