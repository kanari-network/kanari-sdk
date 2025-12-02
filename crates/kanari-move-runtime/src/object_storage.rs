use anyhow::{anyhow, Result};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for an object
pub type ObjectID = AccountAddress;

/// Object ownership types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Owner {
    /// Object is owned by an address
    AddressOwner(AccountAddress),
    /// Object is shared (can be used by anyone)
    Shared,
    /// Object is immutable (frozen, cannot be modified)
    Immutable,
}

/// Represents a Move object with its data and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    /// Unique identifier
    pub id: ObjectID,
    /// Owner of this object
    pub owner: Owner,
    /// Type of the object (e.g., "0x2::coin::Coin<0x2::kanari::KANARI>")
    pub type_: String,
    /// Serialized Move value (BCS encoded)
    pub data: Vec<u8>,
    /// Version number (incremented on each modification)
    pub version: u64,
}

impl Object {
    pub fn new(id: ObjectID, owner: Owner, type_: String, data: Vec<u8>) -> Self {
        Self {
            id,
            owner,
            type_,
            data,
            version: 0,
        }
    }

    pub fn is_owned_by(&self, address: &AccountAddress) -> bool {
        matches!(&self.owner, Owner::AddressOwner(owner) if owner == address)
    }

    pub fn is_shared(&self) -> bool {
        matches!(self.owner, Owner::Shared)
    }

    pub fn is_immutable(&self) -> bool {
        matches!(self.owner, Owner::Immutable)
    }

    pub fn increment_version(&mut self) {
        self.version += 1;
    }
}

/// Object storage manager
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObjectStorage {
    /// All objects indexed by their ID
    objects: HashMap<ObjectID, Object>,
    /// Index: owner address -> list of owned object IDs
    owner_index: HashMap<AccountAddress, Vec<ObjectID>>,
    /// Index: type -> list of object IDs of that type
    type_index: HashMap<String, Vec<ObjectID>>,
}

impl ObjectStorage {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            owner_index: HashMap::new(),
            type_index: HashMap::new(),
        }
    }

    /// Store a new object
    pub fn insert(&mut self, object: Object) -> Result<()> {
        let id = object.id;
        let type_ = object.type_.clone();

        // Update owner index
        if let Owner::AddressOwner(owner) = &object.owner {
            self.owner_index
                .entry(*owner)
                .or_insert_with(Vec::new)
                .push(id);
        }

        // Update type index
        self.type_index
            .entry(type_)
            .or_insert_with(Vec::new)
            .push(id);

        // Store object
        self.objects.insert(id, object);
        Ok(())
    }

    /// Get an object by ID
    pub fn get(&self, id: &ObjectID) -> Option<&Object> {
        self.objects.get(id)
    }

    /// Get mutable reference to an object
    pub fn get_mut(&mut self, id: &ObjectID) -> Option<&mut Object> {
        self.objects.get_mut(id)
    }

    /// Remove an object (for transfer or deletion)
    pub fn remove(&mut self, id: &ObjectID) -> Option<Object> {
        if let Some(object) = self.objects.remove(id) {
            // Remove from owner index
            if let Owner::AddressOwner(owner) = &object.owner {
                if let Some(owned) = self.owner_index.get_mut(owner) {
                    owned.retain(|oid| oid != id);
                }
            }

            // Remove from type index
            if let Some(typed) = self.type_index.get_mut(&object.type_) {
                typed.retain(|oid| oid != id);
            }

            Some(object)
        } else {
            None
        }
    }

    /// Transfer object to a new owner
    pub fn transfer(&mut self, id: &ObjectID, new_owner: AccountAddress) -> Result<()> {
        // Get old owner first
        let old_owner = self.objects
            .get(id)
            .ok_or_else(|| anyhow!("Object not found: {:?}", id))?
            .owner.clone();

        // Remove from old owner index
        if let Owner::AddressOwner(old_owner_addr) = &old_owner {
            if let Some(owned) = self.owner_index.get_mut(old_owner_addr) {
                owned.retain(|oid| oid != id);
            }
        }

        // Update object
        let object = self.objects.get_mut(id).unwrap();
        object.owner = Owner::AddressOwner(new_owner);
        object.increment_version();

        // Add to new owner index
        self.owner_index
            .entry(new_owner)
            .or_insert_with(Vec::new)
            .push(*id);

        Ok(())
    }

    /// Share an object (make it accessible to all)
    pub fn share(&mut self, id: &ObjectID) -> Result<()> {
        // Get old owner first
        let old_owner = self.objects
            .get(id)
            .ok_or_else(|| anyhow!("Object not found: {:?}", id))?
            .owner.clone();

        // Remove from owner index
        if let Owner::AddressOwner(owner_addr) = &old_owner {
            if let Some(owned) = self.owner_index.get_mut(owner_addr) {
                owned.retain(|oid| oid != id);
            }
        }

        // Update object
        let object = self.objects.get_mut(id).unwrap();
        object.owner = Owner::Shared;
        object.increment_version();
        Ok(())
    }

    /// Freeze an object (make it immutable)
    pub fn freeze(&mut self, id: &ObjectID) -> Result<()> {
        // Get old owner first
        let old_owner = self.objects
            .get(id)
            .ok_or_else(|| anyhow!("Object not found: {:?}", id))?
            .owner.clone();

        // Remove from owner index
        if let Owner::AddressOwner(owner_addr) = &old_owner {
            if let Some(owned) = self.owner_index.get_mut(owner_addr) {
                owned.retain(|oid| oid != id);
            }
        }

        // Update object
        let object = self.objects.get_mut(id).unwrap();
        object.owner = Owner::Immutable;
        object.increment_version();
        Ok(())
    }

    /// Get all objects owned by an address
    pub fn get_owned_objects(&self, owner: &AccountAddress) -> Vec<&Object> {
        self.owner_index
            .get(owner)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.objects.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all objects owned by an address (cloned)
    pub fn get_objects_by_owner(&self, owner: &AccountAddress) -> Vec<Object> {
        self.owner_index
            .get(owner)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.objects.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all objects of a specific type owned by an address
    pub fn get_owned_objects_by_type(
        &self,
        owner: &AccountAddress,
        type_: &str,
    ) -> Vec<&Object> {
        self.get_owned_objects(owner)
            .into_iter()
            .filter(|obj| obj.type_ == type_)
            .collect()
    }

    /// Get all objects of a specific type
    pub fn get_objects_by_type(&self, type_: &str) -> Vec<&Object> {
        self.type_index
            .get(type_)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.objects.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if an object exists
    pub fn contains(&self, id: &ObjectID) -> bool {
        self.objects.contains_key(id)
    }

    /// Get total number of objects
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_object(id: u8, owner: AccountAddress) -> Object {
        let id = AccountAddress::from_hex_literal(&format!("0x{:x}", id)).unwrap();
        Object::new(
            id,
            Owner::AddressOwner(owner),
            "0x2::coin::Coin<0x2::kanari::KANARI>".to_string(),
            vec![1, 2, 3],
        )
    }

    #[test]
    fn test_insert_and_get() {
        let mut storage = ObjectStorage::new();
        let owner = AccountAddress::from_hex_literal("0x1").unwrap();
        let obj = create_test_object(1, owner);
        let id = obj.id;

        storage.insert(obj).unwrap();
        assert!(storage.contains(&id));
        assert_eq!(storage.len(), 1);
    }

    #[test]
    fn test_transfer() {
        let mut storage = ObjectStorage::new();
        let owner1 = AccountAddress::from_hex_literal("0x1").unwrap();
        let owner2 = AccountAddress::from_hex_literal("0x2").unwrap();
        let obj = create_test_object(1, owner1);
        let id = obj.id;

        storage.insert(obj).unwrap();
        storage.transfer(&id, owner2).unwrap();

        let transferred = storage.get(&id).unwrap();
        assert!(transferred.is_owned_by(&owner2));
        assert_eq!(transferred.version, 1);
    }

    #[test]
    fn test_get_owned_objects() {
        let mut storage = ObjectStorage::new();
        let owner1 = AccountAddress::from_hex_literal("0x1").unwrap();
        let owner2 = AccountAddress::from_hex_literal("0x2").unwrap();

        storage.insert(create_test_object(1, owner1)).unwrap();
        storage.insert(create_test_object(2, owner1)).unwrap();
        storage.insert(create_test_object(3, owner2)).unwrap();

        let owned = storage.get_owned_objects(&owner1);
        assert_eq!(owned.len(), 2);
    }

    #[test]
    fn test_share_object() {
        let mut storage = ObjectStorage::new();
        let owner = AccountAddress::from_hex_literal("0x1").unwrap();
        let obj = create_test_object(1, owner);
        let id = obj.id;

        storage.insert(obj).unwrap();
        storage.share(&id).unwrap();

        let shared = storage.get(&id).unwrap();
        assert!(shared.is_shared());
        assert_eq!(shared.version, 1);
    }

    #[test]
    fn test_freeze_object() {
        let mut storage = ObjectStorage::new();
        let owner = AccountAddress::from_hex_literal("0x1").unwrap();
        let obj = create_test_object(1, owner);
        let id = obj.id;

        storage.insert(obj).unwrap();
        storage.freeze(&id).unwrap();

        let frozen = storage.get(&id).unwrap();
        assert!(frozen.is_immutable());
        assert_eq!(frozen.version, 1);
    }
}
