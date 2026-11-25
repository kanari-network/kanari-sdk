// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Shared typing helpers for `Coin`/`Balance` resources.
//!
//! Byte layouts and struct-tag checks live in `kanari-types`; this module only
//! provides the runtime's combined `Coin<T> | Balance<T>` view.

use kanari_types::balance::BalanceModule;
use kanari_types::coin::CoinModule;
use move_core_types::language_storage::StructTag;

/// Check if struct tag represents a balance/coin resource.
pub(crate) fn is_balance_struct(struct_tag: &StructTag) -> bool {
    CoinModule::is_coin_struct_tag(struct_tag) || BalanceModule::is_balance_struct_tag(struct_tag)
}

/// Extract token type string from a struct tag's first type parameter.
pub(crate) fn token_type_from_struct_tag(struct_tag: &StructTag) -> Option<String> {
    CoinModule::token_type_from_struct_tag(struct_tag)
}

/// Extract the balance value from a `Coin<T>`/`Balance<T>` resource blob.
///
/// `Coin<T>` is `[32-byte UID][8-byte balance]`; `Balance<T>` is a bare u64,
/// read from the tail so this mirrors the writer when a blob carries trailing
/// bytes beyond the value.
pub(crate) fn extract_balance_from_object_bytes(
    data: &[u8],
    struct_tag: &StructTag,
) -> Option<u64> {
    if CoinModule::is_coin_struct_tag(struct_tag) {
        return CoinModule::read_balance(data);
    }
    if BalanceModule::is_balance_struct_tag(struct_tag) {
        return BalanceModule::read_balance(data);
    }
    None
}
