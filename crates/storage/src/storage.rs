use async_trait::async_trait;
use move_core_types::language_storage::{ModuleId, StructTag};
use crate::{StoredModule, StoredResource, StorageError};

#[async_trait]
pub trait Storage: Send + Sync {
    async fn get_module(&self, id: &ModuleId) -> Result<Option<StoredModule>, StorageError>;
    async fn get_resource(&self, address: &[u8], tag: &StructTag) -> Result<Option<StoredResource>, StorageError>;
    async fn put_module(&mut self, module: StoredModule) -> Result<(), StorageError>;
    async fn put_resource(&mut self, resource: StoredResource) -> Result<(), StorageError>;
    async fn remove_module(&mut self, id: &ModuleId) -> Result<(), StorageError>;
    async fn remove_resource(&mut self, address: &[u8], tag: &StructTag) -> Result<(), StorageError>;
}