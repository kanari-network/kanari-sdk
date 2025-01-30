use std::path::PathBuf;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs;
use uuid::Uuid;
use rocksdb::{DB, Options};

#[derive(Error, Debug)]
pub enum StorageError2 {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("File size exceeds limit of {size}MB")]
    FileSizeExceeded { size: u64 },
    #[error("Upload timeout")]
    UploadTimeout,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Database error: {0}")]
    DbError(#[from] rocksdb::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] Box<bincode::ErrorKind>),
    #[error("Invalid filename")]
    InvalidFilename,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetadata {
    pub file_name: String,
    pub file_size: u64,
    pub mime_type: String,
    pub created_at: Option<SystemTime>,
    pub file_id: String,
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
    db: DB,
}

impl FileStorage {
    pub fn new(storage_path: PathBuf) -> Result<Self, StorageError2> {
        std::fs::create_dir_all(&storage_path)?;
        
        let db_path = storage_path.join("metadata.db");
        let mut opts = Options::default();
        opts.create_if_missing(true);
        
        let db = DB::open(&opts, db_path)?;

        Ok(Self {
            storage_path,
            max_file_size: 100 * 1024 * 1024,
            db,
        })
    }

    pub async fn upload_file(&self, file_path: PathBuf) -> Result<String, StorageError2> {
        let file_metadata = fs::metadata(&file_path).await?;
        
        if file_metadata.len() > self.max_file_size {
            return Err(StorageError2::FileSizeExceeded { 
                size: file_metadata.len() / (1024 * 1024) 
            });
        }

        let file_id = Uuid::new_v4().to_string();
        let dest_path = self.storage_path.join(&file_id);

        // Copy file first
        fs::copy(&file_path, &dest_path).await?;

        // Save metadata after successful copy
        let metadata = FileMetadata {
            file_name: file_path.file_name()
                .ok_or(StorageError2::InvalidFilename)?
                .to_string_lossy()
                .to_string(),
            file_size: file_metadata.len(),
            mime_type: mime_guess::from_path(&file_path)
                .first_or_octet_stream()
                .to_string(),
            created_at: Some(SystemTime::now()),
            file_id: file_id.clone(),
        };

        self.save_metadata(&metadata).await?;
        Ok(file_id)
    }


    async fn save_metadata(&self, metadata: &FileMetadata) -> Result<(), StorageError2> {
        let key = metadata.file_id.as_bytes();
        let value = bincode::serialize(&metadata)?;
        self.db.put(key, value)?;
        Ok(())
    }

    pub async fn get_file(&self, file_id: &str) -> Result<PathBuf, StorageError2> {
        let metadata = self.get_metadata(file_id).await?;
        let file_path = self.storage_path.join(&metadata.file_id);
        
        if !file_path.exists() {
            return Err(StorageError2::FileNotFound(file_id.to_string()));
        }
        
        Ok(file_path)
    }

    pub async fn get_metadata(&self, file_id: &str) -> Result<FileMetadata, StorageError2> {
        let key = file_id.as_bytes();
        match self.db.get(key)? {
            Some(data) => Ok(bincode::deserialize(&data)?),
            None => Err(StorageError2::FileNotFound(file_id.to_string()))
        }
    }

    pub async fn delete_metadata(&self, file_id: &str) -> Result<(), StorageError2> {
        let key = file_id.as_bytes();
        self.db.delete(key)?;
        Ok(())
    }

    pub async fn delete_file(&self, file_id: &str) -> Result<(), StorageError2> {
        // Delete file first
        let file_path = self.storage_path.join(file_id);
        if !file_path.exists() {
            return Err(StorageError2::FileNotFound(file_id.to_string()));
        }
        fs::remove_file(file_path).await?;

        // Then delete metadata
        self.delete_metadata(file_id).await?;
        Ok(())
    }

}