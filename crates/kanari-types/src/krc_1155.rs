// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Canonical identifiers and stable wire-layout helpers for KRC-1155.
//!
//! KRC-1155 is Kanari's semi-fungible token standard (multi-token standard).
//! A token type is identified by its Move `TokenType` object; holder value lives
//! in owned `Balance` objects, and mint/burn authority lives exclusively in
//! `TokenCap`. Each token type has its own max supply.

use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
};
use std::str::FromStr;

/// KRC-1155 framework module metadata.
pub struct Krc1155Module;

impl Krc1155Module {
    pub const MODULE: &'static str = "krc_1155";
    pub const MODULE_PATH: &'static str = "0x2::krc_1155";

    // Struct Names
    pub const TOKEN_TYPE_STRUCT: &'static str = "TokenType";
    pub const BALANCE_STRUCT: &'static str = "Balance";
    pub const TOKEN_CAP_STRUCT: &'static str = "TokenCap";
    pub const MINTED_STRUCT: &'static str = "Minted";
    pub const BURNED_STRUCT: &'static str = "Burned";
    pub const TRANSFERRED_STRUCT: &'static str = "Transferred";

    // Abort Codes from krc_1155.move
    pub const E_TOKEN_CAP_MISMATCH: u64 = 0;
    pub const E_BALANCE_TYPE_MISMATCH: u64 = 1;
    pub const E_INSUFFICIENT_BALANCE: u64 = 2;
    pub const E_ZERO_AMOUNT: u64 = 3;
    pub const E_SUPPLY_OVERFLOW: u64 = 4;
    pub const E_SUPPLY_CAP_EXCEEDED: u64 = 5;

    // Event action codes
    pub const ACTION_MINTED: u8 = 1;
    pub const ACTION_BURNED: u8 = 2;
    pub const ACTION_TRANSFERRED: u8 = 3;

    /// BCS offset of `Balance.token_type`: `Balance.id: UID` occupies one address.
    pub const BALANCE_TOKEN_TYPE_OFFSET: usize = AccountAddress::LENGTH;
    pub const BALANCE_TOKEN_TYPE_END: usize =
        Self::BALANCE_TOKEN_TYPE_OFFSET + AccountAddress::LENGTH;
    /// BCS offset of `Balance.amount`.
    pub const BALANCE_AMOUNT_OFFSET: usize = Self::BALANCE_TOKEN_TYPE_END;
    pub const BALANCE_AMOUNT_END: usize = Self::BALANCE_AMOUNT_OFFSET + 8;

    /// Fully qualified Move module path for `0x2::krc_1155`.
    pub fn module_path() -> String {
        format!("{}::{}", Address::KANARI_SYSTEM_ADDRESS, Self::MODULE)
    }

    /// Get the module ID for `kanari_system::krc_1155`
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;

        let module_name = Identifier::new(Self::MODULE).context("Invalid krc_1155 module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Compatibility alias for [`Self::get_module_id`].
    pub fn module_id() -> ModuleId {
        ModuleId::new(
            Address::kanari_system_account_address(),
            Identifier::new(Self::MODULE).expect("KRC-1155 module identifier is valid"),
        )
    }

    /// Canonical function names used by the runtime and API.
    pub fn function_names() -> Krc1155Functions {
        Krc1155Functions {
            create: "create",
            share: "share",
            mint: "mint",
            mint_to: "mint_to",
            split: "split",
            transfer: "transfer",
            join: "join",
            burn: "burn",
            token_type: "token_type",
            balance_token_type: "balance_token_type",
            balance_value: "balance_value",
            total_supply: "total_supply",
            max_supply: "max_supply",
            name: "name",
            description: "description",
            image_url: "image_url",
        }
    }

    /// Returns true only for the canonical system KRC-1155 TokenType object.
    pub fn is_token_type_object_type(object_type: &str) -> bool {
        let Ok(TypeTag::Struct(tag)) = TypeTag::from_str(object_type) else {
            return false;
        };
        tag.address == Address::kanari_system_account_address()
            && tag.module.as_str() == Self::MODULE
            && tag.name.as_str() == Self::TOKEN_TYPE_STRUCT
            && tag.type_params.is_empty()
    }

    /// Returns true only for the canonical system KRC-1155 TokenCap object.
    pub fn is_token_cap_object_type(object_type: &str) -> bool {
        let Ok(TypeTag::Struct(tag)) = TypeTag::from_str(object_type) else {
            return false;
        };
        tag.address == Address::kanari_system_account_address()
            && tag.module.as_str() == Self::MODULE
            && tag.name.as_str() == Self::TOKEN_CAP_STRUCT
            && tag.type_params.is_empty()
    }

    /// Returns true only for the canonical system KRC-1155 Balance object.
    pub fn is_krc1155_object_type(object_type: &str) -> bool {
        let Ok(TypeTag::Struct(tag)) = TypeTag::from_str(object_type) else {
            return false;
        };
        tag.address == Address::kanari_system_account_address()
            && tag.module.as_str() == Self::MODULE
            && tag.name.as_str() == Self::BALANCE_STRUCT
            && tag.type_params.is_empty()
    }

    /// Extract the token type address from canonical KRC-1155 Balance BCS data.
    pub fn token_type_from_balance_bcs(data: &[u8]) -> Option<AccountAddress> {
        let bytes = data.get(Self::BALANCE_TOKEN_TYPE_OFFSET..Self::BALANCE_TOKEN_TYPE_END)?;
        AccountAddress::from_bytes(bytes).ok()
    }

    /// Extract the amount from canonical KRC-1155 Balance BCS data.
    pub fn amount_from_balance_bcs(data: &[u8]) -> Option<u64> {
        let bytes = data.get(Self::BALANCE_AMOUNT_OFFSET..Self::BALANCE_AMOUNT_END)?;
        Some(u64::from_le_bytes(bytes.try_into().ok()?))
    }
}

/// KRC-1155 module function names.
pub struct Krc1155Functions {
    pub create: &'static str,
    pub share: &'static str,
    pub mint: &'static str,
    pub mint_to: &'static str,
    pub split: &'static str,
    pub transfer: &'static str,
    pub join: &'static str,
    pub burn: &'static str,
    pub token_type: &'static str,
    pub balance_token_type: &'static str,
    pub balance_value: &'static str,
    pub total_supply: &'static str,
    pub max_supply: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub image_url: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_only_canonical_krc_1155_token_type() {
        assert!(Krc1155Module::is_token_type_object_type(
            "0x2::krc_1155::TokenType"
        ));
        assert!(!Krc1155Module::is_token_type_object_type(
            "0x3::krc_1155::TokenType"
        ));
        assert!(!Krc1155Module::is_token_type_object_type(
            "0x2::krc_1155::TokenType<u64>"
        ));
    }

    #[test]
    fn recognizes_only_canonical_krc_1155_token_cap() {
        assert!(Krc1155Module::is_token_cap_object_type(
            "0x2::krc_1155::TokenCap"
        ));
        assert!(!Krc1155Module::is_token_cap_object_type(
            "0x3::krc_1155::TokenCap"
        ));
    }

    #[test]
    fn recognizes_only_canonical_krc_1155_balance() {
        assert!(Krc1155Module::is_krc1155_object_type(
            "0x2::krc_1155::Balance"
        ));
        assert!(!Krc1155Module::is_krc1155_object_type(
            "0x3::krc_1155::Balance"
        ));
        assert!(!Krc1155Module::is_krc1155_object_type(
            "0x2::krc_1155::Balance<u64>"
        ));
    }

    #[test]
    fn reads_token_type_and_amount_from_balance_bcs() {
        let token_type = AccountAddress::from_hex_literal("0xcafe").unwrap();
        let amount: u64 = 42;
        let mut bytes = vec![0u8; Krc1155Module::BALANCE_AMOUNT_END];
        bytes[Krc1155Module::BALANCE_TOKEN_TYPE_OFFSET..Krc1155Module::BALANCE_TOKEN_TYPE_END]
            .copy_from_slice(token_type.as_ref());
        bytes[Krc1155Module::BALANCE_AMOUNT_OFFSET..Krc1155Module::BALANCE_AMOUNT_END]
            .copy_from_slice(&amount.to_le_bytes());
        assert_eq!(
            Krc1155Module::token_type_from_balance_bcs(&bytes),
            Some(token_type)
        );
        assert_eq!(Krc1155Module::amount_from_balance_bcs(&bytes), Some(amount));
        assert_eq!(
            Krc1155Module::token_type_from_balance_bcs(&bytes[..31]),
            None
        );
        assert_eq!(Krc1155Module::amount_from_balance_bcs(&bytes[..64]), None);
    }
    #[test]
    fn abort_codes_match_move_module() {
        assert_eq!(Krc1155Module::E_TOKEN_CAP_MISMATCH, 0);
        assert_eq!(Krc1155Module::E_BALANCE_TYPE_MISMATCH, 1);
        assert_eq!(Krc1155Module::E_INSUFFICIENT_BALANCE, 2);
        assert_eq!(Krc1155Module::E_ZERO_AMOUNT, 3);
        assert_eq!(Krc1155Module::E_SUPPLY_OVERFLOW, 4);
        assert_eq!(Krc1155Module::E_SUPPLY_CAP_EXCEEDED, 5);
    }
}
