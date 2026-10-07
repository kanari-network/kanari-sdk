// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Wallet-visible supply index and per-owner balance resolution.

use super::*;

impl StateManager {
    pub(crate) fn dao_account_address() -> Result<AccountAddress> {
        AccountAddress::from_hex_literal(kanari_types::address::Address::DAO_ADDRESS)
            .context("DAO_ADDRESS constant is not a valid Move account address")
    }

    pub(crate) fn compute_wallet_supply_index(&self) -> Result<BTreeMap<String, u64>> {
        let mut object_balances = BTreeMap::<(AccountAddress, String), u64>::new();
        let mut owners = self.owner_addresses()?.into_iter().collect::<BTreeSet<_>>();
        owners.insert(Self::dao_account_address()?);

        for (_, object) in self.query_objects(None, None, None, None, None)? {
            let Some((token_type, amount)) =
                Self::balance_token_amount(&object.type_, &object.data)
            else {
                continue;
            };
            owners.insert(object.owner);
            let key = (object.owner, token_type);
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
    pub(crate) fn ensure_wallet_supply_index(&mut self) -> Result<bool> {
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

            let Some((token_type, amount)) = Self::balance_token_amount(&obj.type_, &obj.data)
            else {
                continue;
            };

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
}
