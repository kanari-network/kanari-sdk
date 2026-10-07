// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Supply repairs and object-locked coin records.

use super::*;

impl StateManager {
    /// Repair legacy native wallet-visible overcount before startup/checkpoint work.
    ///
    /// Do not call this from normal transaction application. New bad transaction
    /// effects must be rejected by supply invariants instead of being silently
    /// normalized.
    pub fn repair_legacy_native_wallet_overcount(&mut self) -> Result<bool> {
        let indexed_visible = self.indexed_wallet_supply(GAS_COIN)?;
        let ledger_locked_supply = self.object_locked_supply_for_token(GAS_COIN)?;
        let max_wallet_visible = self.total_supply.saturating_sub(ledger_locked_supply);
        if indexed_visible <= max_wallet_visible {
            let current = self
                .global_token_supplies
                .get(GAS_COIN)
                .copied()
                .unwrap_or(0);
            let changed = current != indexed_visible;
            if changed {
                if indexed_visible == 0 {
                    self.global_token_supplies.remove(GAS_COIN);
                } else {
                    self.global_token_supplies
                        .insert(GAS_COIN.to_string(), indexed_visible);
                }
                let supplies_clone = self.global_token_supplies.clone();
                self.save_internal(b"global_token_supplies", &supplies_clone)?;
            }
            return Ok(changed);
        }

        let mut excess = indexed_visible - max_wallet_visible;
        let mut accounts = self
            .owner_addresses()?
            .into_iter()
            .map(|address| self.load_owner_state(&address))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .filter(|account| account.native_balance() > 0)
            .collect::<Vec<_>>();

        accounts.sort_by_key(|account| account.address.to_hex_literal());
        accounts.reverse();

        for mut account in accounts {
            if excess == 0 {
                break;
            }

            let current = account.native_balance();
            let debit = current.min(excess);
            let next = current - debit;
            account.set_token_balance_value(GAS_COIN, next);
            self.save_owner_record(&account)?;
            excess -= debit;
        }

        ensure!(
            excess == 0,
            "Unable to repair legacy native supply overcount: remaining_excess={}",
            excess
        );
        log::warn!(
            "[StateManager] Repaired legacy native wallet overcount: indexed_visible={} total_supply={} object_locked_supply={} max_wallet_visible={} repaired_excess={}",
            indexed_visible,
            self.total_supply,
            ledger_locked_supply,
            max_wallet_visible,
            indexed_visible - max_wallet_visible
        );
        self.sync_native_visible_supply_cache()?;
        let supplies_clone = self.global_token_supplies.clone();
        self.save_internal(b"global_token_supplies", &supplies_clone)?;
        Ok(true)
    }

    /// Repair the cached native wallet-visible overcount if it exceeds the maximum.
    pub fn repair_cached_native_wallet_overcount(&mut self) -> Result<bool> {
        let cached_visible = self
            .global_token_supplies
            .get(GAS_COIN)
            .copied()
            .unwrap_or(0);
        let ledger_locked_supply = self.object_locked_supply_for_token(GAS_COIN)?;
        let max_wallet_visible = self.total_supply.saturating_sub(ledger_locked_supply);
        if cached_visible <= max_wallet_visible {
            return Ok(false);
        }

        self.repair_legacy_native_wallet_overcount()
    }

    /// Load the object-locked coin records from the store.
    pub(crate) fn load_object_locked_coin_records(&self) -> Result<Vec<ObjectLockedCoinRecord>> {
        Ok(self
            .load_internal(OBJECT_LOCKED_COIN_RECORDS_KEY)?
            .unwrap_or_default())
    }

    /// Save the object-locked coin records to the store.
    pub(crate) fn save_object_locked_coin_records(
        &mut self,
        records: &[ObjectLockedCoinRecord],
    ) -> Result<()> {
        self.save_internal(OBJECT_LOCKED_COIN_RECORDS_KEY, records)
    }

    pub(crate) fn object_locked_supply_for_token(&self, token_type: &str) -> Result<u64> {
        let token_type = Self::normalize_token_type(token_type);
        self.load_object_locked_coin_records()?
            .into_iter()
            .filter(|record| record.token_type == token_type)
            .map(|record| record.amount)
            .try_fold(0u64, |acc, amount| {
                acc.checked_add(amount)
                    .require("Object-locked token supply overflow")
            })
    }
}
