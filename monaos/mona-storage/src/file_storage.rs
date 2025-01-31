use std::path::{Path, PathBuf};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::fs;
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
    let kari_path = home_dir.join(KARI_DIR);
    let storage_path = kari_path.join(STORAGE_DIR);
    
    // Create directories if they don't exist
    if !kari_path.exists() {
        std::fs::create_dir_all(&kari_path).expect("Failed to create .kari directory");
    }
    if !storage_path.exists() {
        std::fs::create_dir_all(&storage_path).expect("Failed to create storage directory");
    }
    
    storage_path
}



impl FileStorage {
    pub fn init_storage() -> std::io::Result<()> {
        let home = dirs::home_dir().expect("Could not find home directory");
        let kari_dir = home.join(".kari");
        let storage_dir = kari_dir.join("storage");
    
        // Create .kari directory if it doesn't exist
        if !kari_dir.exists() {
            fs::create_dir_all(&kari_dir)?;
        }
    
        // Create storage directory if it doesn't exist 
        if !storage_dir.exists() {
            fs::create_dir_all(&storage_dir)?;
        }
    
        Ok(())
    }


    pub fn new() -> Result<Self, StorageError2> {
        let path = get_storage_path();
        
        Ok(FileStorage { 
            id: Uuid::new_v4(),
            metadata: FileMetadata {
                filename: String::from(""),
                size: 0,
                content_type: String::from(""),
                uploaded_at: SystemTime::now(),
            },
            path,
            created_at: SystemTime::now(),
        })
    }

    pub async fn store(&self, filename: &str, data: &[u8]) -> Result<FileStorage, StorageError2> {
        let file_path = self.path.join(filename);
        fs::write(&file_path, data)?;

        Ok(FileStorage {
            id: self.id,
            metadata: FileMetadata {
                filename: filename.to_string(),
                size: data.len() as u64,
                content_type: mime_guess::from_path(filename)
                    .first_or_octet_stream()
                    .to_string(),
                uploaded_at: SystemTime::now(),
            },
            path: file_path,
            created_at: SystemTime::now(),
        })
    }

    pub async fn read(&self, filename: &str) -> Result<Vec<u8>, StorageError2> {
        let file_path = self.path.join(filename);
        match fs::read(&file_path) {
            Ok(data) => Ok(data),
            Err(e) => Err(StorageError2::Io(e)),
        }
    }

    // Check if file exists in storage
    pub async fn check_file_exists(&self, file_path: &Path) -> bool {
        if let Ok(canonical_path) = file_path.canonicalize() {
            canonical_path.exists()
        } else {
            false
        }
    }

    // Get complete storage path for a file
    pub fn get_file_path(&self, filename: &str) -> PathBuf {
        get_storage_path().join(filename)
    }

    pub async fn upload(source_path: impl AsRef<Path>, filename: String) -> Result<Self, StorageError2> {
        let source_path = source_path.as_ref();
        
        // Check if source file exists
        if !source_path.exists() {
            return Err(StorageError2::FileNotFound(
                source_path.to_string_lossy().to_string()
            ));
        }

        // Ensure storage directory exists
        FileStorage::init_storage()?;

        let file_size = fs::metadata(source_path)?.len();
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

        // Copy file to storage with error handling
        match fs::copy(source_path, &dest_path) {
            Ok(_) => Ok(FileStorage {
                id: Uuid::new_v4(),
                metadata,
                path: dest_path,
                created_at: SystemTime::now(),
            }),
            Err(e) => Err(StorageError2::Io(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_storage_path_creation() {
        let path = get_storage_path();
        assert!(path.exists(), "Storage directory should be created");
        assert!(path.is_dir(), "Storage path should be a directory");
        
        let kari_dir = path.parent().unwrap();
        assert!(kari_dir.exists(), ".kari directory should be created");
        assert!(kari_dir.is_dir(), ".kari path should be a directory");
    }


    fn create_test_png_data() -> Vec<u8> {
        vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
    }

    async fn setup() -> FileStorage {
        FileStorage::new().expect("Failed to create storage")
    }

    async fn cleanup(storage: &FileStorage) {
        fs::remove_dir_all(&storage.path).unwrap_or_default();
    }

    #[tokio::test]
    async fn test_store_png_file() {
        let storage = setup().await;
        let test_data = create_test_png_data();
        let filename = "test_image.png";

        let result = storage.store(filename, &test_data).await;
        assert!(result.is_ok());

        let stored_path = storage.path.join(filename);
        assert!(stored_path.exists());

        cleanup(&storage).await;
    }

    #[tokio::test]
    async fn test_read_stored_png() {
        let storage = setup().await;
        let test_data = create_test_png_data();
        let filename = "test_read.png";

        storage.store(filename, &test_data).await.unwrap();
        let read_result = storage.read(filename).await;
        
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), test_data);

        cleanup(&storage).await;
    }

    #[tokio::test]
    async fn test_invalid_png_data() {
        let storage = setup().await;
        let invalid_data = vec![0x00, 0x01, 0x02];
        let filename = "invalid.png";

        let result = storage.store(filename, &invalid_data).await;
        assert!(result.is_ok());

        cleanup(&storage).await;
    }

    #[tokio::test]
    async fn test_file_metadata() {
        let storage = setup().await;
        let test_data = create_test_png_data();
        let filename = "metadata_test.png";

        let result = storage.store(filename, &test_data).await.unwrap();
        
        assert_eq!(result.metadata.filename, filename);
        assert_eq!(result.metadata.size, test_data.len() as u64);
        assert_eq!(result.metadata.content_type, "image/png");
        assert!(result.metadata.uploaded_at <= SystemTime::now());

        cleanup(&storage).await;
    }
}