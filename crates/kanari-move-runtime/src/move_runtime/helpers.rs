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
        data: &[u8],
    ) -> String {
        use kanari_crypto::hash_data_blake3;

        // Create unique input: owner + module_id + struct_name + data
        let mut input = Vec::new();
        input.extend_from_slice(owner.as_ref());
        input.extend_from_slice(struct_tag.address.as_ref());
        input.extend_from_slice(struct_tag.module.as_str().as_bytes());
        input.extend_from_slice(struct_tag.name.as_str().as_bytes());
        input.extend_from_slice(data);

        // Hash to get unique ID
        let hash = hash_data_blake3(&input);
        hex::encode(&hash[0..32]) // Use first 32 bytes for object ID
    }

    /// Check if struct tag represents a balance/coin resource
    pub(crate) fn is_balance_resource(&self, struct_tag: &StructTag) -> bool {
        // Common patterns: Coin<T>, Balance<T>, Account<T>
        let name = struct_tag.name.as_str();
        name == "Coin" || name == "Balance" || name == "Account"
    }

    /// Check if struct tag represents a treasury resource
    pub(crate) fn is_treasury_resource(&self, struct_tag: &StructTag) -> bool {
        struct_tag.name.as_str() == "TreasuryCap"
    }

    /// Extract balance value from bytes for resources that may include UID + Balance
    pub(crate) fn extract_balance_from_bytes(
        &self,
        bytes: &[u8],
        _struct_tag: &StructTag,
    ) -> Option<u64> {
        // For `Balance<T>` serialized alone, the bytes are just u64 little-endian.
        // For `Coin<T>` or `TreasuryCap<T>`, the layout is typically: UID (address) followed by u64.
        // We'll try both: if length >= 8, prefer the last 8 bytes as the u64 value.
        if bytes.len() >= 8 {
            let start = bytes.len() - 8;
            let balance_bytes: [u8; 8] = bytes[start..].try_into().ok()?;
            Some(u64::from_le_bytes(balance_bytes))
        } else {
            None
        }
    }

    /// Extract total supply from TreasuryCap bytes
    pub(crate) fn extract_treasury_total_from_bytes(&self, bytes: &[u8]) -> Option<u64> {
        // TreasuryCap layout: UID (address) + total_supply: u64
        if bytes.len() >= 8 {
            let start = bytes.len() - 8;
            let supply_bytes: [u8; 8] = bytes[start..].try_into().ok()?;
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
