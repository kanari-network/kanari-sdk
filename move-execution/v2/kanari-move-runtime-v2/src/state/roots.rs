// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Sparse Merkle tree maintenance: schema, consistency, and overlay deltas.

use super::*;

impl StateManager {
    pub(crate) fn migrate_runtime_state_schema(&mut self) -> Result<()> {
        let persisted = self
            .load_internal::<u32>(RUNTIME_STATE_SCHEMA_KEY)?
            .unwrap_or(0);
        ensure!(
            persisted <= RUNTIME_STATE_SCHEMA_VERSION,
            "State database schema {} is newer than runtime schema {}",
            persisted,
            RUNTIME_STATE_SCHEMA_VERSION
        );
        if persisted < RUNTIME_STATE_SCHEMA_VERSION {
            self.save_internal(RUNTIME_STATE_SCHEMA_KEY, &RUNTIME_STATE_SCHEMA_VERSION)?;
            self.commit()?;
        }
        Ok(())
    }

    pub(crate) fn ensure_smt_initialized(&self) -> Result<()> {
        let Some(smt) = &self.smt else {
            return Ok(());
        };
        if smt.root_hash()? != smt::default_hashes()[0] {
            return Ok(());
        }

        let mut entries: BTreeMap<Vec<u8>, Vec<u8>> = self
            .store
            .logical_entries()
            .context("Failed to read state entries for SMT rebuild")?
            .into_iter()
            .collect();
        Self::retain_canonical_state_root_entries(&mut entries)?;
        if entries.is_empty() {
            return Ok(());
        }

        let updates = entries.into_iter().collect::<Vec<_>>();
        smt.insert(&updates)?;
        Ok(())
    }

    pub(crate) fn canonical_persisted_entries(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut entries: BTreeMap<Vec<u8>, Vec<u8>> = self
            .store
            .logical_entries()
            .context("Failed to read canonical state entries")?
            .into_iter()
            .collect();
        Self::retain_canonical_state_root_entries(&mut entries)?;
        Ok(entries.into_iter().collect())
    }

    pub(crate) fn ensure_smt_consistent(&self) -> Result<()> {
        ensure!(
            self.overlay.is_empty(),
            "Cannot verify SMT with a pending overlay"
        );
        self.repair_persisted_smt().map(|_| ())
    }

    /// Reconcile the persisted SMT base after a component writes canonical
    /// Move modules directly to the shared store.
    pub fn repair_persisted_smt(&self) -> Result<bool> {
        let Some(smt) = &self.smt else {
            return Ok(false);
        };
        let entries = self.canonical_persisted_entries()?;
        let expected = smt::compute_sparse_root(&entries);
        if smt.root_hash()? != expected {
            log::warn!("Persisted SMT is stale; rebuilding canonical state tree");
            smt.rebuild(&entries)?;
            ensure!(
                smt.root_hash()? == expected,
                "SMT rebuild produced wrong root"
            );
            return Ok(true);
        }
        Ok(false)
    }

    /// Audit the persisted incremental tree against canonical state. This is a
    /// maintenance/diagnostic operation and intentionally performs a full scan.
    pub fn validate_smt_consistency(&self) -> Result<()> {
        let Some(smt) = &self.smt else {
            return Ok(());
        };
        ensure!(
            self.overlay.is_empty(),
            "Cannot audit SMT with a pending overlay"
        );
        let entries = self.canonical_persisted_entries()?;
        let mut mismatches = Vec::new();
        for (key, expected) in &entries {
            match smt.get(key)? {
                Some(actual) if actual == *expected => {}
                Some(actual) => mismatches.push(format!(
                    "{}(expected_bytes={}, actual_bytes={})",
                    String::from_utf8_lossy(key),
                    expected.len(),
                    actual.len()
                )),
                None => mismatches.push(format!("{}(missing)", String::from_utf8_lossy(key))),
            }
            if mismatches.len() == 8 {
                break;
            }
        }
        ensure!(
            mismatches.is_empty(),
            "SMT leaf mismatch for canonical keys: {}",
            mismatches.join(", ")
        );
        let expected_root = smt::compute_sparse_root(&entries);
        let stale_known_keys = if smt.persisted_leaf_count()? > entries.len() {
            let canonical_keys = entries
                .iter()
                .map(|(key, _)| key.as_slice())
                .collect::<HashSet<_>>();
            let mut stale_keys = Vec::new();
            for (key, _) in self
                .store
                .logical_entries()?
                .into_iter()
                .filter(|(key, _)| !canonical_keys.contains(key.as_slice()))
            {
                if smt.get(&key)?.is_none() {
                    continue;
                }

                let label = String::from_utf8_lossy(&key).into_owned();
                let label = if key.starts_with(b"object:") {
                    self.store
                        .load::<StoredObject>(&key)
                        .with_context(|| format!("Failed to decode stale SMT object leaf {label}"))?
                        .map(|object| {
                            format!(
                                "{}[owner={},kind={:?},type={}]",
                                label,
                                object.owner.to_hex_literal(),
                                object.owner_kind,
                                object.type_name
                            )
                        })
                        .unwrap_or(label)
                } else {
                    label
                };
                stale_keys.push(label);
                if stale_keys.len() == 8 {
                    break;
                }
            }
            stale_keys
        } else {
            Vec::new()
        };
        ensure!(
            smt.root_hash()? == expected_root,
            "SMT root mismatch after canonical leaf audit: canonical_leaves={} persisted_leaves={} stale_known_keys={}",
            entries.len(),
            smt.persisted_leaf_count()?,
            stale_known_keys.join(",")
        );
        Ok(())
    }

    /// Return read-only SMT status. When `audit` is true this also performs the
    /// expensive full canonical leaf/root comparison; it never repairs state.
    pub fn smt_diagnostics(&self, audit: bool) -> Result<SmtDiagnostics> {
        let (updates, deletes) = self.smt_changes_from_overlay()?;
        let enabled = self.smt.is_some();
        let persisted_root = self
            .smt
            .as_ref()
            .map(|tree| tree.root_hash().map(hex::encode))
            .transpose()?;

        let mut persisted_leaf_count = None;
        let mut consistent = None;
        let mut consistency_error = None;
        let audit_performed = audit && enabled;

        if audit_performed {
            persisted_leaf_count = self
                .smt
                .as_ref()
                .map(|tree| tree.persisted_leaf_count())
                .transpose()?;
            match self.validate_smt_consistency() {
                Ok(()) => consistent = Some(true),
                Err(error) => {
                    consistent = Some(false);
                    consistency_error = Some(error.to_string());
                }
            }
        }

        Ok(SmtDiagnostics {
            enabled,
            persisted_root,
            effective_root: hex::encode(self.try_compute_state_root()?),
            overlay_entries: self.overlay.len(),
            overlay_updates: updates.len(),
            overlay_deletes: deletes.len(),
            canonical_membership_changed: self.canonical_membership_changed()?,
            runtime_schema_version: self.load_internal(RUNTIME_STATE_SCHEMA_KEY)?,
            expected_runtime_schema_version: RUNTIME_STATE_SCHEMA_VERSION,
            wallet_supply_index_version: self.load_internal(WALLET_SUPPLY_INDEX_VERSION_KEY)?,
            expected_wallet_supply_index_version: WALLET_SUPPLY_INDEX_VERSION,
            audit_requested: audit,
            audit_performed,
            persisted_leaf_count,
            consistent,
            consistency_error,
        })
    }

    pub(crate) fn is_canonical_smt_update(&self, key: &[u8], value: &[u8]) -> Result<bool> {
        if Self::is_canonical_state_root_key(key) {
            return Ok(true);
        }
        if let Some(module_id) = key.strip_prefix(b"module:")
            && let Ok(module_id) = std::str::from_utf8(module_id)
        {
            return Ok(self
                .load_index_list(b"module_index")?
                .iter()
                .any(|id| id == &format!("module:{module_id}")));
        }
        if let Some(object_id) = key.strip_prefix(b"object:")
            && let Ok(object_id) = std::str::from_utf8(object_id)
            && let object = bcs::from_bytes::<StoredObject>(value)
                .with_context(|| format!("Malformed object record {object_id}"))?
            && matches!(object.owner_kind, ObjectOwnerKind::AddressOwner(_))
        {
            let canonical_id = canonical_object_id(object_id)
                .ok_or_else(|| anyhow::anyhow!("Invalid canonical object id {object_id}"))?;
            return Ok(self
                .load_index_list(&owned_objects_key(&object.owner))?
                .binary_search(&canonical_id)
                .is_ok());
        }
        Ok(false)
    }

    pub(crate) fn smt_changes_from_overlay(&self) -> Result<OverlaySmtChanges> {
        let mut updates = Vec::new();
        let mut deletes = Vec::new();

        for (key, value_opt) in &self.overlay {
            match value_opt {
                Some(value) => {
                    let canonical = if let Some(object_id) = key.strip_prefix(b"object:")
                        && let Ok(object_id) = std::str::from_utf8(object_id)
                    {
                        let object = bcs::from_bytes::<StoredObject>(value)
                            .with_context(|| format!("Malformed object record {object_id}"))?;
                        if matches!(object.owner_kind, ObjectOwnerKind::AddressOwner(_)) {
                            canonical_object_id(object_id).ok_or_else(|| {
                                anyhow::anyhow!("Invalid canonical object id {object_id}")
                            })? == object_id
                        } else {
                            false
                        }
                    } else {
                        self.is_canonical_smt_update(key, value)?
                    };
                    if canonical {
                        updates.push((key.clone(), value.clone()));
                    } else if key.starts_with(b"module:") || key.starts_with(b"object:") {
                        deletes.push(key.clone());
                    }
                }
                // An object/module may transition out of the canonical root
                // set (for example AddressOwner -> Shared). A non-canonical
                // replacement must remove the previously indexed leaf.
                None if Self::is_canonical_state_root_key(key)
                    || key.starts_with(b"module:")
                    || key.starts_with(b"object:") =>
                {
                    deletes.push(key.clone());
                }
                _ => {}
            }
        }

        // Object membership in the canonical root is driven by owned_objects
        // indexes. An index can drop/add an unchanged object without placing
        // that object's bytes in the overlay, so derive those leaf deltas too.
        for (index_key, value_opt) in self
            .overlay
            .iter()
            .filter(|(key, _)| key.starts_with(b"owned_objects:"))
        {
            let old_ids = self
                .store
                .load::<Vec<String>>(index_key)?
                .unwrap_or_default()
                .into_iter()
                .filter_map(|id| canonical_object_id(&id))
                .collect::<BTreeSet<_>>();
            let new_ids = value_opt
                .as_ref()
                .map(|bytes| {
                    bcs::from_bytes::<Vec<String>>(bytes)
                        .context("Malformed pending owned-object index")
                })
                .transpose()?
                .unwrap_or_default()
                .into_iter()
                .filter_map(|id| canonical_object_id(&id))
                .collect::<BTreeSet<_>>();

            for removed in old_ids.difference(&new_ids) {
                deletes.push(object_key(removed));
            }
            for added in new_ids.difference(&old_ids) {
                let key = object_key(added);
                if let Some(object) = self.load_internal::<StoredObject>(&key)?
                    && matches!(object.owner_kind, ObjectOwnerKind::AddressOwner(_))
                {
                    let value = bcs::to_bytes(&object)?;
                    updates.push((key, value));
                }
            }
        }

        updates.sort_by(|left, right| left.0.cmp(&right.0));
        updates.dedup_by(|left, right| left.0 == right.0);
        let updated_keys = updates
            .iter()
            .map(|(key, _)| key.as_slice())
            .collect::<HashSet<_>>();
        deletes.retain(|key| !updated_keys.contains(key.as_slice()));
        deletes.sort();
        deletes.dedup();

        Ok((updates, deletes))
    }

    pub(crate) fn canonical_membership_changed(&self) -> Result<bool> {
        for (key, value_opt) in &self.overlay {
            if key != b"module_index" && !key.starts_with(b"owned_objects:") {
                continue;
            }
            let old = self.store.load::<Vec<String>>(key)?;
            let new = value_opt
                .as_ref()
                .map(|bytes| {
                    bcs::from_bytes::<Vec<String>>(bytes)
                        .context("Malformed pending canonical membership index")
                })
                .transpose()?;
            if old != new {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Compute a canonical root and propagate storage/index corruption to the caller.
    pub fn try_compute_state_root(&self) -> Result<Vec<u8>> {
        self.try_compute_state_root_with_smt_changes()
            .map(|(root, _)| root)
    }

    /// Compute a canonical root and return reusable SMT overlay deltas when the
    /// production SMT backend is active. Passing these deltas into commit avoids
    /// walking and decoding the same overlay twice on the checkpoint hot path.
    pub fn try_compute_state_root_with_smt_changes(
        &self,
    ) -> Result<(Vec<u8>, Option<PrecomputedSmtChanges>)> {
        let profile_root = matches!(
            std::env::var("KANARI_STATE_ROOT_PROFILE").as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
        );
        let started_at = std::time::Instant::now();
        if let Some(smt) = &self.smt {
            let (updates, deletes) = self.smt_changes_from_overlay()?;
            let (root, node_overlay) =
                smt.root_hash_with_changes_and_overlay(&updates, &deletes)?;
            let root = root.to_vec();
            if profile_root {
                eprintln!(
                    "state root profile: smt updates={} deletes={} total={:.6}s",
                    updates.len(),
                    deletes.len(),
                    started_at.elapsed().as_secs_f64()
                );
            }
            return Ok((root, Some((updates, deletes, node_overlay))));
        }
        let logical_started_at = std::time::Instant::now();
        let mut entries: BTreeMap<Vec<u8>, Vec<u8>> =
            self.store.logical_entries()?.into_iter().collect();
        let logical_count = entries.len();
        let logical_at = std::time::Instant::now();

        for (key, value_opt) in &self.overlay {
            if let Some(value) = value_opt {
                entries.insert(key.clone(), value.clone());
            } else {
                entries.remove(key);
            }
        }
        let overlay_at = std::time::Instant::now();

        Self::retain_canonical_state_root_entries(&mut entries)?;
        let retain_at = std::time::Instant::now();

        let canonical_count = entries.len();
        let root = smt::compute_sparse_root(&entries.into_iter().collect::<Vec<_>>()).to_vec();
        let rooted_at = std::time::Instant::now();
        if profile_root {
            eprintln!(
                "state root profile: logical_entries={} overlay_entries={} canonical_entries={} logical={:.6}s overlay={:.6}s retain={:.6}s compute={:.6}s total={:.6}s",
                logical_count,
                self.overlay.len(),
                canonical_count,
                logical_at.duration_since(logical_started_at).as_secs_f64(),
                overlay_at.duration_since(logical_at).as_secs_f64(),
                retain_at.duration_since(overlay_at).as_secs_f64(),
                rooted_at.duration_since(retain_at).as_secs_f64(),
                rooted_at.duration_since(started_at).as_secs_f64(),
            );
        }
        Ok((root, None))
    }

    /// Legacy convenience API. Consensus and RPC paths must use
    /// `try_compute_state_root` so persistent-state faults are returned rather
    /// than converted into a different root.
    pub fn compute_state_root(&self) -> Vec<u8> {
        self.try_compute_state_root().unwrap_or_else(|e| {
            // Use a distinct sentinel so callers can distinguish a corrupted
            // fallback from a genuinely empty state root (all zeros).
            log::error!(
                "CRITICAL: Failed to compute state root - storage may be corrupted: {}. \
                 Returning sentinel root [0xFF; 32] to signal corruption.",
                e
            );
            vec![0xFF; 32]
        })
    }

    /// Get a snapshot of the canonical state entries including pending overlay.
    pub fn try_canonical_state_snapshot(&self) -> Result<BTreeMap<Vec<u8>, Vec<u8>>> {
        let mut entries: BTreeMap<Vec<u8>, Vec<u8>> =
            self.store.logical_entries()?.into_iter().collect();

        for (key, value_opt) in &self.overlay {
            if let Some(value) = value_opt {
                entries.insert(key.clone(), value.clone());
            } else {
                entries.remove(key);
            }
        }

        Self::retain_canonical_state_root_entries(&mut entries)?;
        Ok(entries)
    }

    /// Get a snapshot of the canonical state, returning empty on error.
    pub fn canonical_state_snapshot(&self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        self.try_canonical_state_snapshot().unwrap_or_else(|e| {
            log::error!(
                "CRITICAL: Failed to compute canonical state snapshot - storage may be corrupted: {}",
                e
            );
            // Return empty snapshot as fallback instead of panicking
            BTreeMap::new()
        })
    }

    /// Get the total number of owners with persisted owner state.
    pub fn try_owner_count(&self) -> Result<usize> {
        Ok(self.load_owner_index_ids()?.len())
    }

    /// Legacy convenience API for callers that cannot return errors. Consensus,
    /// diagnostics, and RPC paths should prefer `try_owner_count`.
    pub fn owner_count(&self) -> usize {
        match self.try_owner_count() {
            Ok(count) => count,
            Err(error) => {
                log::error!("[StateManager] Failed to load owner count: {error:#}");
                0
            }
        }
    }
}
