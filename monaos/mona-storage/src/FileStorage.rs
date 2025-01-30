use std::path::PathBuf;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("File size exceeds limit of {size}MB")]
    FileSizeExceeded { size: u64 },
    #[error("Upload timeout")]
    UploadTimeout,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetadata {
    file_name: String,
    file_size: u64,
    mime_type: String,
    created_at: Option<SystemTime>,
    file_id: String,
}

impl Default for FileMetadata {
    fn default() -> Self {
        Self {
            file_name: String::new(),
            file_size: 0,
            mime_type: String::from("application/octet-stream"),
            created_at: Some(SystemTime::now()),
            file_id: String::new(),
        }
    }
}

#[derive(Debug)]
pub struct FileStorage {
    storage_path: PathBuf,
    max_file_size: u64,
}

impl FileStorage {
    pub fn new(storage_path: PathBuf) -> Self {
        Self {
            storage_path,
            max_file_size: 100 * 1024 * 1024, // 100MB
        }
    }

    pub async fn upload_file(&self, file_path: PathBuf) -> Result<String, StorageError> {
        let metadata = fs::metadata(&file_path).await?;
        
        if metadata.len() > self.max_file_size {
            return Err(StorageError::FileSizeExceeded { 
                size: metadata.len() / (1024 * 1024) 
            });
        }

        let file_id = Uuid::new_v4().to_string();
        let dest_path = self.storage_path.join(&file_id);
        
        fs::create_dir_all(&self.storage_path).await?;
        fs::copy(file_path, dest_path).await?;

        Ok(file_id)
    }

    pub async fn get_file(&self, file_id: &str) -> Result<PathBuf, StorageError> {
        let file_path = self.storage_path.join(file_id);
        if !file_path.exists() {
            return Err(StorageError::FileNotFound(file_id.to_string()));
        }
        Ok(file_path)
    }

    pub async fn delete_file(&self, file_id: &str) -> Result<(), StorageError> {
        let file_path = self.storage_path.join(file_id);
        if !file_path.exists() {
            return Err(StorageError::FileNotFound(file_id.to_string()));
        }
        fs::remove_file(file_path).await?;
        Ok(())
    }
}