// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use kanari_rpc_api::{AccountInfo, BlockData, BlockchainStats, FullBlockData};
use kanari_types::address::Address as KanariAddress;
use log::{info, warn};

use super::*;
use crate::{BlockchainEngine, Checkpoint, CheckpointSyncData};

impl BlockchainEngine {
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
        let pending = match self.pending_txs.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::error!("Pending txs lock poisoned in get_stats, recovering...");
                poisoned.into_inner()
            }
        };

        BlockchainStats {
            height: chain.height(),
            total_blocks: chain.dag_checkpoints.len(),
            total_transactions: chain.get_transaction_count(),
            pending_transactions: pending.len(),
            total_accounts: state.account_count(),
            total_supply: state.total_supply,
        }
    }

    pub fn get_account_info(&self, address: &str) -> Option<AccountInfo> {
        let state = self.state_read();

        state.get_account_by_hex(address).map(|acc| {
            let final_owned_objects = self.resolve_account_objects(&state, &acc.address);
            let sequence_number = self.get_expected_sequence(address);
            let mut actual_token_balances = std::collections::BTreeMap::new();

            for obj in &final_owned_objects {
                if !obj.type_.contains("::coin::Coin<") || obj.data.len() < 40 {
                    continue;
                }

                let Some(start) = obj.type_.find('<') else {
                    continue;
                };
                let Some(end) = obj.type_.rfind('>') else {
                    continue;
                };

                let token_type = obj.type_[start + 1..end].to_string();
                let mut amount_bytes = [0u8; 8];
                amount_bytes.copy_from_slice(&obj.data[32..40]);
                let amount = u64::from_le_bytes(amount_bytes);

                let entry = actual_token_balances.entry(token_type).or_insert(0u64);
                *entry = entry.saturating_add(amount);
            }

            for (token_type, balance) in &acc.token_balances {
                actual_token_balances
                    .entry(token_type.clone())
                    .or_insert_with(|| balance.value());
            }

            AccountInfo {
                address: format!("{:#x}", acc.address),
                sequence_number,
                modules: acc.modules.iter().cloned().collect(),
                token_balances: actual_token_balances,
                owned_objects: Some(final_owned_objects),
            }
        })
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
            transactions: checkpoint.transactions.clone(),
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
        use smt::compute_merkle_root;

        let tx_hashes: Vec<Vec<u8>> = full_block.transactions.iter().map(|tx| tx.hash()).collect();
        let merkle_root = compute_merkle_root(&tx_hashes);

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

    fn decode_hex(s: &str) -> Result<Vec<u8>> {
        hex::decode(s.trim_start_matches("0x")).context("Invalid hex string")
    }

    fn decode_hex_32(s: &str) -> [u8; 32] {
        let bytes = Self::decode_hex(s).unwrap_or_default();
        let mut arr = [0u8; 32];
        if bytes.len() == 32 {
            arr.copy_from_slice(&bytes);
        }
        arr
    }

    fn checkpoint_from_full_block_data(
        &self,
        block_data: &FullBlockData,
        prev_hash: Vec<u8>,
    ) -> Result<Checkpoint> {
        let state_root = Self::decode_hex(&block_data.state_root)
            .context("Invalid state root format in block data")?;
        let vertices = block_data
            .vertices
            .iter()
            .map(|vertex| Self::decode_hex_32(vertex))
            .collect();

        Ok(Checkpoint::new(
            block_data.height,
            vertices,
            block_data.transactions.clone(),
            state_root,
            block_data.timestamp,
            prev_hash,
        ))
    }

    pub fn sync_full_block_from_data(&self, block_data: &FullBlockData) -> Result<()> {
        let stats = self.get_stats();
        info!(
            "[SYNC] Attempting to sync block #{} (our height: {})",
            block_data.height, stats.height
        );

        if block_data.height <= stats.height {
            info!("[SYNC] Already have block #{}, skipping", block_data.height);
            return Ok(());
        }

        if block_data.height != stats.height + 1 {
            warn!(
                "[SYNC] Block #{} is not consecutive (need {})",
                block_data.height,
                stats.height + 1
            );
            anyhow::bail!(
                "Cannot sync block #{}: current height is {}",
                block_data.height,
                stats.height
            );
        }

        info!(
            "[SYNC] Verifying {} transaction signatures from block #{}",
            block_data.transactions.len(),
            block_data.height
        );
        for (i, signed_tx) in block_data.transactions.iter().enumerate() {
            let tx_hash = signed_tx.transaction_hash().to_vec();
            if !signed_tx.verify_signature_for_hash(&tx_hash)? {
                anyhow::bail!(
                    "Invalid or missing signature for transaction {} in block #{}",
                    i + 1,
                    block_data.height
                );
            }
        }

        let prev_hash = {
            let chain = self.blockchain.read().unwrap_or_else(|e| e.into_inner());
            chain.latest_checkpoint().hash()?
        };

        let checkpoint = self.checkpoint_from_full_block_data(block_data, prev_hash)?;
        self.apply_checkpoint(checkpoint)?;

        info!(
            "Synced block #{} with {} transactions",
            block_data.height,
            block_data.transactions.len()
        );

        Ok(())
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
            let tx_hash = signed_tx.transaction_hash().to_vec();
            if !signed_tx.verify_signature_for_hash(&tx_hash)? {
                anyhow::bail!(
                    "Invalid or missing signature for transaction {} in checkpoint #{}",
                    i + 1,
                    checkpoint.sequence
                );
            }
        }

        let mut checkpoint_to_apply = checkpoint.clone();
        let (computed_root, verified_state, to_execute) =
            self.prepare_checkpoint_state(&checkpoint_to_apply)?;

        if checkpoint_to_apply.transactions.is_empty() {
            let previous_checkpoint_root = {
                let chain = self.blockchain.read().unwrap_or_else(|e| e.into_inner());
                chain.latest_checkpoint().state_root.clone()
            };

            if checkpoint_to_apply.state_root != previous_checkpoint_root {
                info!(
                    "[SYNC] Canonicalizing empty checkpoint #{} root: advertised={}, canonical={}",
                    checkpoint_to_apply.sequence,
                    hex::encode(&checkpoint_to_apply.state_root),
                    hex::encode(&previous_checkpoint_root)
                );
            }

            checkpoint_to_apply.state_root = previous_checkpoint_root;
        } else if !self.checkpoint_root_matches(
            checkpoint_to_apply.sequence,
            &computed_root,
            &checkpoint_to_apply.state_root,
        )? {
            info!(
                "[SYNC] Canonicalizing checkpoint #{} root after replay: advertised={}, computed={}",
                checkpoint_to_apply.sequence,
                hex::encode(&checkpoint_to_apply.state_root),
                hex::encode(&computed_root)
            );
            checkpoint_to_apply.state_root = computed_root;
        }

        self.apply_prepared_checkpoint(checkpoint_to_apply, verified_state, to_execute)?;

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
    ) -> Result<serde_json::Value> {
        let runtime = &self.runtime_pool[0];
        runtime.execute_view_function(package_addr, module_name, function_name, type_args, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CheckpointSyncData, consensus::Checkpoint};

    #[test]
    fn sync_checkpoint_from_data_applies_next_checkpoint() {
        let engine = BlockchainEngine::new_in_memory().unwrap();
        let prev_hash = {
            let chain = engine.blockchain.read().unwrap_or_else(|e| e.into_inner());
            chain.latest_checkpoint().hash().unwrap()
        };
        let state_root = engine
            .state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .compute_state_root();
        let checkpoint = Checkpoint::new(1, vec![], vec![], state_root, 42, prev_hash);
        let sync_data = CheckpointSyncData { checkpoint };

        engine.sync_checkpoint_from_data(&sync_data).unwrap();
        assert_eq!(engine.get_stats().height, 1);
    }

    #[test]
    fn sync_checkpoint_from_data_canonicalizes_empty_checkpoint_root() {
        let engine = BlockchainEngine::new_in_memory().unwrap();
        let (prev_hash, previous_root) = {
            let chain = engine.blockchain.read().unwrap_or_else(|e| e.into_inner());
            (
                chain.latest_checkpoint().hash().unwrap(),
                chain.latest_checkpoint().state_root.clone(),
            )
        };
        let checkpoint = Checkpoint::new(1, vec![], vec![], vec![9u8; 32], 42, prev_hash);
        let sync_data = CheckpointSyncData { checkpoint };

        engine.sync_checkpoint_from_data(&sync_data).unwrap();

        let chain = engine.blockchain.read().unwrap_or_else(|e| e.into_inner());
        assert_eq!(chain.latest_checkpoint().sequence, 1);
        assert_eq!(chain.latest_checkpoint().state_root, previous_root);
    }
}
