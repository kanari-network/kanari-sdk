// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Overlay key-value access and monotonic access versions.

use super::*;

impl StateManager {
    // Helper to write to overlay (pub for genesis module)
    /// Write a serialized value to the overlay buffer.
    pub(crate) fn save_internal<T: Serialize + ?Sized>(
        &mut self,
        key: &[u8],
        value: &T,
    ) -> Result<()> {
        let bytes = bcs::to_bytes(value)?;
        self.overlay.insert(key.to_vec(), Some(bytes));
        Ok(())
    }

    // Helper to read from overlay then store
    /// Read a value from the overlay or persistent store.
    pub fn load_internal<T: DeserializeOwned>(&self, key: &[u8]) -> Result<Option<T>> {
        if let Some(val_opt) = self.overlay.get(key) {
            match val_opt {
                Some(bytes) => return Ok(Some(bcs::from_bytes(bytes)?)),
                None => return Ok(None),
            }
        }
        Ok(self.store.load(key)?)
    }

    /// Capture the versions observed by a speculative execution.
    pub fn capture_access_versions(&self, access: &StateAccessSet) -> BTreeMap<Vec<u8>, u64> {
        access
            .reads
            .iter()
            .chain(access.writes.iter())
            .map(|key| {
                (
                    key.clone(),
                    self.access_versions.get(key).copied().unwrap_or(0),
                )
            })
            .collect()
    }

    /// Get a snapshot of the current access versions.
    pub fn access_version_snapshot(&self) -> BTreeMap<Vec<u8>, u64> {
        self.access_versions.clone()
    }

    /// Get the current access epoch counter.
    pub fn access_epoch(&self) -> u64 {
        self.access_epoch
    }

    /// Validate that an access snapshot is still consistent with current versions.
    pub fn validate_access_snapshot(
        &self,
        snapshot: &BTreeMap<Vec<u8>, u64>,
        access: &StateAccessSet,
    ) -> bool {
        access.reads.iter().chain(access.writes.iter()).all(|key| {
            snapshot.get(key).copied().unwrap_or(0)
                == self.access_versions.get(key).copied().unwrap_or(0)
        })
    }

    /// Bump the monotonic version counters for keys written by the access set.
    pub(crate) fn advance_access_versions(&mut self, access: &StateAccessSet) -> Result<()> {
        let persist_access_versions = std::env::var("KANARI_PERSIST_ACCESS_VERSIONS")
            .map(|value| !matches!(value.as_str(), "0" | "false" | "FALSE" | "no" | "NO"))
            .unwrap_or(true);
        if !access.writes.is_empty() {
            self.access_epoch = self
                .access_epoch
                .checked_add(1)
                .context("State access epoch overflow")?;
        }
        for key in &access.writes {
            let next = self
                .access_versions
                .get(key)
                .copied()
                .unwrap_or(0)
                .checked_add(1)
                .context("State access version overflow")?;
            self.access_versions.insert(key.clone(), next);
            if persist_access_versions {
                let mut persisted_key = ACCESS_VERSION_PREFIX.to_vec();
                persisted_key.extend_from_slice(key);
                self.save_internal(&persisted_key, &next)?;
            }
        }
        Ok(())
    }
}
