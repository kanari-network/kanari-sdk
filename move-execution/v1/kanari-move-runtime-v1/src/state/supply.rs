//! Token supply tracking and invariant validation.

use super::*;

impl StateManager {
    fn dao_account_address() -> Result<AccountAddress> {
        AccountAddress::from_hex_literal(kanari_types::address::Address::DAO_ADDRESS)
            .context("DAO_ADDRESS constant is not a valid Move account address")
    }

    fn compute_wallet_supply_index(&self) -> Result<BTreeMap<String, u64>> {
        let mut object_balances = BTreeMap::<(AccountAddress, String), u64>::new();
        let mut owners = self.owner_addresses()?.into_iter().collect::<BTreeSet<_>>();
        owners.insert(Self::dao_account_address()?);

        for (_, object) in self.query_objects(None, None, None, None, None)? {
            let Ok(struct_tag) = StructTag::from_str(&object.type_) else {
                continue;
            };
            if !Self::is_balance_struct(&struct_tag) {
                continue;
            }
            let Some(token_type) = Self::token_type_from_balance_struct(&struct_tag) else {
                continue;
            };
            let Some(amount) = Self::extract_balance_from_object_bytes(&object.data, &struct_tag)
            else {
                continue;
            };
            owners.insert(object.owner);
            let key = (object.owner, Self::normalize_token_type(&token_type));
            let balance = object_balances.entry(key).or_insert(0);
            *balance = balance
                .checked_add(amount)
                .require("Wallet supply index owner balance overflow")?;
        }

        let mut totals = BTreeMap::<String, u64>::new();
        for ((_, token_type), amount) in &object_balances {
            if token_type != GAS_COIN {
                let total = totals.entry(token_type.clone()).or_insert(0);
                *total = total
                    .checked_add(*amount)
                    .require("Wallet supply index token total overflow")?;
            }
        }
        for owner in owners {
            let ledger = self
                .load_owner_state(&owner)?
                .map(|state| state.native_balance())
                .unwrap_or(0);
            let object = object_balances
                .get(&(owner, GAS_COIN.to_string()))
                .copied()
                .unwrap_or(0);
            let amount = if ledger > 0 { ledger } else { object };
            if amount > 0 {
                let total = totals.entry(GAS_COIN.to_string()).or_insert(0);
                *total = total
                    .checked_add(amount)
                    .require("Native wallet supply index overflow")?;
            }
        }
        Ok(totals)
    }

    /// Rebuild the wallet-visible supply index from canonical owner and object data.
    pub(super) fn ensure_wallet_supply_index(&mut self) -> Result<bool> {
        let version = self
            .load_internal::<u32>(WALLET_SUPPLY_INDEX_VERSION_KEY)?
            .unwrap_or(0);
        ensure!(
            version <= WALLET_SUPPLY_INDEX_VERSION,
            "Wallet supply index version {} is newer than runtime version {}",
            version,
            WALLET_SUPPLY_INDEX_VERSION
        );
        if version == WALLET_SUPPLY_INDEX_VERSION {
            return Ok(false);
        }
        self.global_token_supplies = self.compute_wallet_supply_index()?;
        let supplies = self.global_token_supplies.clone();
        self.save_internal(b"global_token_supplies", &supplies)?;
        self.save_internal(
            WALLET_SUPPLY_INDEX_VERSION_KEY,
            &WALLET_SUPPLY_INDEX_VERSION,
        )?;
        Ok(true)
    }

    /// Resolve all token balances for an owner from owned objects and ledger state.
    pub fn resolve_owner_token_balances(
        &self,
        owner: AccountAddress,
    ) -> Result<BTreeMap<String, u64>> {
        let owner_state = self.load_owner_state(&owner)?;
        let native_ledger_balance = owner_state
            .as_ref()
            .map(OwnerState::native_balance)
            .filter(|balance| *balance > 0);
        let mut balances = self.compute_owned_token_balances(owner, native_ledger_balance)?;

        if let Some(owner_state) = owner_state {
            let native_balance = owner_state.native_balance();
            if native_balance > 0 {
                balances
                    .entry(GAS_COIN.to_string())
                    .or_insert(native_balance);
            }
        }

        Ok(balances)
    }

    /// Resolve the balance of a specific token type for an owner.
    pub fn resolve_owner_token_balance(
        &self,
        owner: AccountAddress,
        token_type: &str,
    ) -> Result<u64> {
        let token_type = Self::normalize_token_type(token_type);
        Ok(self
            .resolve_owner_token_balances(owner)?
            .get(&token_type)
            .copied()
            .unwrap_or(0))
    }

    /// Resolve the native token balance for an owner.
    pub fn resolve_owner_native_balance(&self, owner: AccountAddress) -> Result<u64> {
        self.resolve_owner_token_balance(owner, GAS_COIN)
    }

    /// Compute token balances for an owner by scanning their owned objects.
    pub fn compute_owned_token_balances(
        &self,
        owner: AccountAddress,
        native_ledger_balance: Option<u64>,
    ) -> Result<BTreeMap<String, u64>> {
        let mut aggregated: BTreeMap<String, u64> = BTreeMap::new();
        let mut seen_objects = BTreeSet::new();

        for object_id in self.get_owned_objects(&owner)? {
            let canonical_id = canonical_object_id(&object_id).unwrap_or(object_id.clone());
            if !seen_objects.insert(canonical_id) {
                continue;
            }
            let Some(obj) = self.get_object(&object_id)? else {
                continue;
            };

            let Ok(struct_tag) = StructTag::from_str(&obj.type_) else {
                continue;
            };

            if !Self::is_balance_struct(&struct_tag) {
                continue;
            }

            let Some(amount) = Self::extract_balance_from_object_bytes(&obj.data, &struct_tag)
            else {
                continue;
            };

            let Some(token_type) = Self::token_type_from_balance_struct(&struct_tag) else {
                continue;
            };

            let token_type = Self::normalize_token_type(&token_type);
            let entry = aggregated.entry(token_type).or_insert(0);
            *entry = entry
                .checked_add(amount)
                .require("Owned token balance overflow")?;
        }

        if let Some(native_balance) = native_ledger_balance {
            aggregated.insert(GAS_COIN.to_string(), native_balance);
        }

        Ok(aggregated)
    }

    /// Build the database key for a token supply record.
    /// Always canonical so `0x02`/`0x2` spellings share one record.
    pub(super) fn supply_key(token_type: &str) -> Vec<u8> {
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
    pub(super) fn load_persisted_supply_from_store(
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
    pub(super) fn save_native_total_supply(&mut self, total_supply: u64) -> Result<()> {
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
    pub(super) fn issued_supply_for_token(&self, token_type: &str) -> Result<u64> {
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
    pub(super) fn indexed_wallet_supply(&self, token_type: &str) -> Result<u64> {
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
            let Ok(struct_tag) = StructTag::from_str(&object.type_) else {
                continue;
            };
            if !Self::is_balance_struct(&struct_tag)
                || !Self::token_type_from_balance_struct(&struct_tag).is_some_and(|object_token| {
                    Self::normalize_token_type(&object_token) == token_type
                })
            {
                continue;
            }
            let Some(amount) = Self::extract_balance_from_object_bytes(&object.data, &struct_tag)
            else {
                continue;
            };
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
    pub(super) fn sync_native_visible_supply_cache(&mut self) -> Result<bool> {
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
    pub(super) fn load_object_locked_coin_records(&self) -> Result<Vec<ObjectLockedCoinRecord>> {
        Ok(self
            .load_internal(OBJECT_LOCKED_COIN_RECORDS_KEY)?
            .unwrap_or_default())
    }

    /// Save the object-locked coin records to the store.
    pub(super) fn save_object_locked_coin_records(
        &mut self,
        records: &[ObjectLockedCoinRecord],
    ) -> Result<()> {
        self.save_internal(OBJECT_LOCKED_COIN_RECORDS_KEY, records)
    }

    fn object_locked_supply_for_token(&self, token_type: &str) -> Result<u64> {
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
                let Ok(struct_tag) = StructTag::from_str(&object.type_) else {
                    continue;
                };
                if !Self::is_balance_struct(&struct_tag) {
                    continue;
                }
                let Some(object_token) = Self::token_type_from_balance_struct(&struct_tag) else {
                    continue;
                };
                let object_token = Self::normalize_token_type(&object_token);
                if !wanted.contains(object_token.as_str()) {
                    continue;
                }
                let Some(amount) =
                    Self::extract_balance_from_object_bytes(&object.data, &struct_tag)
                else {
                    continue;
                };
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

    /// Check whether supply invariant violations should fail fast.
    pub fn supply_invariant_fail_fast_enabled() -> bool {
        std::env::var("KANARI_FAIL_FAST_ON_SUPPLY_MISMATCH")
            .map(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or_else(|_| {
                matches!(
                    std::env::var("KANARI_NETWORK")
                        .unwrap_or_else(|_| "testnet".to_string())
                        .trim()
                        .to_ascii_lowercase()
                        .as_str(),
                    "mainnet"
                )
            })
    }

    /// Log and optionally bail on a supply invariant violation.
    pub(super) fn report_supply_invariant_violation(
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
    fn save_token_metadata_field<T: Serialize + ?Sized>(
        &mut self,
        prefix: &[u8],
        token_type: &str,
        value: &T,
    ) -> Result<()> {
        // Always key metadata by the canonical token type so lookups never
        // miss due to `0x02` vs `0x2` style spellings.
        let normalized = Self::normalize_token_type(token_type);
        let key = metadata_key(prefix, &normalized);
        self.save_internal(&key, value)
    }

    fn load_token_metadata_field<T: DeserializeOwned>(
        &self,
        prefix: &[u8],
        token_type: &str,
    ) -> Result<Option<T>> {
        // Try canonical key first, then the raw spelling for DBs written
        // before normalization was enforced. A corrupt entry (key holds
        // another type's bytes) is treated as unknown, never fatal: metadata
        // is best-effort display data, and every caller already degrades to
        // None. Storage errors still propagate via save paths.
        let normalized = Self::normalize_token_type(token_type);
        let key = metadata_key(prefix, &normalized);
        match self.load_internal::<T>(&key) {
            Ok(Some(value)) => return Ok(Some(value)),
            Ok(None) => {}
            Err(_) => return Ok(None),
        }
        if normalized != token_type {
            let raw_key = metadata_key(prefix, token_type);
            match self.load_internal::<T>(&raw_key) {
                Ok(value) => return Ok(value),
                Err(_) => return Ok(None),
            }
        }
        Ok(None)
    }
    /// Normalize a token type string to its canonical display form.
    pub(super) fn normalize_token_type(token_type: &str) -> String {
        if let Ok(TypeTag::Struct(st)) = TypeTag::from_str(token_type) {
            return format!("{}", st);
        }
        token_type.to_string()
    }
    /// Persist CoinMetadata fields for the given token type.
    pub(super) fn persist_coin_metadata(&mut self, token_type: &str, data: &[u8]) -> Result<()> {
        #[derive(Deserialize)]
        struct MoveString {
            bytes: Vec<u8>,
        }
        #[derive(Deserialize)]
        struct MoveUrl {
            inner: MoveString,
        }
        #[derive(Deserialize)]
        struct MoveOption<T> {
            vec: Vec<T>,
        }
        #[derive(Deserialize)]
        struct ParsedCoinMetadata {
            id: AccountAddress,
            decimals: u8,
            // On-chain order is `name` then `symbol`
            // (see kanari_system::coin::CoinMetadata).
            name: MoveString,
            symbol: MoveString,
            description: MoveString,
            icon_url: MoveOption<MoveUrl>,
        }

        if let Ok(meta) = bcs::from_bytes::<ParsedCoinMetadata>(data) {
            // Required for canonical BCS layout; metadata is keyed by token type.
            let _ = meta.id;
            self.save_token_metadata_field(b"metadata_decimals:", token_type, &meta.decimals)?;

            if let Ok(name) = String::from_utf8(meta.name.bytes) {
                self.save_token_metadata_field(b"metadata_name:", token_type, &name)?;
            }
            if let Ok(symbol) = String::from_utf8(meta.symbol.bytes) {
                self.save_token_metadata_field(b"metadata_symbol:", token_type, &symbol)?;
            }
            if let Ok(description) = String::from_utf8(meta.description.bytes) {
                self.save_token_metadata_field(b"metadata_description:", token_type, &description)?;
            }
            if let Some(url_obj) = meta.icon_url.vec.into_iter().next()
                && let Ok(url) = String::from_utf8(url_obj.inner.bytes)
            {
                self.save_token_metadata_field(b"metadata_icon_url:", token_type, &url)?;
            }
        } else if data.len() > 32 {
            // Validate: Move caps decimals at 9; never persist a corrupt byte.
            let decimals = data[32];
            if decimals <= 9 {
                self.save_token_metadata_field(b"metadata_decimals:", token_type, &decimals)?;
            }
        }
        Ok(())
    }

    /// Adjust the global token supply cache based on an owner's old and new balances.
    pub(super) fn adjust_global_supplies_for_account_delta(
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
    pub(super) fn capture_supply_changed(
        &mut self,
        account: &OwnerState,
        old_balances: &BTreeMap<String, BalanceRecord>,
    ) -> Result<bool> {
        self.adjust_global_supplies_for_account_delta(old_balances, &account.token_balances)
    }

    /// Recompute token balances for an owner from their owned objects and ledger delta.
    pub(super) fn recompute_token_balances_for_owner(
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

    /// Get token decimals for a specific token type
    pub fn get_token_decimals(&self, token_type: &str) -> Result<Option<u8>> {
        self.load_token_metadata_field(b"metadata_decimals:", token_type)
    }

    ///  Get token name for a specific token type
    pub fn get_token_name(&self, token_type: &str) -> Result<Option<String>> {
        self.load_token_metadata_field(b"metadata_name:", token_type)
    }

    ///  Get token symbol for a specific token type
    pub fn get_token_symbol(&self, token_type: &str) -> Result<Option<String>> {
        self.load_token_metadata_field(b"metadata_symbol:", token_type)
    }

    /// Get token description for a specific token type
    pub fn get_token_description(&self, token_type: &str) -> Result<Option<String>> {
        self.load_token_metadata_field(b"metadata_description:", token_type)
    }

    /// Get token icon URL for a specific token type
    pub fn get_token_icon_url(&self, token_type: &str) -> Result<Option<String>> {
        self.load_token_metadata_field(b"metadata_icon_url:", token_type)
    }

    /// Validate that all supply invariants hold across persisted and cached state.
    pub fn validate_supply_invariants(&self) -> Result<()> {
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
