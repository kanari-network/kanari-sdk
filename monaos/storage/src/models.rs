use serde::{Deserialize, Serialize};
use move_core_types::language_storage::{ModuleId, StructTag};

#[derive(Debug, Serialize, Deserialize)]
pub struct StoredResource {
    pub address: Vec<u8>,
    pub tag: StructTag,
    pub data: Vec<u8>,
    pub version: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StoredModule {
    pub id: ModuleId,
    pub bytecode: Vec<u8>,
    pub version: u64,
}