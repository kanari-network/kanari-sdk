// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Treasury persistence and owned-object index maintenance for apply.

use super::*;

impl StateManager {
    pub(crate) fn persist_treasury_cap_state(
        &mut self,
        token_type: &str,
        owner: AccountAddress,
        total_supply: u64,
    ) -> Result<bool> {
        // supply_key normalizes internally; treasury keys normalized here so
        // 0x02/0x2 spellings share one record going forward.
        let normalized = Self::normalize_token_type(token_type);
        let key = Self::supply_key(&normalized);
        self.save_internal(&key, &TreasuryCap { total_supply })?;

        let mut key_owner = b"treasury:".to_vec();
        key_owner.extend_from_slice(normalized.as_bytes());
        self.save_internal(&key_owner, &owner)?;
        self.add_to_index_list(b"treasury_index", format!("treasury:{}", normalized))?;

        if Self::normalize_token_type(token_type) == GAS_COIN {
            self.save_native_total_supply(total_supply)?;
        }

        Ok(true)
    }

    pub(crate) fn canonical_owned_object_id(object_id: &str) -> String {
        canonical_object_id(object_id).unwrap_or_else(|| object_id.to_string())
    }

    pub(crate) fn remove_owned_object_index_variants(
        &mut self,
        owner: AccountAddress,
        object_id: &str,
        stored_id: Option<&str>,
    ) -> Result<()> {
        let owner_key = owned_objects_key(&owner);
        self.remove_from_index_list(&owner_key, object_id)?;

        if let Some(stored_id) = stored_id
            && stored_id != object_id
        {
            self.remove_from_index_list(&owner_key, stored_id)?;
        }

        let canonical_id = Self::canonical_owned_object_id(object_id);
        if canonical_id != object_id {
            self.remove_from_index_list(&owner_key, &canonical_id)?;
        }
        if let Some(stored_id) = stored_id
            && canonical_id != stored_id
        {
            self.remove_from_index_list(&owner_key, &canonical_id)?;
        }

        Ok(())
    }

    pub(crate) fn refresh_owned_object_index(
        &mut self,
        owner: AccountAddress,
        object_id: &str,
        stored_id: Option<&str>,
    ) -> Result<()> {
        self.remove_owned_object_index_variants(owner, object_id, stored_id)?;
        self.add_to_index_list(
            &owned_objects_key(&owner),
            Self::canonical_owned_object_id(object_id),
        )
    }

    pub(crate) fn add_many_owned_object_index(
        &mut self,
        owner: AccountAddress,
        object_ids: Vec<String>,
    ) -> Result<()> {
        self.add_many_to_index_list(&owned_objects_key(&owner), object_ids)
    }
}
