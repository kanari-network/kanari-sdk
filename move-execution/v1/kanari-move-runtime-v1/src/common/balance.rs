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
