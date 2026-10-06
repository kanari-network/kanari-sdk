// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Shared typing helpers for `Coin`/`Balance` resources.

use kanari_types::balance::BalanceModule;
use kanari_types::coin::CoinModule;
use move_core_types::language_storage::{StructTag, TypeTag};

/// Check if struct tag represents a balance/coin resource
pub(crate) fn is_balance_struct(struct_tag: &StructTag) -> bool {
    let module_name = struct_tag.module.as_str();
    let struct_name = struct_tag.name.as_str();
    (module_name == CoinModule::COIN_MODULE && struct_name == CoinModule::COIN_STRUCT)
        || (module_name == BalanceModule::BALANCE_MODULE
            && struct_name == BalanceModule::BALANCE_STRUCT)
}

/// Extract token type string from struct tag's type parameters
pub(crate) fn token_type_from_struct_tag(struct_tag: &StructTag) -> Option<String> {
    if let Some(TypeTag::Struct(st)) = struct_tag.type_params.first() {
        // Normalize via Move's Display impl so all code paths use one canonical
        // token type key (e.g. `0x2::kanari::KANARI`).
        return Some(format!("{}", st));
    }
    None
}

/// Size of a Move object UID in bytes (address)
const UID_SIZE: usize = 32;
/// Size of a u64 field in bytes
const U64_SIZE: usize = 8;

/// Extract the balance value from a `Coin<T>`/`Balance<T>` resource blob.
///
/// `Coin<T>` is `[32-byte UID][8-byte balance]`; `Balance<T>` is a bare u64,
/// read from the tail so this mirrors `write_balance_to_object_bytes` when a
/// blob carries trailing bytes beyond the 8-byte value.
pub(crate) fn extract_balance_from_object_bytes(data: &[u8], struct_tag: &StructTag) -> Option<u64> {
    let module_name = struct_tag.module.as_str();
    let struct_name = struct_tag.name.as_str();

    if module_name == CoinModule::COIN_MODULE && struct_name == CoinModule::COIN_STRUCT {
        if data.len() < UID_SIZE + U64_SIZE {
            return None;
        }
        let bytes: [u8; U64_SIZE] = data[UID_SIZE..(UID_SIZE + U64_SIZE)].try_into().ok()?;
        return Some(u64::from_le_bytes(bytes));
    }

    if module_name == BalanceModule::BALANCE_MODULE
        && struct_name == BalanceModule::BALANCE_STRUCT
    {
        if data.len() < U64_SIZE {
            return None;
        }
        let bytes: [u8; U64_SIZE] = data[data.len() - U64_SIZE..].try_into().ok()?;
        return Some(u64::from_le_bytes(bytes));
    }

    None
}
