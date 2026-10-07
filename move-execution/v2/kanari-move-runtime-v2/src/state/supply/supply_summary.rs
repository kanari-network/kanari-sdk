// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Per-token supply summaries (totals, visible, locked, untracked).

use super::*;

impl StateManager {
    /// Get a summary of the total, visible, locked, and untracked supply for a token.
    pub fn token_supply_summary(&self, token_type: &str) -> Result<TokenSupplySummary> {
        let token_type = Self::normalize_token_type(token_type);
        let total_supply = self.issued_supply_for_token(&token_type)?;
        let cached_visible = self
            .global_token_supplies
            .get(&token_type)
            .copied()
            .unwrap_or(0);
        let index_version = self
            .load_internal::<u32>(WALLET_SUPPLY_INDEX_VERSION_KEY)?
            .unwrap_or(0);
        let wallet_visible_supply = if token_type == GAS_COIN {
            // Native KANARI has legacy and fastpath writers. A stale-low cache
            // must not show fees or transferred coins as "untracked" when the
            // canonical owner/object index can still account for them. Keep
            // overcounts visible: do not clamp to total_supply here.
            let indexed = self.indexed_wallet_supply(&token_type)?;
            cached_visible.max(indexed)
        } else if index_version == WALLET_SUPPLY_INDEX_VERSION {
            cached_visible
        } else {
            let indexed = self.indexed_wallet_supply(&token_type)?;
            cached_visible.max(indexed)
        };
        let ledger_locked_supply = self.object_locked_supply_for_token(&token_type)?;
        // Only explicit object-locked records count as locked supply.
        // Any remaining gap between issued supply and accounted wallet/object balances
        // must stay visible as untracked instead of being silently re-labeled as locked.
        let object_locked_supply = ledger_locked_supply;
        let accounted_supply = wallet_visible_supply
            .checked_add(object_locked_supply)
            .require("Accounted token supply overflow")?;

        Ok(TokenSupplySummary {
            token_type,
            total_supply,
            wallet_visible_supply,
            object_locked_supply,
            accounted_supply,
            untracked_supply: total_supply.saturating_sub(accounted_supply),
        })
    }

    /// Batch version of [`Self::token_supply_summary`] that performs at most
    /// ONE canonical object pass for all requested tokens instead of one
    /// full scan per token. Numbers are identical to calling
    /// `token_supply_summary` per token: same cache/index/version logic,
    /// same locked-record aggregation, same overflow checks.
    pub fn token_supply_summaries(
        &self,
        token_types: &[String],
    ) -> Result<Vec<TokenSupplySummary>> {
        let normalized: Vec<String> = token_types
            .iter()
            .map(|t| Self::normalize_token_type(t))
            .collect();
        let index_version = self
            .load_internal::<u32>(WALLET_SUPPLY_INDEX_VERSION_KEY)?
            .unwrap_or(0);
        let needs_indexed: Vec<bool> = normalized
            .iter()
            .map(|t| t == GAS_COIN || index_version != WALLET_SUPPLY_INDEX_VERSION)
            .collect();
        // Single object pass: per-token owner balances + owner universe.
        let mut indexed_per_token: BTreeMap<String, BTreeMap<AccountAddress, u64>> =
            BTreeMap::new();
        let mut indexed_owners: BTreeSet<AccountAddress> = BTreeSet::new();
        if needs_indexed.iter().any(|b| *b) {
            let wanted: BTreeSet<&str> = normalized
                .iter()
                .zip(needs_indexed.iter())
                .filter(|(_, b)| **b)
                .map(|(t, _)| t.as_str())
                .collect();
            for (_, object) in self.query_objects(None, None, None, None, None)? {
                let Some((object_token, amount)) =
                    Self::balance_token_amount(&object.type_, &object.data)
                else {
                    continue;
                };
                if !wanted.contains(object_token.as_str()) {
                    continue;
                }
                indexed_owners.insert(object.owner);
                let entry = indexed_per_token
                    .entry(object_token)
                    .or_default()
                    .entry(object.owner)
                    .or_insert(0);
                *entry = entry
                    .checked_add(amount)
                    .require("Indexed owner token balance overflow")?;
            }
            if normalized.iter().any(|t| t == GAS_COIN) {
                indexed_owners.insert(Self::dao_account_address()?);
                for owner in self.owner_addresses()? {
                    indexed_owners.insert(owner);
                }
            }
        }
        // Locked records loaded once, aggregated per token.
        let locked_records = self.load_object_locked_coin_records()?;
        let mut out = Vec::with_capacity(normalized.len());
        for token_type in normalized {
            let total_supply = self.issued_supply_for_token(&token_type)?;
            let cached_visible = self
                .global_token_supplies
                .get(&token_type)
                .copied()
                .unwrap_or(0);
            let wallet_visible_supply = if token_type == GAS_COIN {
                let object_balances = indexed_per_token.get(GAS_COIN);
                let mut total = 0u64;
                for owner in &indexed_owners {
                    let object_balance = object_balances
                        .and_then(|m| m.get(owner))
                        .copied()
                        .unwrap_or(0);
                    let balance = self
                        .load_owner_state(owner)?
                        .map(|state| state.native_balance())
                        .filter(|ledger_balance| *ledger_balance > 0)
                        .unwrap_or(object_balance);
                    total = total
                        .checked_add(balance)
                        .require("Indexed wallet supply overflow")?;
                }
                cached_visible.max(total)
            } else if index_version == WALLET_SUPPLY_INDEX_VERSION {
                cached_visible
            } else {
                let indexed: u64 = indexed_per_token
                    .get(&token_type)
                    .map(|m| {
                        m.values().try_fold(0u64, |acc, amount| {
                            acc.checked_add(*amount)
                                .require("Indexed wallet supply overflow")
                        })
                    })
                    .transpose()?
                    .unwrap_or(0);
                cached_visible.max(indexed)
            };
            let object_locked_supply = locked_records
                .iter()
                .filter(|record| record.token_type == token_type)
                .map(|record| record.amount)
                .try_fold(0u64, |acc, amount| {
                    acc.checked_add(amount)
                        .require("Object-locked token supply overflow")
                })?;
            let accounted_supply = wallet_visible_supply
                .checked_add(object_locked_supply)
                .require("Accounted token supply overflow")?;
            out.push(TokenSupplySummary {
                token_type,
                total_supply,
                wallet_visible_supply,
                object_locked_supply,
                accounted_supply,
                untracked_supply: total_supply.saturating_sub(accounted_supply),
            });
        }
        Ok(out)
    }
}
