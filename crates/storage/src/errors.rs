use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Storage IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
    
    #[error("Module not found: {0}")]
    ModuleNotFound(String),
}