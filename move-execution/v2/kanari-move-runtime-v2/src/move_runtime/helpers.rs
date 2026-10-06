// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Internal helper functions for Move execution.

use kanari_system_natives::dynamic_field::{DynamicFieldResolver, DynamicFieldStorageExt};
use kanari_types::coin::CoinModule;
use kanari_types::transaction::{ObjectInput, ObjectOwnerKind};

use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::StructTag;
use std::collections::BTreeMap;
use std::sync::Arc;

struct RuntimeDynamicFieldResolver {
    store: Arc<crate::storage::persistent_store::PersistentStore>,
    overlay: Option<crate::StateOverlay>,
}

impl RuntimeDynamicFieldResolver {
    fn dynamic_field_key(object_id: &str, name_bytes: &[u8]) -> Vec<u8> {
        crate::common::keys::dynamic_field_key(object_id, name_bytes)
    }
}

impl DynamicFieldResolver for RuntimeDynamicFieldResolver {
    fn get_dynamic_field(
        &self,
        object_id: &str,
        name_bytes: &[u8],
    ) -> Result<Option<Vec<u8>>, String> {
        let key = Self::dynamic_field_key(object_id, name_bytes);
        if let Some(value) = self.overlay.as_ref().and_then(|overlay| overlay.get(&key)) {
            return match value {
                Some(bytes) => bcs::from_bytes(bytes)
                    .map(Some)
                    .map_err(|error| format!("overlay dynamic-field decode failed: {error}")),
                None => Ok(None),
            };
        }
        self.store
            .load::<Vec<u8>>(&key)
            .map_err(|error| format!("persistent dynamic-field read failed: {error}"))
    }
}

/// Size of a Move object UID in bytes (address)
const UID_SIZE: usize = 32;
/// Size of a u64 field in bytes
const U64_SIZE: usize = 8;

impl super::MoveRuntime {
    pub(crate) fn dynamic_field_storage_ext(
        &self,
        overlay: Option<crate::StateOverlay>,
    ) -> DynamicFieldStorageExt {
        DynamicFieldStorageExt::new(Arc::new(RuntimeDynamicFieldResolver {
            store: self.state.store(),
            overlay,
        }))
    }

    pub(crate) fn get_object_for_execution(
        &self,
        object_id: &str,
        overlay: Option<&BTreeMap<Vec<u8>, Option<Vec<u8>>>>,
    ) -> anyhow::Result<Option<crate::storage::object_storage::StoredObject>> {
        let key = crate::common::keys::object_key(object_id);
        if let Some(value) = overlay.and_then(|overlay| overlay.get(&key)) {
            return match value {
                Some(bytes) => Ok(Some(bcs::from_bytes(bytes)?)),
                None => Ok(None),
            };
        }
        if let Some(object) = self.object_storage.get_object(object_id)? {
            return Ok(Some(object));
        }

        let object_addr =
            match move_core_types::account_address::AccountAddress::from_hex_literal(object_id) {
                Ok(addr) => addr,
                Err(_) => return Ok(None),
            };
        let Some(object) = self.state.try_get_stored_object(&object_addr)? else {
            return Ok(None);
        };

        // Hydrate the runtime object cache so subsequent object-native borrows
        // in this process do not miss objects that already exist in state.
        let _ = self.object_storage.store_object(object.clone());
        Ok(Some(object))
    }

    /// Preload potential object arguments into LoadedObjectsExt before execution
    /// This enables native_borrow_global and borrow_global_mut to resolve objects during VM execution
    pub(crate) fn preload_objects_for_execution(
        &self,
        session: &mut move_vm_runtime::session::Session<
            crate::storage::resolver::KanariMoveResolver,
        >,
        object_inputs: &[ObjectInput],
        sender: Option<AccountAddress>,
        overlay: Option<&BTreeMap<Vec<u8>, Option<Vec<u8>>>>,
    ) -> anyhow::Result<()> {
        use kanari_system_natives::object::LoadedObjectsExt;

        let exts = session.get_native_extensions();
        let loaded_ext = exts.get_mut::<LoadedObjectsExt>();

        for input in object_inputs {
            if let Some(stored_obj) =
                self.get_object_for_execution(&input.object_ref.object_id, overlay)?
            {
                let can_mutably_borrow = Self::can_mutably_borrow_preloaded_object(
                    input.mutable,
                    &stored_obj.type_name,
                    &stored_obj.owner_kind,
                    stored_obj.owner,
                    sender,
                    true,
                );
                loaded_ext.insert(
                    input.object_ref.object_id.clone(),
                    stored_obj.type_name,
                    stored_obj.data,
                    can_mutably_borrow,
                );
                log::debug!(
                    "[RUNTIME] Preloaded explicit object input {} into LoadedObjectsExt",
                    input.object_ref.object_id
                );
            }
        }

        Ok(())
    }

    /// Preload object IDs passed as raw address arguments. Raw address args are
    /// allowed to mutably borrow only sender-owned or shared objects; cross-owner
    /// mutable app-object access must be declared through explicit object_inputs
    /// so validation and scheduling can observe the dependency.
    pub(crate) fn preload_object_ids_from_args(
        &self,
        session: &mut move_vm_runtime::session::Session<
            crate::storage::resolver::KanariMoveResolver,
        >,
        args: &[Vec<u8>],
        sender: Option<AccountAddress>,
        overlay: Option<&BTreeMap<Vec<u8>, Option<Vec<u8>>>>,
    ) -> anyhow::Result<()> {
        use kanari_system_natives::object::LoadedObjectsExt;

        let loaded_ext = session
            .get_native_extensions()
            .get_mut::<LoadedObjectsExt>();
        for arg in args {
            if arg.len() != 32 {
                continue;
            }

            let object_id = format!("0x{}", hex::encode(arg));
            if loaded_ext.get(&object_id).is_some() {
                continue;
            }

            let Some(stored_obj) = self.get_object_for_execution(&object_id, overlay)? else {
                continue;
            };

            let can_mutably_borrow = Self::can_mutably_borrow_preloaded_object(
                true,
                &stored_obj.type_name,
                &stored_obj.owner_kind,
                stored_obj.owner,
                sender,
                false,
            );
            loaded_ext.insert(
                object_id.clone(),
                stored_obj.type_name,
                stored_obj.data,
                can_mutably_borrow,
            );
            log::debug!(
                "[RUNTIME] Preloaded address-based object argument {} into LoadedObjectsExt",
                object_id
            );
        }

        Ok(())
    }

    pub(crate) fn can_mutably_borrow_preloaded_object(
        requested_mutable: bool,
        object_type: &str,
        owner_kind: &ObjectOwnerKind,
        owner: AccountAddress,
        sender: Option<AccountAddress>,
        allow_cross_owner_non_coin: bool,
    ) -> bool {
        if !requested_mutable || matches!(owner_kind, ObjectOwnerKind::Immutable) {
            return false;
        }
        if matches!(owner_kind, ObjectOwnerKind::Shared) {
            return true;
        }
        if sender == Some(owner) {
            return true;
        }

        // Cross-owner mutable access to app objects must be declared as an
        // explicit object input so the transaction exposes its dependency to
        // validation/scheduling. Raw address arguments are intentionally scoped
        // to the sender's own owned objects.
        allow_cross_owner_non_coin && !Self::is_coin_object_type(object_type)
    }

    pub(crate) fn is_coin_object_type(object_type: &str) -> bool {
        object_type.contains("::coin::Coin<") || object_type.contains("::coin::coin::Coin<")
    }

    /// Check if struct tag represents a treasury resource
    pub(crate) fn is_treasury_resource(&self, struct_tag: &StructTag) -> bool {
        struct_tag.name.as_str() == CoinModule::TREASURY_CAP_STRUCT
    }

    /// Extract total supply from TreasuryCap bytes
    pub(crate) fn extract_treasury_total_from_bytes(&self, bytes: &[u8]) -> Option<u64> {
        // TreasuryCap: [32-byte address][8-byte total_supply]
        if bytes.len() >= (UID_SIZE + U64_SIZE) {
            let supply_bytes: [u8; U64_SIZE] =
                bytes[UID_SIZE..(UID_SIZE + U64_SIZE)].try_into().ok()?;
            Some(u64::from_le_bytes(supply_bytes))
        } else {
            None
        }
    }
}
