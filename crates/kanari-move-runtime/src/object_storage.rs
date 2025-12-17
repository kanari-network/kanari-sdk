use crate::storage::persistent_store::PersistentStore;
use anyhow::Result;
/// Object Storage Layer for persistent object tracking
/// Stores transferred objects that can be queried and used as function arguments
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Trait abstraction for object storage backends. Allows swapping in-memory and
/// persistent implementations without changing the runtime.
pub trait ObjectStore: Send + Sync {
    fn store_object(&self, obj: StoredObject) -> Result<(), String>;
    fn get_object(&self, id: &str) -> Option<StoredObject>;
    fn get_objects_by_owner(&self, owner: &AccountAddress) -> Vec<StoredObject>;
    fn delete_object(&self, id: &str) -> Result<(), String>;
    fn transfer_object(&self, id: &str, new_owner: AccountAddress) -> Result<(), String>;
    fn count(&self) -> usize;
    fn clear(&self) -> Result<(), String>;
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

/// Object Storage - in-memory cache backed by persistent DB
pub struct ObjectStorage {
    // Object ID -> StoredObject
    objects: Arc<RwLock<HashMap<String, StoredObject>>>,
    // Owner Address -> Vec<Object IDs>
    owner_index: Arc<RwLock<HashMap<AccountAddress, Vec<String>>>>,
    // Optional persistent backend
    persistent: Option<Arc<PersistentStore>>,
}

impl ObjectStorage {
    const OBJECT_INDEX_KEY: &'static str = "object_index";

    pub fn new() -> Self {
        Self {
            objects: Arc::new(RwLock::new(HashMap::new())),
            owner_index: Arc::new(RwLock::new(HashMap::new())),
            persistent: None,
        }
    }

    /// Return a boxed in-memory `ObjectStore` trait object.
    pub fn boxed_inmemory() -> Box<dyn ObjectStore> {
        Box::new(Self::new())
    }

    /// Create a new ObjectStorage backed by RocksDB persistence (uses `PersistentStore::open_default`).
    pub fn new_with_persistence() -> Result<Self> {
        let store = PersistentStore::open_default()?;
        let store = Arc::new(store);

        // Load object index (list of ids) if present and rebuild in-memory maps
        let mut objects_map: HashMap<String, StoredObject> = HashMap::new();

        if let Ok(Some(ids)) = store.load_json::<Vec<String>>(Self::OBJECT_INDEX_KEY) {
            for id in ids.into_iter() {
                if let Ok(Some(obj)) = store.load_json::<StoredObject>(&format!("object:{}", id)) {
                    objects_map.insert(id.clone(), obj);
                }
            }
        }

        // Build owner index from objects
        let mut owner_map: HashMap<AccountAddress, Vec<String>> = HashMap::new();
        for (id, obj) in objects_map.iter() {
            owner_map
                .entry(obj.owner)
                .or_insert_with(Vec::new)
                .push(id.clone());
        }

        Ok(Self {
            objects: Arc::new(RwLock::new(objects_map)),
            owner_index: Arc::new(RwLock::new(owner_map)),
            persistent: Some(store),
        })
    }

    /// Return a boxed persistent `ObjectStore` trait object.
    pub fn boxed_with_persistence() -> Result<Box<dyn ObjectStore>> {
        Ok(Box::new(Self::new_with_persistence()?))
    }

    /// Store an object
    pub fn store_object(&self, obj: StoredObject) -> Result<(), String> {
        let id = obj.id.clone();
        let owner = obj.owner;

        // Store object
        {
            let mut objects = self
                .objects
                .write()
                .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
            objects.insert(id.clone(), obj.clone());
        }

        // Update owner index
        {
            let mut owner_index = self
                .owner_index
                .write()
                .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
            owner_index
                .entry(owner)
                .or_insert_with(Vec::new)
                .push(id.clone());
        }

        // Persist to DB if available
        if let Some(store) = &self.persistent {
            // save object
            store
                .save_json(&format!("object:{}", id.clone()), &obj)
                .map_err(|e| format!("Failed to persist object: {}", e))?;

            // update object index
            let mut ids = store
                .load_json::<Vec<String>>(Self::OBJECT_INDEX_KEY)
                .map_err(|e| format!("Failed to load object index: {}", e))?
                .unwrap_or_default();
            if !ids.iter().any(|x| x == &id) {
                ids.push(id.clone());
                store
                    .save_json(Self::OBJECT_INDEX_KEY, &ids)
                    .map_err(|e| format!("Failed to persist object index: {}", e))?;
            }
        }

        Ok(())
    }

    /// Get object by ID
    pub fn get_object(&self, id: &str) -> Option<StoredObject> {
        // Prefer in-memory; if not present and persistent enabled, try loading from DB
        if let Ok(objects) = self.objects.read() {
            if let Some(obj) = objects.get(id).cloned() {
                return Some(obj);
            }
        }

        if let Some(store) = &self.persistent {
            if let Ok(Some(obj)) = store.load_json::<StoredObject>(&format!("object:{}", id)) {
                // populate in-memory caches
                let _ = self.objects.write().map(|mut o| {
                    o.insert(id.to_string(), obj.clone());
                });
                let _ = self.owner_index.write().map(|mut idx| {
                    idx.entry(obj.owner)
                        .or_insert_with(Vec::new)
                        .push(id.to_string());
                });
                return Some(obj);
            }
        }
        None
    }

    /// Get all objects owned by an address
    pub fn get_objects_by_owner(&self, owner: &AccountAddress) -> Vec<StoredObject> {
        let owner_index = match self.owner_index.read() {
            Ok(idx) => idx,
            Err(_) => return Vec::new(),
        };

        let object_ids = match owner_index.get(owner) {
            Some(ids) => ids.clone(),
            None => return Vec::new(),
        };

        let objects = match self.objects.read() {
            Ok(objs) => objs,
            Err(_) => return Vec::new(),
        };

        object_ids
            .iter()
            .filter_map(|id| objects.get(id).cloned())
            .collect()
    }

    /// Delete object by ID
    pub fn delete_object(&self, id: &str) -> Result<(), String> {
        let removed = {
            let mut objects = self
                .objects
                .write()
                .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
            objects.remove(id)
        };

        if let Some(obj) = removed {
            // Remove from owner index
            let mut owner_index = self
                .owner_index
                .write()
                .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
            if let Some(ids) = owner_index.get_mut(&obj.owner) {
                ids.retain(|oid| oid != id);
            }

            // Remove from persistent store if enabled
            if let Some(store) = &self.persistent {
                // delete object key
                store
                    .delete(&format!("object:{}", id))
                    .map_err(|e| format!("Failed to delete persisted object: {}", e))?;

                // update index
                let mut ids = store
                    .load_json::<Vec<String>>(Self::OBJECT_INDEX_KEY)
                    .map_err(|e| format!("Failed to load object index: {}", e))?
                    .unwrap_or_default();
                ids.retain(|x| x != id);
                store
                    .save_json(Self::OBJECT_INDEX_KEY, &ids)
                    .map_err(|e| format!("Failed to persist object index: {}", e))?;
            }
        }

        Ok(())
    }

    /// Update object ownership
    pub fn transfer_object(&self, id: &str, new_owner: AccountAddress) -> Result<(), String> {
        let old_owner = {
            let mut objects = self
                .objects
                .write()
                .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

            let obj = objects
                .get_mut(id)
                .ok_or_else(|| format!("Object {} not found", id))?;

            let old_owner = obj.owner;
            obj.owner = new_owner;
            old_owner
        };

        // Update owner indices
        {
            let mut owner_index = self
                .owner_index
                .write()
                .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

            // Remove from old owner
            if let Some(ids) = owner_index.get_mut(&old_owner) {
                ids.retain(|oid| oid != id);
            }

            // Add to new owner
            owner_index
                .entry(new_owner)
                .or_insert_with(Vec::new)
                .push(id.to_string());
        }

        // Persist updated object if available
        if let Some(store) = &self.persistent {
            // fetch the updated object from memory
            if let Ok(objects) = self.objects.read() {
                if let Some(obj) = objects.get(id) {
                    store
                        .save_json(&format!("object:{}", id), obj)
                        .map_err(|e| format!("Failed to persist transferred object: {}", e))?;
                }
            }
        }

        Ok(())
    }

    /// Get total number of objects
    pub fn count(&self) -> usize {
        self.objects.read().map(|objs| objs.len()).unwrap_or(0)
    }

    /// Clear all objects
    pub fn clear(&self) -> Result<(), String> {
        {
            let mut objects = self
                .objects
                .write()
                .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
            objects.clear();
        }
        {
            let mut owner_index = self
                .owner_index
                .write()
                .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
            owner_index.clear();
        }
        Ok(())
    }
}

// Implement the ObjectStore trait for the in-memory ObjectStorage so it can
// be used as a boxed trait object by the runtime.
impl ObjectStore for ObjectStorage {
    fn store_object(&self, obj: StoredObject) -> Result<(), String> {
        // Call the inherent implementation
        ObjectStorage::store_object(self, obj)
    }

    fn get_object(&self, id: &str) -> Option<StoredObject> {
        ObjectStorage::get_object(self, id)
    }

    fn get_objects_by_owner(&self, owner: &AccountAddress) -> Vec<StoredObject> {
        ObjectStorage::get_objects_by_owner(self, owner)
    }

    fn delete_object(&self, id: &str) -> Result<(), String> {
        ObjectStorage::delete_object(self, id)
    }

    fn transfer_object(&self, id: &str, new_owner: AccountAddress) -> Result<(), String> {
        ObjectStorage::transfer_object(self, id, new_owner)
    }

    fn count(&self) -> usize {
        ObjectStorage::count(self)
    }

    fn clear(&self) -> Result<(), String> {
        ObjectStorage::clear(self)
    }
}

impl Default for ObjectStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_get_object() {
        let storage = ObjectStorage::new();
        let owner = AccountAddress::random();

        let obj = StoredObject {
            id: "test123".to_string(),
            owner,
            type_name: "TestType".to_string(),
            data: vec![1, 2, 3],
            version: 0,
        };

        storage.store_object(obj.clone()).unwrap();

        let retrieved = storage.get_object("test123").unwrap();
        assert_eq!(retrieved.id, obj.id);
        assert_eq!(retrieved.owner, obj.owner);
    }

    #[test]
    fn test_get_objects_by_owner() {
        let storage = ObjectStorage::new();
        let owner = AccountAddress::random();

        for i in 0..3 {
            let obj = StoredObject {
                id: format!("obj{}", i),
                owner,
                type_name: "TestType".to_string(),
                data: vec![i],
                version: 0,
            };
            storage.store_object(obj).unwrap();
        }

        let objects = storage.get_objects_by_owner(&owner);
        assert_eq!(objects.len(), 3);
    }

    #[test]
    fn test_transfer_object() {
        let storage = ObjectStorage::new();
        let owner1 = AccountAddress::random();
        let owner2 = AccountAddress::random();

        let obj = StoredObject {
            id: "test123".to_string(),
            owner: owner1,
            type_name: "TestType".to_string(),
            data: vec![1, 2, 3],
            version: 0,
        };

        storage.store_object(obj).unwrap();
        storage.transfer_object("test123", owner2).unwrap();

        let retrieved = storage.get_object("test123").unwrap();
        assert_eq!(retrieved.owner, owner2);

        let owner1_objs = storage.get_objects_by_owner(&owner1);
        assert_eq!(owner1_objs.len(), 0);

        let owner2_objs = storage.get_objects_by_owner(&owner2);
        assert_eq!(owner2_objs.len(), 1);
    }
}
