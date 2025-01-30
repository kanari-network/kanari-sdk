use std::{fs, path::PathBuf, time::Duration};
use thiserror::Error;
use rocksdb::{DB, Error as RocksError};
use bincode;
mod FileStorage;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    DbError(#[from] RocksError),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] bincode::Error),
}

pub trait BlockchainStorage {
    fn save_data(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError>;
    fn load_data(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError>;
    fn flush(&self) -> Result<(), StorageError>;
}

pub struct RocksDBStorage {
    db: DB,
}

impl RocksDBStorage {
    pub fn new(path: PathBuf) -> Result<Self, StorageError> {
        const MAX_RETRIES: u32 = 5;
        let mut backoff = Duration::from_millis(100);
        
        let mut attempts = 0;
        while attempts < MAX_RETRIES {
            // Cleanup any stale lock files
            let lock_path = path.join("LOCK");
            if lock_path.exists() {
                let _ = fs::remove_file(&lock_path);
                std::thread::sleep(Duration::from_millis(100));
            }

            // Configure and open DB
            let mut opts = rocksdb::Options::default();
            opts.create_if_missing(true);
            opts.set_keep_log_file_num(1);
            opts.set_max_open_files(10);
            opts.set_use_fsync(true);
            
            match DB::open(&opts, &path) {
                Ok(db) => return Ok(Self { db }),
                Err(_) => {
                    attempts += 1;
                    if attempts < MAX_RETRIES {
                        std::thread::sleep(backoff);
                        backoff *= 2; // Exponential backoff
                    }
                }
            }
        }

        match DB::open_default(&path) {
            Ok(_) => unreachable!(),
            Err(e) => Err(StorageError::DbError(e))
        }
    }
}

impl Drop for RocksDBStorage {
    fn drop(&mut self) {
        let _ = self.db.flush();
    }
}

impl BlockchainStorage for RocksDBStorage {
    fn save_data(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        self.db.put(key, value)?;
        Ok(())
    }

    fn load_data(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        match self.db.get(key)? {
            Some(data) => Ok(Some(data)),
            None => Ok(None),
        }
    }

    fn flush(&self) -> Result<(), StorageError> {
        self.db.flush()?;
        Ok(())
    }
}