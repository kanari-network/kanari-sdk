// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Persisted supply records: keys, loads, and index sync.

use super::*;

impl StateManager {
    /// Build the database key for a token supply record.
    /// Always canonical so `0x02`/`0x2` spellings share one record.
    pub(crate) fn supply_key(token_type: &str) -> Vec<u8> {
        let normalized = Self::normalize_token_type(token_type);
        let mut key = b"supply:".to_vec();
        key.extend_from_slice(normalized.as_bytes());
        key
    }

    /// Raw-spelling key for backward compat with DBs written before
    /// normalization. New writes always use [`Self::supply_key`].
    fn supply_key_raw(token_type: &str) -> Vec<u8> {
        let mut key = b"supply:".to_vec();
        key.extend_from_slice(token_type.as_bytes());
        key
    }

    /// Load a persisted supply value from the store for the given token type.
    /// Tries the canonical key first, then the raw spelling.
    pub(crate) fn load_persisted_supply_from_store(
        store: &PersistentStore,
        token_type: &str,
    ) -> Result<Option<u64>> {
        let key = Self::supply_key(token_type);
        if let Some(cap) = store.load::<TreasuryCap>(&key)? {
            return Ok(Some(cap.total_supply));
        }
        if let Some(supply) = store.load::<u64>(&key)? {
            return Ok(Some(supply));
        }
        let raw_key = Self::supply_key_raw(token_type);
        if raw_key != key {
            if let Some(cap) = store.load::<TreasuryCap>(&raw_key)? {
                return Ok(Some(cap.total_supply));
            }
            if let Some(supply) = store.load::<u64>(&raw_key)? {
                return Ok(Some(supply));
            }
        }
        Ok(None)
    }

    /// Save the native total supply and update the supply record.
    pub(crate) fn save_native_total_supply(&mut self, total_supply: u64) -> Result<()> {
        self.total_supply = total_supply;
        self.save_internal(b"total_supply", &total_supply)?;

        let supply_key = Self::supply_key(GAS_COIN);
        if self
            .load_internal::<TreasuryCap>(&supply_key)
            .ok()
            .flatten()
            .is_some()
        {
            self.save_internal(&supply_key, &TreasuryCap { total_supply })?;
        } else if self
            .load_internal::<u64>(&supply_key)
            .ok()
            .flatten()
            .is_some()
        {
            self.save_internal(&supply_key, &total_supply)?;
        }

        Ok(())
    }

    /// Get the total issued supply for a token type.
    pub(crate) fn issued_supply_for_token(&self, token_type: &str) -> Result<u64> {
        let normalized = Self::normalize_token_type(token_type);
        if normalized == GAS_COIN {
            return Ok(self.total_supply);
        }

        let supply_key = Self::supply_key(&normalized);
        if let Some(cap) = self.load_internal::<TreasuryCap>(&supply_key)? {
            return Ok(cap.total_supply);
        }
        if let Some(supply) = self.load_internal::<u64>(&supply_key)? {
            return Ok(supply);
        }
        // Backward compat: DBs written with raw spelling.
        let raw_key = Self::supply_key_raw(token_type);
        if raw_key != supply_key {
            if let Some(cap) = self.load_internal::<TreasuryCap>(&raw_key)? {
                return Ok(cap.total_supply);
            }
            if let Some(supply) = self.load_internal::<u64>(&raw_key)? {
                return Ok(supply);
            }
        }
        Ok(self
            .global_token_supplies
            .get(&normalized)
            .copied()
            .unwrap_or(0))
    }

    /// Compute wallet-visible supply by scanning all owners and objects.
    pub(crate) fn indexed_wallet_supply(&self, token_type: &str) -> Result<u64> {
        let token_type = Self::normalize_token_type(token_type);
        let mut owners = self.owner_addresses()?.into_iter().collect::<BTreeSet<_>>();
        let mut object_balances = BTreeMap::<AccountAddress, u64>::new();
        // Gas fees are credited to the DAO owner ledger. The DAO may not have
        // a normal account-index entry, so include it explicitly when
        // calculating the native token's wallet-visible supply.
        if token_type == GAS_COIN {
            owners.insert(Self::dao_account_address()?);
        }
        // Aggregate matching objects during the canonical object pass. Previously this
        // pass only discovered owners and then `resolve_owner_token_balance` rescanned
        // every object owned by every discovered account.
        for (_, object) in self.query_objects(None, None, None, None, None)? {
            let Some((object_token, amount)) =
                Self::balance_token_amount(&object.type_, &object.data)
            else {
                continue;
            };
            if object_token != token_type {
                continue;
            }
            owners.insert(object.owner);
            let balance = object_balances.entry(object.owner).or_insert(0);
            *balance = balance
                .checked_add(amount)
                .require("Indexed owner token balance overflow")?;
        }

        let mut total = 0u64;
        for owner in owners {
            let object_balance = object_balances.get(&owner).copied().unwrap_or(0);
            let balance = if token_type == GAS_COIN {
                self.load_owner_state(&owner)?
                    .map(|state| state.native_balance())
                    .filter(|ledger_balance| *ledger_balance > 0)
                    .unwrap_or(object_balance)
            } else {
                object_balance
            };
            total = total
                .checked_add(balance)
                .require("Indexed wallet supply overflow")?;
        }
        Ok(total)
    }

    /// Sync the native visible supply cache with the canonical index.
    pub(crate) fn sync_native_visible_supply_cache(&mut self) -> Result<bool> {
        let indexed_visible = self.indexed_wallet_supply(GAS_COIN)?.min(self.total_supply);
        let current = self
            .global_token_supplies
            .get(GAS_COIN)
            .copied()
            .unwrap_or(0);
        if current == indexed_visible {
            return Ok(false);
        }

        if indexed_visible == 0 {
            self.global_token_supplies.remove(GAS_COIN);
        } else {
            self.global_token_supplies
                .insert(GAS_COIN.to_string(), indexed_visible);
        }
        Ok(true)
    }
}
