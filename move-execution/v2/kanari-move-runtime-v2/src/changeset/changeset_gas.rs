// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Gas accounting on a changeset.

use crate::changeset::ChangeSet;
use kanari_types::coin::CoinModule;
use move_core_types::account_address::AccountAddress;

impl ChangeSet {
    /// Collect gas fees to DAO
    pub fn collect_gas(&mut self, dao_address: AccountAddress, gas_amount: u64) {
        self.collect_gas_for_coin(dao_address, kanari_types::gas_coin::GAS_COIN, gas_amount);
    }

    /// Canonical settlement coin for this changeset (`None` = native KANARI).
    pub fn gas_coin_type(&self) -> &str {
        self.gas_coin_type
            .as_deref()
            .unwrap_or(kanari_types::gas_coin::GAS_COIN)
    }

    /// Whether gas settles in native KANARI (legacy accounting path).
    pub fn is_native_gas(&self) -> bool {
        self.gas_coin_type
            .as_deref()
            .is_none_or(|coin| coin == kanari_types::gas_coin::GAS_COIN)
    }

    /// Record the settlement coin before collecting fees.
    pub fn set_gas_coin_type(&mut self, coin_type: &str) {
        let normalized = CoinModule::normalize_token_type(coin_type);
        if normalized == kanari_types::gas_coin::GAS_COIN {
            self.gas_coin_type = None;
        } else {
            self.gas_coin_type = Some(normalized);
        }
    }

    /// Record gas fees collected for the DAO in any supported coin.
    ///
    /// Native KANARI keeps the legacy ledger path (owner delta + native gas
    /// credits). For other coins, `token_gas_credits` records the sender's
    /// object debit; the engine adds an equal DAO-owned `Coin<T>` output.
    pub fn collect_gas_for_coin(
        &mut self,
        dao_address: AccountAddress,
        coin_type: &str,
        gas_amount: u64,
    ) {
        let normalized = CoinModule::normalize_token_type(coin_type);
        if normalized == kanari_types::gas_coin::GAS_COIN {
            self.get_or_create_owner_delta(dao_address)
                .credit(gas_amount);
            let collected = self.native_gas_credits.entry(dao_address).or_insert(0);
            *collected = collected.saturating_add(gas_amount);
            return;
        }
        self.set_gas_coin_type(&normalized);
        let collected = self.token_gas_credits.entry(normalized).or_insert(0);
        *collected = collected.saturating_add(gas_amount);
        // DAO address is retained in the native credit map with zero effect so
        // conflict analysis still serializes fee collection per collector.
        self.native_gas_credits.entry(dao_address).or_insert(0);
    }

    /// Total gas debited for `coin_type` in this changeset.
    pub fn gas_debit_for_coin(&self, coin_type: &str) -> u64 {
        let normalized = CoinModule::normalize_token_type(coin_type);
        if normalized == kanari_types::gas_coin::GAS_COIN {
            return self.native_gas_credits.values().copied().sum();
        }
        self.token_gas_credits
            .get(&normalized)
            .copied()
            .unwrap_or(0)
    }

    /// Sets the total gas consumed by this execution.
    pub fn set_gas_used(&mut self, gas: u64) {
        self.gas_used = gas;
    }

    /// Marks this execution as failed with the given error message.
    pub fn mark_failed(&mut self, error: String) {
        self.success = false;
        self.error_message = Some(error);
    }
}
