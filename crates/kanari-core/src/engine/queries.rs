// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use kanari_rpc_api::ObjectInfo;
use kanari_rpc_api::{
    BlockData, BlockchainStats, CanonicalStateDiffResponse, CanonicalStateEntry,
    CanonicalStateSnapshotResponse, CompareCanonicalStateSnapshotRequest, FullBlockData, OwnerInfo,
    RpcObjectOwnerKindFilter, SmtStatusResponse,
};
use kanari_types::address::Address as KanariAddress;
use kanari_types::transaction::{ObjectOwnerKind, ObjectRef};
use log::{info, warn};

use super::*;
use crate::{BlockchainEngine, Checkpoint, CheckpointSyncData};

impl BlockchainEngine {
    pub fn smt_status(&self, audit: bool) -> Result<SmtStatusResponse> {
        let diagnostics = self.state_read().smt_diagnostics(audit)?;
        let stats = self.get_stats();
        Ok(SmtStatusResponse {
            height: stats.height,
            checkpoint_state_root: stats.state_root,
            enabled: diagnostics.enabled,
            persisted_root: diagnostics.persisted_root,
            effective_root: diagnostics.effective_root,
            overlay_entries: diagnostics.overlay_entries,
            overlay_updates: diagnostics.overlay_updates,
            overlay_deletes: diagnostics.overlay_deletes,
            canonical_membership_changed: diagnostics.canonical_membership_changed,
            runtime_schema_version: diagnostics.runtime_schema_version,
            expected_runtime_schema_version: diagnostics.expected_runtime_schema_version,
            wallet_supply_index_version: diagnostics.wallet_supply_index_version,
            expected_wallet_supply_index_version: diagnostics.expected_wallet_supply_index_version,
            audit_requested: diagnostics.audit_requested,
            audit_performed: diagnostics.audit_performed,
            persisted_leaf_count: diagnostics.persisted_leaf_count,
            consistent: diagnostics.consistent,
            consistency_error: diagnostics.consistency_error,
        })
    }

    pub fn canonical_state_snapshot_entries(
        &self,
        limit: Option<usize>,
        prefix: Option<&str>,
    ) -> Vec<CanonicalStateEntry> {
        let state = self.state_read();
        let mut entries = state
            .canonical_state_snapshot()
            .into_iter()
            .map(|(key, value)| CanonicalStateEntry {
                key: String::from_utf8_lossy(&key).into_owned(),
                value: hex::encode(value),
            })
            .filter(|entry| match prefix {
                Some(prefix) => entry.key.starts_with(prefix),
                None => true,
            })
            .collect::<Vec<_>>();
        if let Some(limit) = limit {
            entries.truncate(limit);
        }
        entries
    }

    pub fn canonical_state_snapshot_response(
        &self,
        limit: Option<usize>,
        prefix: Option<&str>,
    ) -> CanonicalStateSnapshotResponse {
        let entries = self.canonical_state_snapshot_entries(limit, prefix);
        CanonicalStateSnapshotResponse {
            height: self.get_stats().height,
            state_root: self.latest_checkpoint_state_root_hex(),
            entry_count: entries.len(),
            entries,
        }
    }

    pub fn compare_canonical_state_snapshot(
        &self,
        req: &CompareCanonicalStateSnapshotRequest,
    ) -> CanonicalStateDiffResponse {
        use std::collections::BTreeMap;

        let local_entries = self.canonical_state_snapshot_entries(None, None);
        let local_map = local_entries
            .iter()
            .map(|entry| (entry.key.clone(), entry.value.clone()))
            .collect::<BTreeMap<_, _>>();
        let remote_map = req
            .entries
            .iter()
            .map(|entry| (entry.key.clone(), entry.value.clone()))
            .collect::<BTreeMap<_, _>>();

        let mut first_divergence = None;
        for key in local_map.keys().chain(remote_map.keys()) {
            match (local_map.get(key), remote_map.get(key)) {
                (Some(left), Some(right)) if left == right => {}
                (Some(left), Some(right)) => {
                    first_divergence = Some(format!("key={} left={} right={}", key, left, right));
                    break;
                }
                (Some(left), None) => {
                    first_divergence = Some(format!("key={} missing_on_right left={}", key, left));
                    break;
                }
                (None, Some(right)) => {
                    first_divergence = Some(format!("key={} missing_on_left right={}", key, right));
                    break;
                }
                (None, None) => {}
            }
        }

        CanonicalStateDiffResponse {
            height: self.get_stats().height,
            state_root: self.latest_checkpoint_state_root_hex(),
            local_entry_count: local_map.len(),
            remote_entry_count: remote_map.len(),
            first_divergence,
        }
    }

    pub fn canonical_state_snapshot_dump(&self, limit: Option<usize>) -> Vec<(String, String)> {
        let mut entries = self
            .canonical_state_snapshot_entries(limit, None)
            .into_iter()
            .map(|entry| (entry.key, entry.value))
            .collect::<Vec<_>>();
        if let Some(limit) = limit {
            entries.truncate(limit);
        }
        entries
    }

    pub fn first_canonical_state_divergence(&self, other: &Self) -> Option<String> {
        let left = self.state_read().canonical_state_snapshot();
        let right = other.state_read().canonical_state_snapshot();

        for key in left.keys().chain(right.keys()) {
            match (left.get(key), right.get(key)) {
                (Some(left_value), Some(right_value)) if left_value == right_value => {}
                (Some(left_value), Some(right_value)) => {
                    return Some(format!(
                        "key={} left={} right={}",
                        String::from_utf8_lossy(key),
                        hex::encode(left_value),
                        hex::encode(right_value)
                    ));
                }
                (Some(left_value), None) => {
                    return Some(format!(
                        "key={} missing_on_right left={}",
                        String::from_utf8_lossy(key),
                        hex::encode(left_value)
                    ));
                }
                (None, Some(right_value)) => {
                    return Some(format!(
                        "key={} missing_on_left right={}",
                        String::from_utf8_lossy(key),
                        hex::encode(right_value)
                    ));
                }
                (None, None) => {}
            }
        }

        None
    }

    pub fn latest_checkpoint_hash_hex(&self) -> String {
        let chain = self.blockchain.read().unwrap_or_else(|e| e.into_inner());
        chain
            .latest_checkpoint()
            .hash()
            .map(hex::encode)
            .unwrap_or_default()
    }

    pub fn latest_checkpoint_state_root_hex(&self) -> String {
        let chain = self.blockchain.read().unwrap_or_else(|e| e.into_inner());
        hex::encode(&chain.latest_checkpoint().state_root)
    }

    pub fn get_stats(&self) -> BlockchainStats {
        let state = self.state_read();
        let chain = match self.blockchain.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::error!("Blockchain lock poisoned in get_stats, recovering...");
                poisoned.into_inner()
            }
        };
        let pending_transactions = self.pending_transaction_len();

        BlockchainStats {
            height: chain.height(),
            total_blocks: chain.dag_checkpoints.len(),
            total_transactions: chain.get_transaction_count(),
            pending_transactions,
            total_owners: state.owner_count(),
            total_supply: state.total_supply,
            state_root: hex::encode(&chain.latest_checkpoint().state_root),
        }
    }

    pub fn get_owner_info(&self, owner: &str) -> Option<OwnerInfo> {
        let state = self.state_read();

        state.get_owner_state_by_hex(owner).map(|acc| {
            let final_owned_objects = self.resolve_account_objects(&state, &acc.address);
            let nonce = self.get_expected_nonce(owner);
            let balances = state
                .resolve_owner_token_balances(acc.address)
                .unwrap_or_else(|_| {
                    acc.token_balances
                        .iter()
                        .map(|(token_type, balance)| (token_type.clone(), balance.value()))
                        .collect()
                });

            OwnerInfo {
                owner: format!("{:#x}", acc.owner_address()),
                nonce: Some(nonce),
                modules: acc.modules.iter().cloned().collect(),
                balances,
                owned_object_count: Some(final_owned_objects.len()),
                owned_objects: Some(final_owned_objects),
            }
        })
    }

    pub fn get_objects_by_type(&self, object_type: &str) -> Result<Vec<ObjectInfo>> {
        self.query_objects(None, None, Some(object_type), None, None)
    }

    pub fn query_objects(
        &self,
        owner: Option<&str>,
        owner_kind: Option<RpcObjectOwnerKindFilter>,
        object_type: Option<&str>,
        min_version: Option<u64>,
        max_version: Option<u64>,
    ) -> Result<Vec<ObjectInfo>> {
        let state = self.state_read();
        let owner_addr = owner
            .map(KanariAddress::parse_to_account_address)
            .transpose()?;
        let owner_kind_filter = owner_kind.as_ref().map(|kind| match kind {
            RpcObjectOwnerKindFilter::Address => ObjectOwnerKind::AddressOwner("".to_string()),
            RpcObjectOwnerKindFilter::Shared => ObjectOwnerKind::Shared,
            RpcObjectOwnerKindFilter::Immutable => ObjectOwnerKind::Immutable,
        });
        Ok(state
            .query_objects(
                owner_addr,
                owner_kind_filter.as_ref(),
                object_type,
                min_version,
                max_version,
            )?
            .into_iter()
            .map(|(id, obj)| {
                let digest = obj.digest();
                ObjectInfo {
                    id,
                    owner: format!("{:#x}", obj.owner),
                    owner_kind: obj.owner_kind,
                    type_: obj.type_,
                    data: obj.data,
                    version: obj.version,
                    digest: Some(digest),
                }
            })
            .collect())
    }

    pub fn get_object_by_ref(&self, object_ref: &ObjectRef) -> Result<Option<ObjectInfo>> {
        let state = self.state_read();
        let Some(obj) = state.get_object(&object_ref.object_id)? else {
            return Ok(None);
        };
        if let Some(version) = object_ref.version
            && obj.version != version
        {
            return Ok(None);
        }
        if let Some(digest) = &object_ref.digest
            && obj.digest() != *digest
        {
            return Ok(None);
        }
        let digest = obj.digest();
        Ok(Some(ObjectInfo {
            id: object_ref.object_id.clone(),
            owner: format!("{:#x}", obj.owner),
            owner_kind: obj.owner_kind,
            type_: obj.type_,
            data: obj.data,
            version: obj.version,
            digest: Some(digest),
        }))
    }

    pub fn get_module_bytecode(&self, address: &str, module_name: &str) -> Option<Vec<u8>> {
        use move_core_types::{identifier::Identifier, language_storage::ModuleId};

        let addr = match KanariAddress::parse_to_account_address(address) {
            Ok(a) => a,
            Err(_) => return None,
        };

        let ident = match Identifier::new(module_name) {
            Ok(i) => i,
            Err(_) => return None,
        };

        let module_id = ModuleId::new(addr, ident);
        let runtime = &self.runtime_pool[0];
        runtime.get_module_bytes(&module_id)
    }

    pub fn list_all_modules(&self) -> Vec<(String, String)> {
        let runtime = &self.runtime_pool[0];
        runtime
            .list_modules()
            .into_iter()
            .map(|module_id| {
                (
                    format!("0x{}", module_id.address()),
                    module_id.name().to_string(),
                )
            })
            .collect()
    }

    fn checkpoint_hash_hex(checkpoint: &Checkpoint) -> String {
        checkpoint.hash().map(hex::encode).unwrap_or_default()
    }

    fn block_data_from_checkpoint(checkpoint: &Checkpoint) -> BlockData {
        BlockData {
            height: checkpoint.sequence,
            timestamp: checkpoint.timestamp,
            hash: Self::checkpoint_hash_hex(checkpoint),
            prev_hash: hex::encode(&checkpoint.prev_checkpoint_hash),
            state_root: hex::encode(&checkpoint.state_root),
            tx_count: checkpoint.transactions.len(),
            events: Vec::new(),
            transaction_effects: checkpoint.transaction_effects.iter().cloned().collect(),
            object_changes: checkpoint.object_changes.iter().cloned().collect(),
            object_graph_edges: checkpoint.object_graph_edges.iter().cloned().collect(),
        }
    }

    fn full_block_data_from_checkpoint(checkpoint: &Checkpoint) -> FullBlockData {
        FullBlockData {
            height: checkpoint.sequence,
            timestamp: checkpoint.timestamp,
            hash: Self::checkpoint_hash_hex(checkpoint),
            prev_hash: hex::encode(&checkpoint.prev_checkpoint_hash),
            state_root: hex::encode(&checkpoint.state_root),
            tx_count: checkpoint.transactions.len(),
            events: Vec::new(),
            transactions: checkpoint.transactions.iter().cloned().collect(),
            transaction_effects: checkpoint.transaction_effects.iter().cloned().collect(),
            object_changes: checkpoint.object_changes.iter().cloned().collect(),
            object_graph_edges: checkpoint.object_graph_edges.iter().cloned().collect(),
            vertices: checkpoint.vertices.iter().map(hex::encode).collect(),
        }
    }

    pub fn get_block(&self, height: u64) -> Option<BlockData> {
        let chain = match self.blockchain.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::error!("Blockchain lock poisoned in get_block, recovering...");
                poisoned.into_inner()
            }
        };
        chain
            .get_checkpoint(height)
            .map(Self::block_data_from_checkpoint)
    }

    pub fn get_full_block(&self, height: u64) -> Option<FullBlockData> {
        let chain = match self.blockchain.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::error!("Blockchain lock poisoned in get_full_block, recovering...");
                poisoned.into_inner()
            }
        };
        chain
            .get_checkpoint(height)
            .map(Self::full_block_data_from_checkpoint)
    }

    pub fn get_checkpoint_sync(&self, sequence: u64) -> Option<CheckpointSyncData> {
        let chain = match self.blockchain.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::error!("Blockchain lock poisoned in get_checkpoint_sync, recovering...");
                poisoned.into_inner()
            }
        };
        chain
            .get_checkpoint(sequence)
            .cloned()
            .map(|checkpoint| CheckpointSyncData { checkpoint })
    }

    pub fn block_from_full_data(full_block: &FullBlockData) -> kanari_types::block::Block {
        use kanari_types::block::{Block, BlockHeader};
        use smt::compute_merkle_root as compute_transaction_merkle_root;

        let tx_hashes: Vec<Vec<u8>> = full_block.transactions.iter().map(|tx| tx.hash()).collect();
        let merkle_root = compute_transaction_merkle_root(&tx_hashes);

        let header = BlockHeader::new(
            full_block.height,
            hex::decode(&full_block.prev_hash).unwrap_or_default(),
            hex::decode(&full_block.state_root).unwrap_or_default(),
            merkle_root,
            full_block.tx_count,
            full_block.timestamp,
        );

        Block {
            header,
            transactions: full_block.transactions.clone(),
            events: full_block.events.clone(),
        }
    }

    pub fn sync_checkpoint_from_data(&self, checkpoint_data: &CheckpointSyncData) -> Result<()> {
        let stats = self.get_stats();
        let checkpoint = &checkpoint_data.checkpoint;
        info!(
            "[SYNC] Attempting to sync checkpoint #{} (our height: {})",
            checkpoint.sequence, stats.height
        );

        if checkpoint.sequence <= stats.height {
            info!(
                "[SYNC] Already have checkpoint #{}, skipping",
                checkpoint.sequence
            );
            return Ok(());
        }

        if checkpoint.sequence != stats.height + 1 {
            warn!(
                "[SYNC] Checkpoint #{} is not consecutive (need {})",
                checkpoint.sequence,
                stats.height + 1
            );
            anyhow::bail!(
                "Cannot sync checkpoint #{}: current height is {}",
                checkpoint.sequence,
                stats.height
            );
        }

        info!(
            "[SYNC] Verifying {} transaction signatures from checkpoint #{}",
            checkpoint.transactions.len(),
            checkpoint.sequence
        );
        for (i, signed_tx) in checkpoint.transactions.iter().enumerate() {
            signed_tx.verified_transaction_hash().map_err(|e| {
                anyhow::anyhow!(
                    "Invalid or missing signature for transaction {} in checkpoint #{}: {}",
                    i + 1,
                    checkpoint.sequence,
                    e
                )
            })?;
        }

        if checkpoint.transactions.is_empty() {
            anyhow::bail!(
                "Refusing to sync empty checkpoint #{} from network",
                checkpoint.sequence
            );
        }

        let checkpoint_to_apply = checkpoint.clone();
        let (computed_root, verified_state, to_execute) = self
            .prepare_checkpoint_state(&checkpoint_to_apply)
            .map_err(|error| anyhow::anyhow!("{error:#}"))?;

        if !self.checkpoint_root_matches(
            checkpoint_to_apply.sequence,
            &computed_root,
            &checkpoint_to_apply.state_root,
        )? {
            anyhow::bail!(
                "Checkpoint #{} state root mismatch: advertised={}, computed={}",
                checkpoint_to_apply.sequence,
                hex::encode(&checkpoint_to_apply.state_root),
                hex::encode(&computed_root)
            );
        }

        self.apply_prepared_checkpoint(checkpoint_to_apply, verified_state, to_execute, true)?;

        info!(
            "Synced checkpoint #{} with {} transactions",
            checkpoint.sequence,
            checkpoint.transactions.len()
        );

        Ok(())
    }

    pub fn execute_view_function(
        &self,
        package_addr: &str,
        module_name: &str,
        function_name: &str,
        type_args: &[String],
        args: &[Vec<u8>],
        object_inputs: &[kanari_types::transaction::ObjectInput],
    ) -> Result<serde_json::Value> {
        let runtime = &self.runtime_pool[0];
        runtime.execute_view_function(
            package_addr,
            module_name,
            function_name,
            type_args,
            args,
            object_inputs,
        )
    }
}

#[cfg(test)]
#[path = "../../tests/unit/queries_tests.rs"]
mod tests;
