/// Object Storage Layer for persistent object tracking
/// Stores transferred objects that can be queried and used as function arguments
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

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
}

impl ObjectStorage {
    pub fn new() -> Self {
        Self {
            objects: Arc::new(RwLock::new(HashMap::new())),
            owner_index: Arc::new(RwLock::new(HashMap::new())),
        }
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
            objects.insert(id.clone(), obj);
        }

        // Update owner index
        {
            let mut owner_index = self
                .owner_index
                .write()
                .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
            owner_index.entry(owner).or_insert_with(Vec::new).push(id);
        }

        Ok(())
    }

    /// Get object by ID
    pub fn get_object(&self, id: &str) -> Option<StoredObject> {
        let objects = self.objects.read().ok()?;
        objects.get(id).cloned()
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
