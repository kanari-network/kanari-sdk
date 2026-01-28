// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};

/// Persistent peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub last_seen: u64,
}

/// Persistent peer storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerStore {
    pub peers: HashMap<String, PeerInfo>,
    #[serde(skip)]
    pub file_path: PathBuf,
}

impl PeerStore {
    /// Create a new peer store with specified file path
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            peers: HashMap::new(),
            file_path,
        }
    }

    /// Load peer store from disk and filter out old peers
    pub fn load(file_path: PathBuf) -> Result<Self> {
        if file_path.exists() {
            let contents = fs::read_to_string(&file_path)?;
            let mut store: PeerStore = serde_json::from_str(&contents)?;
            store.file_path = file_path;

            // Filter out peers older than 24 hours
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let old_count = store.peers.len();
            store.peers.retain(|_, info| {
                // Keep peers seen in last 24 hours (86400 seconds)
                // If last_seen is 0 (legacy), keep them for now or discard? Let's discard if strictly checking.
                // But for safety, let's say if 0, update to now? No, that's fake.
                // Let's keep if diff < 86400.
                now.saturating_sub(info.last_seen) < 86400
            });
            let new_count = store.peers.len();

            if old_count != new_count {
                info!(
                    "Loaded {} peers from disk (discarded {} old peers)",
                    new_count,
                    old_count - new_count
                );
            } else {
                info!("Loaded {} peers from disk", store.peers.len());
            }

            Ok(store)
        } else {
            info!("No existing peer store found, creating new one");
            Ok(Self::new(file_path))
        }
    }

    /// Save peer store to disk
    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&self.file_path, json)?;
        info!("Saved {} peers to disk", self.peers.len());
        Ok(())
    }

    /// Add or update a peer
    pub fn add_peer(&mut self, peer_id: PeerId, addresses: Vec<Multiaddr>) {
        let peer_id_str = peer_id.to_string();
        let addr_strs: Vec<String> = addresses.iter().map(|a| a.to_string()).collect();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // If peer exists, update it. Only update addresses if we have new ones.
        if let Some(existing) = self.peers.get_mut(&peer_id_str) {
            existing.last_seen = timestamp;
            if !addr_strs.is_empty() {
                // Merge new addresses avoiding duplicates
                for new_addr in addr_strs {
                    if !existing.addresses.contains(&new_addr) {
                        existing.addresses.push(new_addr);
                    }
                }
            }
        } else {
            // New peer
            self.peers.insert(
                peer_id_str.clone(),
                PeerInfo {
                    peer_id: peer_id_str,
                    addresses: addr_strs,
                    last_seen: timestamp,
                },
            );
        }
    }

    /// Remove a peer
    #[allow(dead_code)]
    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        let peer_id_str = peer_id.to_string();
        self.peers.remove(&peer_id_str);
    }

    /// Get all known peer addresses for reconnection
    #[allow(dead_code)]
    pub fn get_peer_addresses(&self) -> Vec<Multiaddr> {
        let mut addresses = Vec::new();

        for peer_info in self.peers.values() {
            for addr_str in &peer_info.addresses {
                if let Ok(addr) = addr_str.parse::<Multiaddr>() {
                    addresses.push(addr);
                } else {
                    warn!("Failed to parse peer address: {}", addr_str);
                }
            }
        }

        addresses
    }

    /// Clean up old peers (not seen in last 7 days)
    pub fn cleanup_old_peers(&mut self, max_age_secs: u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let before_count = self.peers.len();
        self.peers
            .retain(|_, peer| now - peer.last_seen < max_age_secs);
        let removed = before_count - self.peers.len();

        if removed > 0 {
            info!("Cleaned up {} old peers", removed);
        }
    }

    /// Get default file path for peer store
    pub fn default_path(data_dir: &str) -> PathBuf {
        PathBuf::from(data_dir).join("peers.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_peer_store_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("peers.json");

        // Create and save
        let mut store = PeerStore::new(file_path.clone());
        let peer_id = PeerId::random();
        store.add_peer(peer_id, vec![]);
        store.save().unwrap();

        // Load
        let loaded = PeerStore::load(file_path).unwrap();
        assert_eq!(loaded.peers.len(), 1);
    }
}
