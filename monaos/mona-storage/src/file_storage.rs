use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;
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
    #[error("File not found")]
    InvalidId,

    #[error("File not found")]
    Serialization(serde_json::Error),  // Make sure this variant can hold serde_json::Error


    #[error("File not found")]
    NotFound,

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

// Implement methods for FileStorage
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

    pub fn store(&self, filename: &str, data: &[u8]) -> Result<FileStorage, StorageError2> {
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

    pub fn read(&self, filename: &str) -> Result<Vec<u8>, StorageError2> {
        let file_path = self.path.join(filename);
        match fs::read(&file_path) {
            Ok(data) => Ok(data),
            Err(e) => Err(StorageError2::Io(e)),
        }
    }

    // Check if file exists in storage
    pub fn check_file_exists(&self, file_path: &Path) -> bool {
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

    // file from storage
    pub fn upload(source_path: impl AsRef<Path>, filename: String) -> Result<Self, StorageError2> {
        let source_path = source_path.as_ref();
        
        // Check source file
        if !source_path.exists() {
            return Err(StorageError2::FileNotFound(
                source_path.to_string_lossy().to_string(),
            ));
        }
    
        // Initialize storage
        FileStorage::init_storage()?;
    
        // Generate new UUID for file
        let id = Uuid::new_v4();
        let storage_path = get_storage_path();
        
        // Create paths
        let file_ext = source_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        let dest_filename = format!("{}.{}", id, file_ext);
        let dest_path = storage_path.join(&dest_filename);
        
        // Create metadata
        let metadata = FileMetadata {
            filename,
            size: fs::metadata(source_path)?.len(),
            content_type: mime_guess::from_path(source_path)
                .first_or_octet_stream()
                .to_string(),
            uploaded_at: SystemTime::now(),
        };
    
        // Save file
        fs::copy(source_path, &dest_path)?;
        
        // Save metadata
        let metadata_path = storage_path.join(format!("{}.json", id));
        let metadata_json = serde_json::to_string(&metadata)?;
        fs::write(&metadata_path, metadata_json)?;
    
        Ok(FileStorage {
            id,
            metadata,
            path: dest_path,
            created_at: SystemTime::now(),
        })
    }


    pub fn get_by_id(id_str: &str) -> Result<Self, StorageError2> {
        // Parse UUID
        let id = Uuid::parse_str(id_str)
            .map_err(|_| StorageError2::InvalidId)?;
    
        // Get storage path
        let storage_path = get_storage_path();
        
        // Find file by looking for metadata first
        let metadata_path = storage_path.join(format!("{}.json", id_str));
        if !metadata_path.exists() {
            return Err(StorageError2::NotFound);
        }
    
        // Load metadata
        let metadata = std::fs::read_to_string(&metadata_path)
            .map_err(|e| StorageError2::Io(e))?;
        let metadata: FileMetadata = serde_json::from_str(&metadata)
            .map_err(StorageError2::Serialization)?;
    
        // Find actual file by extension
        let file_ext = Path::new(&metadata.filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        let file_path = storage_path.join(format!("{}.{}", id_str, file_ext));
    
        if !file_path.exists() {
            return Err(StorageError2::NotFound);
        }
    
        Ok(Self {
            id,
            path: file_path,
            metadata,
            created_at: SystemTime::now(),
        })
    }

}

