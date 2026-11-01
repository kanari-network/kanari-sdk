// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Supply invariant validation (full and cached fast paths).

use super::*;

impl StateManager {
    /// Reject a cached native total supply that disagrees with the persisted
    /// treasury. Both the full and the fast invariant checks open with this,
    /// so the canonical total is established once.
    fn validate_persisted_native_supply(&self) -> Result<()> {
        let supply_key = Self::supply_key(GAS_COIN);
        let persisted_native_supply =
            if let Some(cap) = self.load_internal::<TreasuryCap>(&supply_key)? {
                Some(cap.total_supply)
            } else {
                self.load_internal::<u64>(&supply_key)?
            };
        if let Some(persisted) = persisted_native_supply
            && persisted != self.total_supply
        {
            anyhow::bail!(
                "native total supply mismatch: state.total_supply={} persisted_treasury={}",
                self.total_supply,
                persisted
            );
        }
        Ok(())
    }

    fn tracked_fungible_token_types(&self) -> Result<BTreeSet<String>> {
        let mut token_types = self
            .global_token_supplies
            .keys()
            .map(|token_type| CoinModule::normalize_token_type(token_type))
            .collect::<BTreeSet<_>>();
        token_types.insert(GAS_COIN.to_string());

        if let Some(entries) = self.load_internal::<Vec<String>>(b"treasury_index")? {
            for entry in entries {
                let token_type = entry.strip_prefix("treasury:").unwrap_or(&entry);
                token_types.insert(CoinModule::normalize_token_type(token_type));
            }
        }

        for record in self.load_object_locked_coin_records()? {
            token_types.insert(CoinModule::normalize_token_type(&record.token_type));
        }

        Ok(token_types)
    }

    fn validate_token_supply_bound(
        token_type: &str,
        total_supply: u64,
        wallet_visible_supply: u64,
        object_locked_supply: u64,
    ) -> Result<()> {
        let accounted_supply = wallet_visible_supply
            .checked_add(object_locked_supply)
            .require("Accounted token supply overflow")?;
        let label = if token_type == GAS_COIN {
            "native"
        } else {
            "fungible token"
        };
        ensure!(
            wallet_visible_supply <= total_supply && accounted_supply <= total_supply,
            "{} supply overcount: token_type={} total_supply={} accounted_supply={} wallet_visible_supply={} object_locked_supply={}",
            label,
            token_type,
            total_supply,
            accounted_supply,
            wallet_visible_supply,
            object_locked_supply
        );
        Ok(())
    }

    /// Validate supply bounds for every known fungible token using canonical object
    /// state.
    pub fn validate_supply_invariants(&self) -> Result<()> {
        self.validate_persisted_native_supply()?;

        let canonical_index = self.compute_wallet_supply_index()?;
        let mut token_types = self.tracked_fungible_token_types()?;
        token_types.extend(
            canonical_index
                .keys()
                .map(|token_type| CoinModule::normalize_token_type(token_type)),
        );

        for token_type in token_types {
            let summary = self.token_supply_summary(&token_type)?;
            Self::validate_token_supply_bound(
                &token_type,
                summary.total_supply,
                summary.wallet_visible_supply,
                summary.object_locked_supply,
            )?;

            let canonical_visible = canonical_index.get(&token_type).copied().unwrap_or(0);
            ensure!(
                canonical_visible == summary.wallet_visible_supply,
                "wallet supply index mismatch: token_type={} indexed={} cached={}",
                token_type,
                canonical_visible,
                summary.wallet_visible_supply
            );
        }

        ensure!(
            canonical_index == self.global_token_supplies,
            "wallet supply index mismatch for one or more token types"
        );

        Ok(())
    }

    /// Fast transaction-path validation after the wallet-visible cache has been
    /// reconciled. Full checkpoint/RPC validation still derives balances from
    /// canonical owner/object indexes via `validate_supply_invariants`.
    pub fn validate_token_supply_bound_for_test(
        total: u64,
        wallet: u64,
        locked: u64,
    ) -> Result<()> {
        Self::validate_token_supply_bound("test", total, wallet, locked)
    }

    /// Fast transaction-path validation after the wallet-visible cache has been
    /// reconciled. Full checkpoint/RPC validation still derives balances from
    /// canonical owner/object indexes via `validate_supply_invariants`.
    pub fn validate_cached_supply_invariants(&self) -> Result<()> {
        let canonical_index = self.compute_wallet_supply_index()?;
        let mut token_types = self.tracked_fungible_token_types()?;
        token_types.extend(
            canonical_index
                .keys()
                .map(|token_type| CoinModule::normalize_token_type(token_type)),
        );

        for token_type in token_types {
            let total_supply = self.issued_supply_for_token(&token_type)?;
            let wallet_visible_supply = self
                .global_token_supplies
                .get(&token_type)
                .copied()
                .unwrap_or(0);
            let object_locked_supply = self.object_locked_supply_for_token(&token_type)?;
            Self::validate_token_supply_bound(
                &token_type,
                total_supply,
                wallet_visible_supply,
                object_locked_supply,
            )?;
        }

        Ok(())
    }
}
