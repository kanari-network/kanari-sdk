// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Helper functions for MoveRuntime resource parsing and object ID generation
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::{StructTag, TypeTag};

impl super::MoveRuntime {
    /// Generate unique object ID from address, type, and data
    /// Uses Blake3 hash for deterministic but unique IDs
    pub(crate) fn generate_object_id(
        &self,
        owner: &AccountAddress,
        struct_tag: &StructTag,
        _data: &[u8],
    ) -> String {
        use kanari_crypto::hash_data_blake3;
        // Create unique input: owner + module address + module name + struct name
        // NOTE: do NOT include resource data here to ensure stable object IDs
        let mut input = Vec::new();
        input.extend_from_slice(owner.as_ref());
        input.extend_from_slice(struct_tag.address.as_ref());
        input.extend_from_slice(struct_tag.module.as_str().as_bytes());
        input.extend_from_slice(struct_tag.name.as_str().as_bytes());

        let hash = hash_data_blake3(&input);
        hex::encode(&hash[0..32])
    }

    /// Check if struct tag represents a balance/coin resource
    pub(crate) fn is_balance_resource(&self, struct_tag: &StructTag) -> bool {
        let module_name = struct_tag.module.as_str();
        let struct_name = struct_tag.name.as_str();

        (module_name == "coin" && struct_name == "Coin")
            || (module_name == "balance" && struct_name == "Balance")
    }

    /// Check if struct tag represents a treasury resource
    pub(crate) fn is_treasury_resource(&self, struct_tag: &StructTag) -> bool {
        struct_tag.name.as_str() == "TreasuryCap"
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
        if module_name == "balance" && struct_name == "Balance" {
            if bytes.len() == 8 {
                let balance_bytes: [u8; 8] = bytes.try_into().ok()?;
                return Some(u64::from_le_bytes(balance_bytes));
            }
        }

        // Coin<T> (with UID): [32-byte address][8-byte id][8-byte balance]
        if module_name == "coin" && struct_name == "Coin" {
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
                let addr = st.address.short_str_lossless();
                let module = st.module.as_str().to_string();
                let name = st.name.as_str().to_string();
                return Some(format!("0x{}::{}::{}", addr, module, name));
            }
        }
        None
    }
}
