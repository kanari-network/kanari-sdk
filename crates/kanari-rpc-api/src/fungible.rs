// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Fungible asset RPC types.

use crate::transaction::TransactionDetails;
use serde::{Deserialize, Serialize};

/// Cursor for paginating the holder list, which is ordered by
/// `(balance DESC, owner ASC)`.
///
/// A page returns holders strictly after this position, so advancing with
/// the returned `next_cursor` never repeats or skips entries even when
/// balances change between calls (entries that moved are repositioned, but
/// no entry is silently dropped from the iteration contract below).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FungibleAssetHolderCursor {
    pub balance: u64,
    pub owner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FungibleAssetInfo {
    pub token_type: String,
    pub name: String,
    pub symbol: String,
    /// No fallback: `None` means on-chain metadata unknown.
    /// Clients must show unknown, never assume 9/6/0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    pub total_supply: u64,
    pub wallet_visible_supply: u64,
    pub circulating_supply: u64,
    pub object_locked_supply: u64,
    pub accounted_supply: u64,
    pub untracked_supply: u64,
    pub holders_count: usize,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FungibleAssetHolder {
    pub owner: String,
    pub balance: u64,
    pub coin_object_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FungibleAssetHoldersResponse {
    pub token_type: String,
    pub holders: Vec<FungibleAssetHolder>,
    /// Cursor for the next page. `None` means this was the last page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<FungibleAssetHolderCursor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FungibleAssetTransactionsResponse {
    pub token_type: String,
    pub transactions: Vec<TransactionDetails>,
}
