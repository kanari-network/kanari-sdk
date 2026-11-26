// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Overlay key-value access and monotonic access versions.

use super::*;
use kanari_types::env::env_flag_default_true;

impl StateManager {
    // Helper to write to overlay (pub for genesis module)
    /// Write a serialized value to the overlay buffer.
    pub(crate) fn save_internal<T: Serialize + ?Sized>(
        &mut self,
        key: &[u8],
        value: &T,
    ) -> Result<()> {
        let bytes = bcs::to_bytes(value)?;
        std::sync::Arc::make_mut(&mut self.overlay).insert(key.to_vec(), Some(bytes));
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
            .map(|key| (key.clone(), self.access_versions.get(key).copied().unwrap_or(0)))
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
        self.advance_access_versions_for_keys(access.writes.iter().cloned())
    }

    /// Bump the monotonic version counters for an explicit key iterator.
    ///
    /// Equivalent to building a write-only `StateAccessSet`, but batch fast
    /// paths can pass the keys they already collected without cloning each
    /// key into another wrapper set.
    pub(crate) fn advance_access_versions_for_keys<I>(&mut self, keys: I) -> Result<()>
    where
        I: IntoIterator<Item = Vec<u8>>,
    {
        let writes: BTreeSet<Vec<u8>> = keys.into_iter().collect();
        if writes.is_empty() {
            return Ok(());
        }
        let persist_access_versions = env_flag_default_true("KANARI_PERSIST_ACCESS_VERSIONS");
        let epoch = self.access_epoch.checked_add(1).context("State access epoch overflow")?;
        self.access_epoch = epoch;

        let mut batch = BTreeMap::new();
        for key in &writes {
            let next = self
                .access_versions
                .get(key)
                .copied()
                .unwrap_or(0)
                .checked_add(1)
                .context("State access version overflow")?;
            self.access_versions.insert(key.clone(), next);
            if persist_access_versions {
                batch.insert(key.clone(), next);
            }
        }

        if persist_access_versions {
            // Persist one ordered delta blob per batch instead of one overlay
            // entry per state key. Restart recovery replays batches in epoch
            // order, so the same versions are restored without amplifying the
            // overlay/root hot path.
            let mut batch_key = ACCESS_VERSION_BATCH_PREFIX.to_vec();
            batch_key.extend_from_slice(&epoch.to_be_bytes());
            self.save_internal(&batch_key, &batch)?;
            self.save_internal(ACCESS_EPOCH_KEY, &epoch)?;
        }
        Ok(())
    }
}
