use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageConfig {
    pub storage_path: PathBuf,
    pub max_size: usize,
    pub cache_size: usize,
    pub backup_enabled: bool,
}