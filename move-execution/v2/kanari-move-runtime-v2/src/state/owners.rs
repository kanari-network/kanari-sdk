// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Owner state records and lookups.

use super::*;

impl StateManager {
    // Helper to construct DB key for one owner-state record.
    pub(crate) fn owner_state_key(address: &AccountAddress) -> Vec<u8> {
        let mut key = b"account:".to_vec();
        key.extend_from_slice(address.as_ref());
        key
    }

    pub(crate) fn save_owner_record(&mut self, owner_state: &OwnerState) -> Result<()> {
        self.save_internal(&Self::owner_state_key(&owner_state.address), owner_state)
    }

    pub(crate) fn add_many_to_index_list<I>(&mut self, key: &[u8], values: I) -> Result<()>
    where
        I: IntoIterator<Item = String>,
    {
        let mut values = values.into_iter().peekable();
        if values.peek().is_none() {
            return Ok(());
        }

        let existing = self.load_index_list(key)?;
        let mut index: BTreeSet<String> = existing.into_iter().collect();
        let mut changed = false;

        for value in values {
            changed |= index.insert(value);
        }

        if changed {
            let sorted = index.into_iter().collect::<Vec<_>>();
            self.save_index_list(key, &sorted)?;
        }

        Ok(())
    }

    /// Get all registered owner addresses.
    pub fn owner_addresses(&self) -> Result<Vec<AccountAddress>> {
        let ids = self.load_owner_index_ids()?;
        ids.into_iter()
            .map(|id| {
                AccountAddress::from_hex_literal(&id)
                    .with_context(|| format!("Owner index contains invalid address {id}"))
            })
            .collect()
    }

    pub(crate) fn load_owner_index_ids(&self) -> Result<Vec<String>> {
        let owner_ids = self.load_index_list(OWNER_INDEX_KEY)?;
        if !owner_ids.is_empty() {
            return Ok(owner_ids);
        }

        self.load_index_list(LEGACY_ACCOUNT_INDEX_KEY)
    }

    pub(crate) fn load_owner_state_or_default(
        &self,
        address: AccountAddress,
    ) -> Result<OwnerState> {
        Ok(self
            .load_owner_state(&address)?
            .unwrap_or_else(|| OwnerState::new(address)))
    }

    /// Load the persisted owner state for the given address.
    pub fn load_owner_state(&self, owner: &AccountAddress) -> Result<Option<OwnerState>> {
        self.load_internal(&Self::owner_state_key(owner))
    }

    pub(crate) fn save_owner_state_without_supply_index(
        &mut self,
        owner_state: &OwnerState,
    ) -> Result<()> {
        if owner_state.is_empty() {
            self.overlay
                .insert(Self::owner_state_key(&owner_state.address), None);
            self.remove_from_index_list(OWNER_INDEX_KEY, &owner_state.address.to_hex_literal())
        } else {
            self.save_owner_record(owner_state)?;
            self.add_to_index_list(OWNER_INDEX_KEY, owner_state.address.to_hex_literal())
        }
    }

    /// Save owner state and update the supply index cache.
    pub fn save_owner_state(&mut self, owner_state: &OwnerState) -> Result<()> {
        let old_balances = self
            .load_owner_state(&owner_state.address)?
            .map(|state| state.token_balances)
            .unwrap_or_default();
        self.save_owner_state_without_supply_index(owner_state)?;
        let old_native = old_balances
            .get(GAS_COIN)
            .map(|balance| balance.value())
            .unwrap_or(0);
        let new_native = owner_state.native_balance();
        let cached_native = self
            .global_token_supplies
            .get(GAS_COIN)
            .copied()
            .unwrap_or(0);
        let projected_native =
            u128::from(cached_native.saturating_sub(old_native)) + u128::from(new_native);
        let update_supply_index =
            old_native == new_native || projected_native <= u128::from(self.total_supply);
        if update_supply_index && self.capture_supply_changed(owner_state, &old_balances)? {
            let supplies = self.global_token_supplies.clone();
            self.save_internal(b"global_token_supplies", &supplies)?;
        }
        if !update_supply_index {
            // Supply cache gated off, but holder membership still follows
            // owner_state (capture_supply_changed already covered the other branch).
            self.update_holder_index_for_delta(
                owner_state.address,
                &old_balances,
                &owner_state.token_balances,
            )?;
        }
        Ok(())
    }

    /// Get the owner state, returning None on error instead of propagating.
    pub fn get_owner_state(&self, owner: &AccountAddress) -> Option<OwnerState> {
        match self.load_owner_state(owner) {
            Ok(account) => account,
            Err(e) => {
                log::error!(
                    "[StateManager] Failed to load owner state {}: {}",
                    owner.to_hex_literal(),
                    e
                );
                None
            }
        }
    }

    /// Look up owner state by hex address, returning an error on parse failure.
    pub fn try_get_owner_state_by_hex(&self, owner: &str) -> Result<Option<OwnerState>> {
        let addr = kanari_types::address::Address::parse_to_account_address(owner)
            .with_context(|| format!("Failed to parse owner address: {owner}"))?;
        self.load_owner_state(&addr)
            .with_context(|| format!("Failed to load owner state for {owner}"))
    }

    /// Look up owner state by hex address, returning None on any error.
    pub fn get_owner_state_by_hex(&self, owner: &str) -> Option<OwnerState> {
        // Use Address::parse_to_account_address which handles tagged addresses,
        // tagged public keys (hashing), and regular 0x addresses.
        match self.try_get_owner_state_by_hex(owner) {
            Ok(state) => state,
            Err(error) => {
                log::warn!("[StateManager] Failed to get owner state by hex: {error:#}");
                None
            }
        }
    }
}
