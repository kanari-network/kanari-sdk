// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Object reads: balance codecs, id lookup, and object queries.

use super::*;

impl StateManager {
    pub(crate) fn write_balance_to_object_bytes(
        data: &mut [u8],
        struct_tag: &StructTag,
        amount: u64,
    ) -> bool {
        let module_name = struct_tag.module.as_str();
        let struct_name = struct_tag.name.as_str();
        let bytes = amount.to_le_bytes();

        if module_name == CoinModule::COIN_MODULE && struct_name == CoinModule::COIN_STRUCT {
            if data.len() < UID_SIZE + U64_SIZE {
                return false;
            }
            data[UID_SIZE..(UID_SIZE + U64_SIZE)].copy_from_slice(&bytes);
            return true;
        }

        if module_name == BalanceModule::BALANCE_MODULE
            && struct_name == BalanceModule::BALANCE_STRUCT
        {
            if data.len() < U64_SIZE {
                return false;
            }
            let start = data.len() - U64_SIZE;
            data[start..].copy_from_slice(&bytes);
            return true;
        }

        false
    }
    pub(crate) fn treasury_cap_token_supply(type_name: &str, data: &[u8]) -> Option<(String, u64)> {
        let struct_tag = StructTag::from_str(type_name).ok()?;
        if struct_tag.module.as_str() != CoinModule::COIN_MODULE
            || struct_tag.name.as_str() != CoinModule::TREASURY_CAP_STRUCT
        {
            return None;
        }
        let token_type = token_type_from_struct_tag(&struct_tag)?;
        if data.len() < UID_SIZE + U64_SIZE {
            return None;
        }
        let supply_bytes: [u8; U64_SIZE] = data[UID_SIZE..(UID_SIZE + U64_SIZE)].try_into().ok()?;
        Some((
            Self::normalize_token_type(&token_type),
            u64::from_le_bytes(supply_bytes),
        ))
    }

    // Helper to construct DB key for object

    pub(crate) fn object_lookup_ids(id: &str) -> Vec<String> {
        let mut ids = vec![id.to_string()];
        if let Some(canonical_id) = canonical_object_id(id)
            && canonical_id != id
        {
            ids.push(canonical_id);
        }
        ids
    }

    pub(crate) fn load_stored_object_by_any_id(
        &self,
        object_id: &str,
    ) -> Result<Option<(String, StoredObject)>> {
        for candidate_id in Self::object_lookup_ids(object_id) {
            let obj_key = object_key(&candidate_id);
            if let Some(stored) = self.load_internal::<StoredObject>(&obj_key)? {
                return Ok(Some((candidate_id, stored)));
            }
        }
        Ok(None)
    }

    pub(crate) fn is_canonical_state_root_key(key: &[u8]) -> bool {
        key != GENESIS_INITIALIZED_KEY
            && (key == b"total_supply"
                || key.starts_with(b"resource:")
                || key.starts_with(b"account:")
                || key.starts_with(b"df:")
                || key.starts_with(b"system:")
                || key.starts_with(b"supply:")
                || key.starts_with(b"treasury:")
                || key.starts_with(b"nft:"))
    }

    pub(crate) fn retain_canonical_state_root_entries(
        entries: &mut BTreeMap<Vec<u8>, Vec<u8>>,
    ) -> Result<()> {
        let mut canonical_object_keys = BTreeSet::new();
        let mut canonical_module_keys = BTreeSet::new();

        for (key, value) in entries.iter() {
            if key == b"module_index" {
                let module_ids = bcs::from_bytes::<Vec<String>>(value)
                    .context("Malformed canonical module index")?;
                canonical_module_keys.extend(
                    module_ids
                        .into_iter()
                        .filter(|id| id.starts_with("module:"))
                        .map(String::into_bytes),
                );
            } else if key.starts_with(b"owned_objects:") {
                let object_ids = bcs::from_bytes::<Vec<String>>(value).with_context(|| {
                    format!(
                        "Malformed owned-object index {:?}",
                        String::from_utf8_lossy(key)
                    )
                })?;
                canonical_object_keys.extend(
                    object_ids
                        .into_iter()
                        .map(|id| {
                            canonical_object_id(&id).ok_or_else(|| {
                                anyhow::anyhow!("Invalid object id in owned-object index: {id}")
                            })
                        })
                        .collect::<Result<Vec<_>>>()?
                        .into_iter()
                        .map(|id| {
                            let object_key = object_key(&id);
                            let bytes = entries.get(&object_key).ok_or_else(|| {
                                anyhow::anyhow!("Owned-object index references missing object {id}")
                            })?;
                            bcs::from_bytes::<StoredObject>(bytes)
                                .with_context(|| format!("Malformed object record {id}"))?;
                            Ok(object_key)
                        })
                        .collect::<Result<Vec<_>>>()?,
                );
            }
        }

        entries.retain(|key, _| {
            Self::is_canonical_state_root_key(key)
                || canonical_module_keys.contains(key)
                || canonical_object_keys.contains(key)
        });
        Ok(())
    }

    pub(crate) fn balance_token_amount(type_name: &str, data: &[u8]) -> Option<(String, u64)> {
        let struct_tag = StructTag::from_str(type_name).ok()?;
        if !is_balance_struct(&struct_tag) {
            return None;
        }
        let token_type = token_type_from_struct_tag(&struct_tag)?;
        let amount = extract_balance_from_object_bytes(data, &struct_tag)?;
        Some((Self::normalize_token_type(&token_type), amount))
    }

    /// Get all object IDs owned by an address
    pub fn get_owned_objects(&self, owner: &AccountAddress) -> Result<Vec<String>> {
        let owner_key = owned_objects_key(owner);
        let mut clean: Vec<String> = self.load_internal(&owner_key)?.unwrap_or_default();
        clean.sort();
        clean.dedup();
        Ok(clean)
    }

    /// Get a specific object by ID
    pub fn get_object(&self, object_id: &str) -> Result<Option<CreatedObject>> {
        let Some((stored_id, stored)) = self.load_stored_object_by_any_id(object_id)? else {
            return Ok(None);
        };
        let normalized_id = canonical_object_id(&stored_id).unwrap_or(stored_id.clone());
        let uid = AccountAddress::from_hex_literal(&normalized_id)
            .ok()
            .map(UIDRecord::new);
        let id = AccountAddress::from_hex_literal(&normalized_id)
            .ok()
            .map(IDRecord::new);

        Ok(Some(CreatedObject {
            owner: stored.owner,
            owner_kind: stored.owner_kind,
            uid,
            id,
            type_: stored.type_name,
            data: stored.data,
            version: stored.version,
        }))
    }

    /// Get all objects matching the specified type.
    pub fn get_objects_by_type(&self, object_type: &str) -> Result<Vec<(String, CreatedObject)>> {
        self.query_objects(None, None, Some(object_type), None, None)
    }

    /// Query objects by owner, type, and version range.
    pub fn query_objects(
        &self,
        owner: Option<AccountAddress>,
        owner_kind: Option<&ObjectOwnerKind>,
        object_type: Option<&str>,
        min_version: Option<u64>,
        max_version: Option<u64>,
    ) -> Result<Vec<(String, CreatedObject)>> {
        let object_ids = self.load_index_list(b"object_index")?;
        let mut objects = Vec::new();

        for object_id in object_ids {
            let object = self.get_object(&object_id)?.ok_or_else(|| {
                anyhow::anyhow!("Object index references missing object {object_id}")
            })?;
            if let Some(owner) = owner
                && object.owner != owner
            {
                continue;
            }
            if let Some(owner_kind) = owner_kind {
                let matches = match owner_kind {
                    ObjectOwnerKind::AddressOwner(_) => {
                        matches!(object.owner_kind, ObjectOwnerKind::AddressOwner(_))
                    }
                    ObjectOwnerKind::Shared => matches!(object.owner_kind, ObjectOwnerKind::Shared),
                    ObjectOwnerKind::Immutable => {
                        matches!(object.owner_kind, ObjectOwnerKind::Immutable)
                    }
                };
                if !matches {
                    continue;
                }
            }
            if let Some(object_type) = object_type
                && object.type_ != object_type
            {
                continue;
            }
            if let Some(min_version) = min_version
                && object.version < min_version
            {
                continue;
            }
            if let Some(max_version) = max_version
                && object.version > max_version
            {
                continue;
            }

            objects.push((object_id, object));
        }

        Ok(objects)
    }
}
