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

    /// Validate that all supply invariants hold across persisted and cached state.
    pub fn validate_supply_invariants(&self) -> Result<()> {
        self.validate_persisted_native_supply()?;

        let native_supply = self.token_supply_summary(GAS_COIN)?;
        // Wallet-visible balance caches only reflect top-level wallet-owned
        // coin objects. Coins can also be held inside DeFi objects (for
        // example escrow funds), so visible supply may be lower than issued
        // supply without implying a burn. It must never exceed total supply.
        if native_supply.wallet_visible_supply > native_supply.total_supply {
            anyhow::bail!(
                "native supply overcount: total_supply={} wallet_visible_supply={} object_locked_supply={}",
                native_supply.total_supply,
                native_supply.wallet_visible_supply,
                native_supply.object_locked_supply
            );
        }
        if native_supply.accounted_supply > native_supply.total_supply {
            anyhow::bail!(
                "native supply overcount: total_supply={} accounted_supply={} wallet_visible_supply={} object_locked_supply={}",
                native_supply.total_supply,
                native_supply.accounted_supply,
                native_supply.wallet_visible_supply,
                native_supply.object_locked_supply
            );
        }

        let canonical_index = self.compute_wallet_supply_index()?;
        let canonical_visible = canonical_index.get(GAS_COIN).copied().unwrap_or(0);
        ensure!(
            canonical_visible == native_supply.wallet_visible_supply,
            "native wallet supply index mismatch: indexed={} cached={}",
            canonical_visible,
            native_supply.wallet_visible_supply
        );
        ensure!(
            canonical_index == self.global_token_supplies,
            "wallet supply index mismatch for one or more token types"
        );

        Ok(())
    }

    /// Fast transaction-path validation after the wallet-visible cache has been
    /// reconciled. Full checkpoint/RPC validation still derives balances from
    /// canonical owner/object indexes via `validate_supply_invariants`.
    pub fn validate_cached_supply_invariants(&self) -> Result<()> {
        self.validate_persisted_native_supply()?;

        let wallet_visible_supply = self
            .global_token_supplies
            .get(GAS_COIN)
            .copied()
            .unwrap_or(0);
        let object_locked_supply = self.object_locked_supply_for_token(GAS_COIN)?;
        let accounted_supply = wallet_visible_supply
            .checked_add(object_locked_supply)
            .require("Accounted native supply overflow")?;
        ensure!(
            wallet_visible_supply <= self.total_supply && accounted_supply <= self.total_supply,
            "native supply overcount: total_supply={} accounted_supply={} wallet_visible_supply={} object_locked_supply={}",
            self.total_supply,
            accounted_supply,
            wallet_visible_supply,
            object_locked_supply
        );
        Ok(())
    }
}
