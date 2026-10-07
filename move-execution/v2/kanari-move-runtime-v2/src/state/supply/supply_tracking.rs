// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Supply cache tracking: invariant flags, normalization, and global deltas.

use super::*;

impl StateManager {
    /// Check whether supply invariant violations should fail fast.
    ///
    /// Fail-closed everywhere by default: a corrupt supply must never boot
    /// or commit silently on any network. Explicit opt-out only via
    /// `KANARI_FAIL_FAST_ON_SUPPLY_MISMATCH=0/false/no/off` (for emergency
    /// recovery of a legacy-corrupt DB, then re-enable).
    pub fn supply_invariant_fail_fast_enabled() -> bool {
        std::env::var("KANARI_FAIL_FAST_ON_SUPPLY_MISMATCH")
            .map(|value| {
                !matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "0" | "false" | "no" | "off"
                )
            })
            .unwrap_or(true)
    }

    /// Log and optionally bail on a supply invariant violation.
    pub(crate) fn report_supply_invariant_violation(
        context: &str,
        error: &anyhow::Error,
    ) -> Result<()> {
        log::error!(
            "[StateManager] Supply invariant check failed {}: {}",
            context,
            error
        );

        if Self::supply_invariant_fail_fast_enabled() {
            anyhow::bail!("Supply invariant check failed {}: {}", context, error);
        }

        Ok(())
    }
    /// Normalize a token type string to its canonical display form.
    pub(crate) fn normalize_token_type(token_type: &str) -> String {
        if let Ok(TypeTag::Struct(st)) = TypeTag::from_str(token_type) {
            return format!("{}", st);
        }
        token_type.to_string()
    }

    /// Adjust the global token supply cache based on an owner's old and new balances.
    pub(crate) fn adjust_global_supplies_for_account_delta(
        &mut self,
        old_balances: &BTreeMap<String, BalanceRecord>,
        new_balances: &BTreeMap<String, BalanceRecord>,
    ) -> Result<bool> {
        let mut changed = false;
        let mut tokens = BTreeSet::new();
        tokens.extend(old_balances.keys().cloned());
        tokens.extend(new_balances.keys().cloned());

        for token_type in tokens {
            let old_amount = old_balances
                .get(&token_type)
                .map(|x| x.value())
                .unwrap_or(0);
            let new_amount = new_balances
                .get(&token_type)
                .map(|x| x.value())
                .unwrap_or(0);

            if old_amount == new_amount {
                continue;
            }

            changed = true;
            let current_supply = self
                .global_token_supplies
                .get(&token_type)
                .copied()
                .unwrap_or(0);

            let updated_supply = if new_amount >= old_amount {
                current_supply
                    .checked_add(new_amount - old_amount)
                    .require("Wallet-visible token supply overflow")?
            } else {
                current_supply
                    .checked_sub(old_amount - new_amount)
                    .require("Wallet-visible token supply underflow")?
            };

            if updated_supply == 0 {
                self.global_token_supplies.remove(&token_type);
            } else {
                self.global_token_supplies
                    .insert(token_type, updated_supply);
            }
        }

        Ok(changed)
    }

    /// Update the global supply cache after an owner's token balances changed.
    pub(crate) fn capture_supply_changed(
        &mut self,
        account: &OwnerState,
        old_balances: &BTreeMap<String, BalanceRecord>,
    ) -> Result<bool> {
        let changed =
            self.adjust_global_supplies_for_account_delta(old_balances, &account.token_balances)?;
        self.update_holder_index_for_delta(account.address, old_balances, &account.token_balances)?;
        Ok(changed)
    }

    /// Recompute token balances for an owner from their owned objects and ledger delta.
    pub(crate) fn recompute_token_balances_for_owner(
        &mut self,
        owner: AccountAddress,
        native_delta: i128,
        native_object_changed: bool,
        native_gas_credit: u64,
        native_ledger_before: u64,
        native_object_before: u64,
    ) -> Result<bool> {
        let mut owner_state = self.load_owner_state_or_default(owner)?;
        let old_balances = owner_state.token_balances.clone();
        let native_balance_after_owner_deltas = owner_state.native_balance();
        let mut aggregated = self.compute_owned_token_balances(owner, None)?;

        let native_object_balance = aggregated.remove(GAS_COIN);
        owner_state.token_balances = aggregated
            .into_iter()
            .map(|(token_type, amount)| (token_type, BalanceRecord::new(amount)))
            .collect();

        let native_balance = if let Some(object_balance) = native_object_balance {
            let prior_non_object_balance =
                native_ledger_before.saturating_sub(native_object_before);
            let object_backed_with_ledger = object_balance
                .checked_add(prior_non_object_balance)
                .and_then(|amount| amount.checked_add(native_gas_credit))
                .require("Native owner balance overflow during object reconciliation")?;
            if native_delta > 0 {
                // Only gas credits are guaranteed not to be represented by
                // the Move Coin objects. Other positive owner deltas may be
                // mint/transfer effects already reflected in object_balance.
                object_backed_with_ledger.max(native_balance_after_owner_deltas)
            } else if native_delta < 0 {
                // Keep the canonical object balance when it already reflects
                // the debit; otherwise use the owner ledger's debited value.
                object_backed_with_ledger.min(native_balance_after_owner_deltas)
            } else {
                object_backed_with_ledger
            }
        } else if native_object_changed {
            0
        } else {
            native_balance_after_owner_deltas
        };
        owner_state.set_token_balance_value(GAS_COIN, native_balance);
        self.save_owner_state_without_supply_index(&owner_state)?;

        self.capture_supply_changed(&owner_state, &old_balances)
    }
}
