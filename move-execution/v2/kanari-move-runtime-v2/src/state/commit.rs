// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Committing the overlay to persistent storage with SMT updates.

use super::*;

impl StateManager {
    /// Commit pending overlay changes to the persistent store and update SMT
    pub fn commit(&mut self) -> Result<()> {
        self.commit_with_raw_updates(Vec::new())
    }

    /// Commit canonical state together with durable metadata owned by the caller.
    /// The extra records share the same RocksDB write batch as the state overlay.
    pub fn commit_with_raw_update(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        self.commit_with_raw_updates(vec![(key, value)])
    }

    /// Commit raw changes with a verified root and precomputed SMT changes.
    pub fn commit_with_raw_changes_verified_root_and_smt_changes(
        &mut self,
        updates: Vec<(Vec<u8>, Vec<u8>)>,
        deletes: Vec<Vec<u8>>,
        verified_root: &[u8],
        smt_changes: Option<PrecomputedSmtChanges>,
    ) -> Result<()> {
        let root: [u8; 32] = verified_root
            .try_into()
            .context("Verified state root must be 32 bytes")?;
        self.commit_with_raw_updates_and_root(updates, deletes, Some(root), smt_changes)
    }

    pub(crate) fn commit_with_raw_updates(
        &mut self,
        updates: Vec<(Vec<u8>, Vec<u8>)>,
    ) -> Result<()> {
        self.commit_with_raw_updates_and_root(updates, Vec::new(), None, None)
    }

    pub(crate) fn commit_with_raw_updates_and_root(
        &mut self,
        mut updates: Vec<(Vec<u8>, Vec<u8>)>,
        mut deletes: Vec<Vec<u8>>,
        verified_root: Option<[u8; 32]>,
        precomputed_smt_changes: Option<PrecomputedSmtChanges>,
    ) -> Result<()> {
        for (key, val_opt) in &self.overlay {
            if let Some(val) = val_opt {
                updates.push((key.clone(), val.clone()));
            } else {
                deletes.push(key.clone());
            }
        }

        // Derive SMT deltas before writing the overlay so index transitions can
        // compare the old persisted membership with the new overlay membership.
        let smt_changes = if self.smt.is_some() {
            match precomputed_smt_changes {
                Some(changes) => Some(changes),
                None => {
                    let (updates, deletes) = self.smt_changes_from_overlay()?;
                    let node_overlay = if let Some(smt) = &self.smt {
                        if let Some(root) = verified_root {
                            let (computed_root, overlay) =
                                smt.root_hash_with_changes_and_overlay(&updates, &deletes)?;
                            ensure!(
                                computed_root == root,
                                "verified SMT root mismatch while precomputing node overlay"
                            );
                            overlay
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                    Some((updates, deletes, node_overlay))
                }
            }
        } else {
            None
        };

        // Update SMT if available
        if let (Some(smt), Some((smt_updates, smt_deletes, node_overlay))) =
            (&self.smt, smt_changes)
        {
            if let (Some(root), Some(db)) = (verified_root, self.store.get_db())
                && !node_overlay.is_empty()
            {
                let mut batch = WriteBatch::default();
                for (key, value) in &updates {
                    batch.put(key, value);
                }
                for key in &deletes {
                    batch.delete(key);
                }
                smt.stage_changes_with_precomputed_overlay(
                    &mut batch,
                    &smt_updates,
                    &smt_deletes,
                    root,
                    &node_overlay,
                )?;
                db.write(batch)?;
                smt.apply_precomputed_node_overlay(node_overlay);
                self.overlay.clear();
                return Ok(());
            }

            self.store.apply_raw_changes(&updates, &deletes)?;
            let update_result = (|| -> Result<()> {
                if let Some(root) = verified_root {
                    if node_overlay.is_empty() {
                        smt.apply_changes_with_root(&smt_updates, &smt_deletes, root)?;
                    } else {
                        smt.apply_changes_with_precomputed_overlay(
                            &smt_updates,
                            &smt_deletes,
                            root,
                            node_overlay,
                        )?;
                    }
                } else if !smt_deletes.is_empty() {
                    smt.delete(&smt_deletes)?;
                    if !smt_updates.is_empty() {
                        smt.insert(&smt_updates)?;
                    }
                } else if !smt_updates.is_empty() {
                    smt.insert(&smt_updates)?;
                }
                Ok(())
            })();
            if let Err(update_error) = update_result {
                // Canonical state has already committed. Repair immediately so callers
                // never receive a simple failure while the live DB remains split-brain.
                self.repair_persisted_smt().with_context(|| {
                    format!("SMT delta update failed ({update_error}); full repair also failed")
                })?;
            }
        } else {
            self.store.apply_raw_changes(&updates, &deletes)?;
        }

        self.overlay.clear();
        Ok(())
    }
}
