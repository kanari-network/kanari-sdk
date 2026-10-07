// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Owner-balance recompute marking for changeset application.

use super::*;

impl StateManager {
    pub(crate) fn needs_object_locked_reconciliation(&self, changeset: &ChangeSet) -> Result<bool> {
        if !changeset.treasuries.is_empty()
            || !changeset.token_balance_sets.is_empty()
            || !changeset.deleted_objects.is_empty()
        {
            return Ok(true);
        }

        changeset
            .created_objects
            .iter()
            .map(|(object_id, created)| {
                Ok(!self.is_system_clock_object(object_id, &created.type_)?
                    && (Self::is_object_locked_coin_holder_type(&created.type_)
                        || Self::treasury_cap_token_supply(&created.type_, &created.data)
                            .is_some()))
            })
            .try_fold(false, |acc, item| item.map(|value| acc || value))
    }

    pub(crate) fn mark_owner_for_object_balance_recompute(
        owners_to_recompute: &mut BTreeSet<AccountAddress>,
        native_object_changed_owners: &mut BTreeSet<AccountAddress>,
        non_native_object_changed_owners: &mut BTreeSet<AccountAddress>,
        owner: AccountAddress,
        token_type: &str,
    ) {
        owners_to_recompute.insert(owner);
        if Self::normalize_token_type(token_type) == GAS_COIN {
            native_object_changed_owners.insert(owner);
        } else {
            non_native_object_changed_owners.insert(owner);
        }
    }

    pub(crate) fn record_object_balance_owner_if_needed(
        owners_to_recompute: &mut BTreeSet<AccountAddress>,
        native_object_changed_owners: &mut BTreeSet<AccountAddress>,
        non_native_object_changed_owners: &mut BTreeSet<AccountAddress>,
        owner: AccountAddress,
        type_name: &str,
        data: &[u8],
    ) {
        if let Some((token_type, _)) = Self::balance_token_amount(type_name, data) {
            Self::mark_owner_for_object_balance_recompute(
                owners_to_recompute,
                native_object_changed_owners,
                non_native_object_changed_owners,
                owner,
                &token_type,
            );
        }
    }

    pub(crate) fn object_balance_token_for_type_and_data(
        type_name: &str,
        data: &[u8],
    ) -> Option<String> {
        Self::balance_token_amount(type_name, data)
            .map(|(token_type, _)| Self::normalize_token_type(&token_type))
    }

    pub(crate) fn changeset_creates_object_balance_for_owner(
        changeset: &ChangeSet,
        owner: AccountAddress,
        token_type: &str,
    ) -> bool {
        changeset.created_objects.iter().any(|(_, created)| {
            created.owner == owner
                && Self::object_balance_token_for_type_and_data(&created.type_, &created.data)
                    .is_some_and(|created_token| created_token == token_type)
        })
    }

    pub(crate) fn changeset_deletes_object_balance_for_owner(
        &self,
        changeset: &ChangeSet,
        owner: AccountAddress,
        token_type: &str,
    ) -> Result<bool> {
        for object_id in &changeset.deleted_objects {
            let Some((_, existing)) = self.load_stored_object_by_any_id(object_id)? else {
                continue;
            };
            if existing.owner != owner {
                continue;
            }
            if Self::object_balance_token_for_type_and_data(&existing.type_name, &existing.data)
                .is_some_and(|existing_token| existing_token == token_type)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(crate) fn owner_has_object_backed_token_balance(
        &self,
        owner: AccountAddress,
        token_type: &str,
    ) -> Result<bool> {
        Ok(self
            .compute_owned_token_balances(owner, None)?
            .contains_key(token_type))
    }

    pub(crate) fn changeset_touches_object_backed_token_for_owner(
        &self,
        changeset: &ChangeSet,
        owner: AccountAddress,
        token_type: &str,
    ) -> Result<bool> {
        Ok(
            self.owner_has_object_backed_token_balance(owner, token_type)?
                || Self::changeset_creates_object_balance_for_owner(changeset, owner, token_type)
                || self.changeset_deletes_object_balance_for_owner(changeset, owner, token_type)?,
        )
    }
}
