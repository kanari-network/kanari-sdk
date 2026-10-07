// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Per-token holder index (incremental membership + one-time rebuild).

use super::*;

impl StateManager {
    /// Version of the per-token holders index. Bump to force a one-time
    /// rebuild (stale canonical keys are simply orphaned; lookups always use
    /// canonical keys, so orphans are never read).
    pub(crate) const TOKEN_HOLDERS_INDEX_VERSION: u32 = 1;
    const TOKEN_HOLDERS_INDEX_VERSION_KEY: &[u8] = b"runtime:token_holders_index_version";

    fn holders_key(token_type: &str) -> Vec<u8> {
        let mut key = b"holders:".to_vec();
        key.extend_from_slice(token_type.as_bytes());
        key
    }

    fn holder_amount(
        balances: &BTreeMap<String, BalanceRecord>,
        token_type: &str,
        normalized: &str,
    ) -> u64 {
        balances
            .get(token_type)
            .or_else(|| {
                if normalized != token_type {
                    balances.get(normalized)
                } else {
                    None
                }
            })
            .map(|b| b.value())
            .unwrap_or(0)
    }

    /// Load the indexed holder set for a token. Callers must check
    /// [`Self::token_holder_index_ready`] first: when the index hasn't been
    /// built yet, a missing key is meaningless and the caller must fall back
    /// to a full scan. Once built, a missing key means no recorded holders.
    pub fn token_holder_set(&self, token_type: &str) -> Result<BTreeSet<AccountAddress>> {
        let normalized = Self::normalize_token_type(token_type);
        Ok(self
            .load_internal::<BTreeSet<AccountAddress>>(&Self::holders_key(&normalized))?
            .unwrap_or_default())
    }

    /// Whether the holders index is built. Read paths use this to decide
    /// between the index and a full scan.
    pub fn token_holder_index_ready(&self) -> Result<bool> {
        Ok(self
            .load_internal::<u32>(Self::TOKEN_HOLDERS_INDEX_VERSION_KEY)?
            .unwrap_or(0)
            == Self::TOKEN_HOLDERS_INDEX_VERSION)
    }

    /// Incrementally maintain the holders index from an owner's balance delta.
    /// Idempotent: re-applying the same delta is a no-op. Skipped entirely
    /// until [`Self::ensure_token_holders_index`] has built the index.
    pub(crate) fn update_holder_index_for_delta(
        &mut self,
        owner: AccountAddress,
        old_balances: &BTreeMap<String, BalanceRecord>,
        new_balances: &BTreeMap<String, BalanceRecord>,
    ) -> Result<()> {
        if !self.token_holder_index_ready()? {
            return Ok(());
        }
        let mut tokens = BTreeSet::new();
        tokens.extend(old_balances.keys().cloned());
        tokens.extend(new_balances.keys().cloned());
        for token in tokens {
            let normalized = Self::normalize_token_type(&token);
            let new_amount = Self::holder_amount(new_balances, &token, &normalized);
            let key = Self::holders_key(&normalized);
            if new_amount > 0 {
                let mut set: BTreeSet<AccountAddress> =
                    self.load_internal(&key)?.unwrap_or_default();
                if set.insert(owner) {
                    self.save_internal(&key, &set)?;
                }
            } else {
                if let Some(mut set) = self.load_internal::<BTreeSet<AccountAddress>>(&key)?
                    && set.remove(&owner)
                {
                    self.save_internal(&key, &set)?;
                }
                // Also drop a legacy raw-spelling entry if one exists.
                if normalized != token {
                    let raw_key = Self::holders_key(&token);
                    if let Some(mut set) =
                        self.load_internal::<BTreeSet<AccountAddress>>(&raw_key)?
                        && set.remove(&owner)
                    {
                        self.save_internal(&raw_key, &set)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// One-time rebuild of the holders index from canonical state. Returns
    /// `true` when a build happened. One pass over owners; each owner's
    /// balances resolve from their owned objects plus ledger.
    pub(crate) fn ensure_token_holders_index(&mut self) -> Result<bool> {
        if self.token_holder_index_ready()? {
            return Ok(false);
        }
        let mut per_token: BTreeMap<String, BTreeSet<AccountAddress>> = BTreeMap::new();
        for owner in self.owner_addresses()? {
            for (token, amount) in self.resolve_owner_token_balances(owner)? {
                if amount > 0 {
                    per_token.entry(token).or_default().insert(owner);
                }
            }
        }
        for (token, set) in &per_token {
            self.save_internal(&Self::holders_key(token), set)?;
        }
        self.save_internal(
            Self::TOKEN_HOLDERS_INDEX_VERSION_KEY,
            &Self::TOKEN_HOLDERS_INDEX_VERSION,
        )?;
        Ok(true)
    }
}
