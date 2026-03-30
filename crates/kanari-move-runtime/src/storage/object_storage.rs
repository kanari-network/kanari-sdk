// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::storage::persistent_store::{PersistentStore, PersistentStoreError};
use anyhow::Result;
/// Object Storage Layer for persistent object tracking
/// Stores transferred objects that can be queried and used as function arguments
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::TypeTag;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

/// Error types that can occur during `ObjectStorage` operations.
#[derive(Debug)]
pub enum ObjectStorageError {
    /// Failed to acquire or interact with an in-memory lock.
    LockError(String),

    /// Failure writing to or reading from the persistent backend.
    PersistenceError(anyhow::Error),

    /// The requested object was not found in the store.
    NotFound,
}

impl ObjectStorageError {
    /// Returns true when this is a lock-related error.
    pub fn is_lock_error(&self) -> bool {
        matches!(self, ObjectStorageError::LockError(_))
    }

    /// Returns true when this is a persistence backend error.
    pub fn is_persistence_error(&self) -> bool {
        matches!(self, ObjectStorageError::PersistenceError(_))
    }

    /// Returns true when the object was not found.
    pub fn is_not_found(&self) -> bool {
        matches!(self, ObjectStorageError::NotFound)
    }
}

impl std::fmt::Display for ObjectStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectStorageError::LockError(s) => write!(f, "LockError: {}", s),
            ObjectStorageError::PersistenceError(e) => write!(f, "PersistenceError: {}", e),
            ObjectStorageError::NotFound => write!(f, "NotFound"),
        }
    }
}

impl std::error::Error for ObjectStorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ObjectStorageError::PersistenceError(e) => e.source(),
            _ => None,
        }
    }
}

impl From<anyhow::Error> for ObjectStorageError {
    fn from(e: anyhow::Error) -> Self {
        ObjectStorageError::PersistenceError(e)
    }
}

impl From<PersistentStoreError> for ObjectStorageError {
    fn from(e: PersistentStoreError) -> Self {
        ObjectStorageError::PersistenceError(e.into())
    }
}

/// Trait abstraction for object storage backends. Allows swapping in-memory and
/// persistent implementations without changing the runtime.
pub trait ObjectStore: Send + Sync {
    fn store_object(&self, obj: StoredObject) -> Result<(), ObjectStorageError>;
    fn get_object(&self, id: &str) -> Option<StoredObject>;
    fn get_objects_by_owner(&self, owner: &AccountAddress) -> Vec<StoredObject>;
    fn transfer_object(
        &self,
        id: &str,
        new_owner: AccountAddress,
    ) -> Result<(), ObjectStorageError>;
    fn delete_object(&self, id: &str) -> Result<(), ObjectStorageError>;
    fn count(&self) -> usize;
    fn clear(&self) -> Result<(), ObjectStorageError>;

    fn get_coins_by_type_and_owner(
        &self,
        owner: AccountAddress,
        coin_type: &TypeTag,
    ) -> Vec<StoredObject>;
}

/// Stored object with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredObject {
    pub id: String,
    pub owner: AccountAddress,
    pub type_name: String,
    pub data: Vec<u8>,
    pub version: u64,
}

struct InnerState {
    objects: BTreeMap<String, StoredObject>,
    // 🚨 ลบ owner_index ออกจาก Memory เพื่อแก้ปัญหา RAM บวม
}

pub struct ObjectStorage {
    state: Arc<RwLock<InnerState>>,
    persistent: Option<Arc<PersistentStore>>,
}

impl ObjectStorage {
    const OBJECT_INDEX_KEY: &'static str = "object_index";

    // 🚨 Helper สร้าง Key สำหรับดึงข้อมูล Owner Index ตรงจากฐานข้อมูล RocksDB
    fn owner_key(owner: &AccountAddress) -> Vec<u8> {
        let mut key = b"owner_index:".to_vec();
        key.extend_from_slice(owner.as_ref());
        key
    }

    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(InnerState {
                objects: BTreeMap::new(),
            })),
            persistent: None,
        }
    }

    pub fn boxed_inmemory() -> Box<dyn ObjectStore> {
        Box::new(Self::new())
    }
}

impl Default for ObjectStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectStorage {
    pub fn get_coins_by_type_and_owner(
        &self,
        owner: AccountAddress,
        coin_type: &move_core_types::language_storage::TypeTag,
    ) -> Vec<StoredObject> {
        self.get_objects_by_owner(&owner)
            .into_iter()
            .filter(|obj| {
                if let Ok(struct_tag) = obj
                    .type_name
                    .parse::<move_core_types::language_storage::StructTag>()
                    && struct_tag.module.as_str() == "coin"
                    && struct_tag.name.as_str() == "Coin"
                    && let Some(tag) = struct_tag.type_params.first()
                {
                    return tag == coin_type;
                }
                false
            })
            .collect()
    }

    /// Create a new ObjectStorage backed by RocksDB persistence (uses `PersistentStore::open_default`).
    pub fn new_with_persistence() -> Result<Self> {
        let store = PersistentStore::open_default()?;
        let store = Arc::new(store);

        let mut objects_map: BTreeMap<String, StoredObject> = BTreeMap::new();

        if let Ok(Some(ids)) = store.load::<Vec<String>>(Self::OBJECT_INDEX_KEY.as_bytes()) {
            for id in ids.into_iter() {
                if let Ok(Some(obj)) =
                    store.load::<StoredObject>(format!("object:{}", id).as_bytes())
                {
                    objects_map.insert(id.clone(), obj);
                }
            }
        }

        Ok(Self {
            state: Arc::new(RwLock::new(InnerState {
                objects: objects_map,
            })),
            persistent: Some(store),
        })
    }

    pub fn boxed_with_persistence() -> Result<Box<dyn ObjectStore>> {
        if cfg!(miri) {
            return Ok(Self::boxed_inmemory());
        }
        Ok(Box::new(Self::new_with_persistence()?))
    }

    pub fn store_object(&self, obj: StoredObject) -> Result<(), ObjectStorageError> {
        let id = obj.id.clone();
        let owner = obj.owner;
        let mut old_owner = None;

        // 🚨 Lock Poisoning Fix: ใช้ unwrap_or_else เสมอ
        {
            let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
            if let Some(existing) = state.objects.get(&id) {
                old_owner = Some(existing.owner);
            }
            state.objects.insert(id.clone(), obj.clone());
        }

        // 🚨 Persist owner index directly to DB instead of keeping in memory
        if let Some(store) = &self.persistent {
            store.save(format!("object:{}", id).as_bytes(), &obj)?;

            if let Some(old) = old_owner {
                if old != owner {
                    // ลบออกจากกระเป๋าคนเก่า
                    let old_key = Self::owner_key(&old);
                    let mut old_ids: Vec<String> = store.load(&old_key)?.unwrap_or_default();
                    old_ids.retain(|oid| oid != &id);
                    store.save(&old_key, &old_ids)?;

                    // ใส่กระเป๋าคนใหม่
                    let new_key = Self::owner_key(&owner);
                    let mut new_ids: Vec<String> = store.load(&new_key)?.unwrap_or_default();
                    if !new_ids.contains(&id) {
                        new_ids.push(id.clone());
                        store.save(&new_key, &new_ids)?;
                    }
                }
            } else {
                // Object ใหม่
                let new_key = Self::owner_key(&owner);
                let mut new_ids: Vec<String> = store.load(&new_key)?.unwrap_or_default();
                if !new_ids.contains(&id) {
                    new_ids.push(id.clone());
                    store.save(&new_key, &new_ids)?;
                }
            }

            // อัปเดต Global Object Index
            let mut ids: Vec<String> = store
                .load(Self::OBJECT_INDEX_KEY.as_bytes())?
                .unwrap_or_default();

            if !ids.iter().any(|x| x == &id) {
                ids.push(id.clone());
                store.save(Self::OBJECT_INDEX_KEY.as_bytes(), &ids)?;
            }
        }

        Ok(())
    }

    /// Get object by ID
    pub fn get_object(&self, id: &str) -> Option<StoredObject> {
        // 🚨 Lock Poisoning Fix
        let state = self.state.read().unwrap_or_else(|e| e.into_inner());
        if let Some(obj) = state.objects.get(id) {
            return Some(obj.clone());
        }
        drop(state); // Drop Lock ก่อนไปแตะฐานข้อมูล

        if let Some(store) = &self.persistent
            && let Ok(Some(obj)) = store.load::<StoredObject>(format!("object:{}", id).as_bytes())
        {
            let mut write_state = self.state.write().unwrap_or_else(|e| e.into_inner());
            write_state.objects.insert(id.to_string(), obj.clone());
            return Some(obj);
        }
        None
    }

    /// Get all objects owned by an address
    pub fn get_objects_by_owner(&self, owner: &AccountAddress) -> Vec<StoredObject> {
        // 🚨 Read Owner Index from DB directly if persistent
        if let Some(store) = &self.persistent {
            let key = Self::owner_key(owner);
            let ids: Vec<String> = store.load(&key).unwrap_or(None).unwrap_or_default();

            let mut results = Vec::new();
            for id in ids {
                if let Some(obj) = self.get_object(&id) {
                    results.push(obj);
                }
            }
            return results;
        }

        // 🚨 Fallback: in-memory calculation (used in testing environments)
        let state = self.state.read().unwrap_or_else(|e| e.into_inner());
        state
            .objects
            .values()
            .filter(|obj| obj.owner == *owner)
            .cloned()
            .collect()
    }

    // Transfer object ownership
    pub fn transfer_object(
        &self,
        id: &str,
        new_owner: AccountAddress,
    ) -> Result<(), ObjectStorageError> {
        if self.get_object(id).is_none() {
            return Err(ObjectStorageError::NotFound);
        }

        // 🚨 FIX: ดึงค่าออกมาจาก Scope ของ RwLock ตรงๆ โดยไม่ต้องประกาศตัวแปรหลอกๆ ไว้ก่อน
        let (old_owner, obj_to_persist) = {
            let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
            if let Some(obj) = state.objects.get_mut(id) {
                let old = obj.owner;
                obj.owner = new_owner;
                (old, obj.clone()) // ส่งคืนค่าเป็น Tuple
            } else {
                return Err(ObjectStorageError::NotFound);
            }
        };

        if let Some(store) = &self.persistent {
            // ไม่ต้องเช็ค if let Some(obj) แล้ว เพราะเราได้ค่ามาเป็นชิ้นเป็นอันแล้ว
            store.save(format!("object:{}", id).as_bytes(), &obj_to_persist)?;

            let old_key = Self::owner_key(&old_owner);
            let mut old_ids: Vec<String> = store.load(&old_key)?.unwrap_or_default();
            old_ids.retain(|oid| oid != id);
            store.save(&old_key, &old_ids)?;

            let new_key = Self::owner_key(&new_owner);
            let mut new_ids: Vec<String> = store.load(&new_key)?.unwrap_or_default();
            let id_str = id.to_string();
            if !new_ids.contains(&id_str) {
                new_ids.push(id_str);
                store.save(&new_key, &new_ids)?;
            }
        }

        Ok(())
    }

    /// Delete object
    pub fn delete_object(&self, id: &str) -> Result<(), ObjectStorageError> {
        let mut old_owner = None;

        {
            let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
            if let Some(obj) = state.objects.remove(id) {
                old_owner = Some(obj.owner);
            }
        }

        if let Some(store) = &self.persistent {
            store.delete(format!("object:{}", id).as_bytes())?;

            if let Some(owner) = old_owner {
                let owner_key = Self::owner_key(&owner);
                let mut ids: Vec<String> = store.load(&owner_key)?.unwrap_or_default();
                ids.retain(|oid| oid != id);
                store.save(&owner_key, &ids)?;
            }

            let mut ids: Vec<String> = store
                .load(Self::OBJECT_INDEX_KEY.as_bytes())?
                .unwrap_or_default();
            if let Some(pos) = ids.iter().position(|x| x == id) {
                ids.remove(pos);
                store.save(Self::OBJECT_INDEX_KEY.as_bytes(), &ids)?;
            }
        }

        Ok(())
    }

    /// Get total number of objects
    pub fn count(&self) -> usize {
        self.state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .objects
            .len()
    }

    /// Clear all objects
    pub fn clear(&self) -> Result<(), ObjectStorageError> {
        let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
        state.objects.clear();
        Ok(())
    }
}

// Implement the ObjectStore trait for the in-memory ObjectStorage so it can
// be used as a boxed trait object by the runtime.
impl ObjectStore for ObjectStorage {
    fn store_object(&self, obj: StoredObject) -> Result<(), ObjectStorageError> {
        ObjectStorage::store_object(self, obj)
    }

    fn get_object(&self, id: &str) -> Option<StoredObject> {
        ObjectStorage::get_object(self, id)
    }

    fn get_objects_by_owner(&self, owner: &AccountAddress) -> Vec<StoredObject> {
        ObjectStorage::get_objects_by_owner(self, owner)
    }

    fn transfer_object(
        &self,
        id: &str,
        new_owner: AccountAddress,
    ) -> Result<(), ObjectStorageError> {
        ObjectStorage::transfer_object(self, id, new_owner)
    }

    fn delete_object(&self, id: &str) -> Result<(), ObjectStorageError> {
        ObjectStorage::delete_object(self, id)
    }

    fn count(&self) -> usize {
        ObjectStorage::count(self)
    }

    fn clear(&self) -> Result<(), ObjectStorageError> {
        ObjectStorage::clear(self)
    }

    fn get_coins_by_type_and_owner(
        &self,
        owner: AccountAddress,
        coin_type: &TypeTag,
    ) -> Vec<StoredObject> {
        ObjectStorage::get_coins_by_type_and_owner(self, owner, coin_type)
    }
}
