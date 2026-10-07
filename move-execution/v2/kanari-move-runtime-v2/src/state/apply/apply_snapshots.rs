// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Supply snapshots and object-locked coin reconciliation for apply.

use super::*;

impl StateManager {
    pub(crate) fn supply_tracking_token_types(&self, changeset: &ChangeSet) -> BTreeSet<String> {
        let mut token_types: BTreeSet<String> =
            self.global_token_supplies.keys().cloned().collect();
        token_types.insert(GAS_COIN.to_string());

        for (_, token_type, _) in &changeset.treasuries {
            token_types.insert(Self::normalize_token_type(token_type));
        }
        for (_, token_type, _) in &changeset.token_balance_sets {
            token_types.insert(Self::normalize_token_type(token_type));
        }
        for (_, created) in &changeset.created_objects {
            if let Some((token_type, _)) = Self::balance_token_amount(&created.type_, &created.data)
            {
                token_types.insert(token_type);
            }
        }

        token_types
    }

    pub(crate) fn visible_supply_snapshot(&self, token_type: &str) -> Result<u64> {
        let token_type = Self::normalize_token_type(token_type);
        let cached = self
            .global_token_supplies
            .get(&token_type)
            .copied()
            .unwrap_or(0);
        Ok(cached.max(self.indexed_wallet_supply(&token_type)?))
    }

    pub(crate) fn capture_supply_snapshots(
        &self,
        token_types: BTreeSet<String>,
    ) -> Result<(BTreeMap<String, u64>, BTreeMap<String, u64>)> {
        let mut issued = BTreeMap::new();
        let mut visible = BTreeMap::new();

        for token_type in token_types {
            issued.insert(
                token_type.clone(),
                self.issued_supply_for_token(&token_type)?,
            );
            visible.insert(
                token_type.clone(),
                self.visible_supply_snapshot(&token_type)?,
            );
        }

        Ok((issued, visible))
    }

    pub(crate) fn add_locked_coin_record(
        records: &mut Vec<ObjectLockedCoinRecord>,
        holder: &(String, CreatedObject),
        token_type: &str,
        amount: u64,
    ) -> Result<()> {
        if amount == 0 {
            return Ok(());
        }

        if let Some(existing) = records
            .iter_mut()
            .find(|record| record.holder_object_id == holder.0 && record.token_type == token_type)
        {
            existing.amount = existing
                .amount
                .checked_add(amount)
                .require("Object-locked coin record overflow")?;
            existing.holder_type = holder.1.type_.clone();
            existing.owner = holder.1.owner;
            return Ok(());
        }

        records.push(ObjectLockedCoinRecord {
            holder_object_id: holder.0.clone(),
            holder_type: holder.1.type_.clone(),
            owner: holder.1.owner,
            token_type: token_type.to_string(),
            amount,
        });
        Ok(())
    }

    pub(crate) fn release_locked_coin_records(
        records: &mut Vec<ObjectLockedCoinRecord>,
        holder_ids: &HashSet<String>,
        token_type: &str,
        amount: u64,
    ) {
        let mut remaining = amount;

        for prefer_holder in [true, false] {
            if remaining == 0 {
                break;
            }

            for record in records.iter_mut() {
                if remaining == 0 {
                    break;
                }
                if record.token_type != token_type {
                    continue;
                }
                if prefer_holder && !holder_ids.contains(&record.holder_object_id) {
                    continue;
                }

                let release = record.amount.min(remaining);
                record.amount -= release;
                remaining -= release;
            }
        }

        records.retain(|record| record.amount > 0);
    }

    pub(crate) fn reconcile_object_locked_coin_records(
        &mut self,
        changeset: &ChangeSet,
        issued_before: &BTreeMap<String, u64>,
        visible_before: &BTreeMap<String, u64>,
    ) -> Result<()> {
        let holder_candidates: Vec<(String, CreatedObject)> = changeset
            .created_objects
            .iter()
            .filter(|(_, created)| Self::is_object_locked_coin_holder_type(&created.type_))
            .map(|(id, created)| (id.clone(), created.clone()))
            .collect();

        if holder_candidates.is_empty() && issued_before.is_empty() {
            return Ok(());
        }

        let holder_ids: HashSet<String> =
            holder_candidates.iter().map(|(id, _)| id.clone()).collect();
        let deleted_ids: HashSet<String> = changeset.deleted_objects.iter().cloned().collect();
        let mut records = self.load_object_locked_coin_records()?;
        let original_records = records.clone();

        if !deleted_ids.is_empty() {
            records.retain(|record| !deleted_ids.contains(&record.holder_object_id));
        }

        let token_types: BTreeSet<String> = issued_before
            .keys()
            .chain(visible_before.keys())
            .cloned()
            .collect();

        for token_type in token_types {
            let issued_before_value = issued_before.get(&token_type).copied().unwrap_or(0);
            let visible_before_value = visible_before.get(&token_type).copied().unwrap_or(0);
            let issued_after_value = self.issued_supply_for_token(&token_type)?;
            let visible_after_value = self.visible_supply_snapshot(&token_type)?;

            let issued_delta = issued_after_value as i128 - issued_before_value as i128;
            let visible_delta = visible_after_value as i128 - visible_before_value as i128;
            let locked_delta = issued_delta - visible_delta;

            if locked_delta > 0 {
                if let Some(holder) = holder_candidates.first() {
                    Self::add_locked_coin_record(
                        &mut records,
                        holder,
                        &token_type,
                        locked_delta as u64,
                    )?;
                }
            } else if locked_delta < 0 {
                Self::release_locked_coin_records(
                    &mut records,
                    &holder_ids,
                    &token_type,
                    (-locked_delta) as u64,
                );
            }
        }

        if records != original_records {
            self.save_object_locked_coin_records(&records)?;
        }

        Ok(())
    }
}
