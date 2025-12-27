// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Helper functions for MoveRuntime resource parsing and object ID generation
use kanari_types::address::Address;
use kanari_types::balance::BalanceModule;
use kanari_types::coin::CoinModule;
use move_core_types::language_storage::{StructTag, TypeTag};

impl super::MoveRuntime {
    /// Check if struct tag represents a balance/coin resource
    pub(crate) fn is_balance_resource(&self, struct_tag: &StructTag) -> bool {
        let module_name = struct_tag.module.as_str();
        let struct_name = struct_tag.name.as_str();

        (module_name == CoinModule::COIN_MODULE && struct_name == CoinModule::COIN_STRUCT)
            || (module_name == BalanceModule::BALANCE_MODULE
                && struct_name == BalanceModule::BALANCE_STRUCT)
    }

    /// Check if struct tag represents a treasury resource
    pub(crate) fn is_treasury_resource(&self, struct_tag: &StructTag) -> bool {
        struct_tag.name.as_str() == CoinModule::TREASURY_CAP_STRUCT
    }

    /// Extract balance value from bytes for resources that may include UID + Balance
    pub(crate) fn extract_balance_from_bytes(
        &self,
        bytes: &[u8],
        struct_tag: &StructTag,
    ) -> Option<u64> {
        let module_name = struct_tag.module.as_str();
        let struct_name = struct_tag.name.as_str();

        // Balance<T> (no UID): just 8 bytes
        if module_name == BalanceModule::BALANCE_MODULE
            && struct_name == BalanceModule::BALANCE_STRUCT
        {
            if bytes.len() == 8 {
                let balance_bytes: [u8; 8] = bytes.try_into().ok()?;
                return Some(u64::from_le_bytes(balance_bytes));
            }
        }

        // Coin<T> (with UID): [32-byte address][8-byte id][8-byte balance]
        if module_name == CoinModule::COIN_MODULE && struct_name == CoinModule::COIN_STRUCT {
            if bytes.len() >= 48 {
                let balance_bytes: [u8; 8] = bytes[40..48].try_into().ok()?;
                return Some(u64::from_le_bytes(balance_bytes));
            }
        }

        None
    }

    /// Extract total supply from TreasuryCap bytes
    pub(crate) fn extract_treasury_total_from_bytes(&self, bytes: &[u8]) -> Option<u64> {
        // TreasuryCap: [32-byte address][8-byte id][8-byte total_supply]
        if bytes.len() >= 48 {
            let supply_bytes: [u8; 8] = bytes[40..48].try_into().ok()?;
            Some(u64::from_le_bytes(supply_bytes))
        } else {
            None
        }
    }

    /// Extract token type string from struct tag's type parameters
    pub(crate) fn token_type_from_struct_tag(&self, struct_tag: &StructTag) -> Option<String> {
        if let Some(first) = struct_tag.type_params.get(0) {
            if let TypeTag::Struct(st) = first {
                // Use kanari_types::Address to format the address consistently
                let addr = Address::from(st.address).to_hex();
                let module = st.module.as_str().to_string();
                let name = st.name.as_str().to_string();
                return Some(format!("0x{}::{}::{}", addr, module, name));
            }
        }
        None
    }
}
