use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};

/// Kanari module constants and utilities
pub struct KanariModule;

impl KanariModule {
    pub const KANARI_MODULE: &'static str = "kanari";

    /// The amount of Mist per Kanari token (10^-9 of a Kanari token)
    pub const MIST_PER_KANARI: u64 = 1_000_000_000;

    /// The total supply of Kanari denominated in whole Kanari tokens (100 Million)
    pub const TOTAL_SUPPLY_KANARI: u64 = 100_000_000;

    /// The total supply of Kanari denominated in Mist (100 Million * 10^9)
    pub const TOTAL_SUPPLY_MIST: u64 = 100_000_000_000_000_000;

    /// Get the module ID for kanari_system::kanari
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;

        let module_name =
            Identifier::new(Self::KANARI_MODULE).context("Invalid kanari module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Get function names used in kanari module
    pub fn function_names() -> KanariFunctions {
        KanariFunctions {
            new: "new",
            transfer: "transfer",
            burn: "burn",
        }
    }
    /// Convert Kanari to Mist
    pub fn kanari_to_mist(kanari: u64) -> u64 {
        kanari.saturating_mul(Self::MIST_PER_KANARI)
    }

    /// Convert Mist to Kanari (rounded down)
    pub fn mist_to_kanari(mist: u64) -> u64 {
        mist / Self::MIST_PER_KANARI
    }

    /// Format amount in Mist as Kanari string
    pub fn format_kanari(mist: u64) -> String {
        let kanari = mist / Self::MIST_PER_KANARI;
        let remaining_mist = mist % Self::MIST_PER_KANARI;
        if remaining_mist == 0 {
            format!("{} KANARI", kanari)
        } else {
            format!("{}.{:09} KANARI", kanari, remaining_mist)
        }
    }
}

/// Kanari module function names
pub struct KanariFunctions {
    pub new: &'static str,
    pub transfer: &'static str,
    pub burn: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(KanariModule::MIST_PER_KANARI, 1_000_000_000);
        assert_eq!(KanariModule::TOTAL_SUPPLY_KANARI, 100_000_000);
        assert_eq!(
            KanariModule::TOTAL_SUPPLY_MIST,
            KanariModule::TOTAL_SUPPLY_KANARI * KanariModule::MIST_PER_KANARI
        );
    }

    #[test]
    fn test_amount_conversion() {
        assert_eq!(
            KanariModule::kanari_to_mist(1),
            KanariModule::MIST_PER_KANARI
        );
        assert_eq!(
            KanariModule::kanari_to_mist(100),
            100 * KanariModule::MIST_PER_KANARI
        );
        assert_eq!(
            KanariModule::mist_to_kanari(KanariModule::MIST_PER_KANARI),
            1
        );
        assert_eq!(
            KanariModule::mist_to_kanari(KanariModule::TOTAL_SUPPLY_MIST),
            KanariModule::TOTAL_SUPPLY_KANARI
        );
    }

    #[test]
    fn test_format_kanari() {
        assert_eq!(KanariModule::format_kanari(1_000_000_000), "1 KANARI");
        assert_eq!(
            KanariModule::format_kanari(1_500_000_000),
            "1.500000000 KANARI"
        );
        assert_eq!(KanariModule::format_kanari(1), "0.000000001 KANARI");
    }

    #[test]
    fn test_module_id() {
        let module_id = KanariModule::get_module_id();
        assert!(module_id.is_ok());
    }
}
