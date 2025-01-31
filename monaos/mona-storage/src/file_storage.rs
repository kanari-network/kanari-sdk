use std::path::{Path, PathBuf};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileStorage {
    pub id: Uuid,
    pub metadata: FileMetadata,  
    pub path: PathBuf,
    pub created_at: SystemTime,
}


#[derive(Error, Debug)]
pub enum StorageError2 {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("UUID error: {0}")]
    UuidError(#[from] uuid::Error),
    
    #[error("Serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),
    
    #[error("Unknown error")]
    Unknown,

    #[error("File not found: {0}")]
    FileNotFound(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileMetadata {
    pub filename: String,
    pub size: u64,
    pub content_type: String,
    pub uploaded_at: SystemTime,
}

const KARI_DIR: &str = ".kari";
const STORAGE_DIR: &str = "storage";

fn get_storage_path() -> PathBuf {
    let home_dir = dirs::home_dir().expect("Could not find home directory");
    home_dir.join(KARI_DIR).join(STORAGE_DIR)
}


impl FileStorage {
    pub async fn check_file_exists(&self, file_path: &Path) -> bool {
        file_path.exists()
    }

    pub async fn upload_file(&self, source_path: &Path) -> Result<FileStorage, StorageError2> {
        if !self.check_file_exists(source_path).await {
            return Err(StorageError2::FileNotFound(
                source_path.to_string_lossy().to_string(),
            ));
        }

        let filename = source_path
            .file_name()
            .ok_or_else(|| StorageError2::Unknown)?
            .to_string_lossy()
            .to_string();

        let file_size = fs::metadata(source_path).await?.len();
        let storage_path = get_storage_path();
        let unique_filename = format!("{}.{}", Uuid::new_v4(), filename);
        let dest_path = storage_path.join(&unique_filename);

        let metadata = FileMetadata {
            filename,
            size: file_size,
            content_type: mime_guess::from_path(source_path)
                .first_or_octet_stream()
                .to_string(),
            uploaded_at: SystemTime::now(),
        };

        // Copy file to storage
        fs::copy(source_path, &dest_path).await?;

        Ok(FileStorage {
            id: Uuid::new_v4(),
            metadata,
            path: dest_path,
            created_at: SystemTime::now(),
        })
    }
}