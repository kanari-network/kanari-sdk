// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use kanari_rpc_api::{BlockData, BlockchainStats, FullBlockData, OwnerInfo};
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
            let sequence_number = self.get_expected_sequence(owner);
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
                sequence_number,
                modules: acc.modules.iter().cloned().collect(),
                balances,
                owned_object_count: Some(final_owned_objects.len()),
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
            transaction_effects: checkpoint.transaction_effects.iter().cloned().collect(),
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
        let (computed_root, verified_state, to_execute) =
            self.prepare_checkpoint_state(&checkpoint_to_apply)?;

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
    ) -> Result<serde_json::Value> {
        let runtime = &self.runtime_pool[0];
        runtime.execute_view_function(package_addr, module_name, function_name, type_args, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CheckpointSyncData, consensus::Checkpoint};
    use kanari_crypto::keys::{CurveType, generate_keypair};
    use kanari_move_runtime_v1::changeset::{ChangeSet, CreatedObject};
    use kanari_move_runtime_v1::state::OwnerState;
    use kanari_types::balance::BalanceRecord;
    use kanari_types::coin::TreasuryCap;
    use kanari_types::kanari::KANARI_TOKEN_TYPE;
    use kanari_types::address::Address as KanariAddress;
    use kanari_types::transaction::{SignedTransaction, Transaction};

    fn signed_transfer(sequence_number: u64) -> SignedTransaction {
        let sender = generate_keypair(CurveType::Ed25519).unwrap();
        let recipient = generate_keypair(CurveType::Ed25519).unwrap();
        let tx = Transaction::new_transfer(
            sender.tagged_address(),
            "0xaaaa".to_string(),
            recipient.address,
            1,
            sequence_number,
        );
        let mut signed_tx = SignedTransaction::new(tx);
        signed_tx
            .sign(&sender.private_key, sender.curve_type)
            .unwrap();
        signed_tx
    }

    fn fund_sender_with_coin(
        engine: &BlockchainEngine,
        owner: move_core_types::account_address::AccountAddress,
        coin_object_id: &str,
        balance: u64,
    ) {
        let mut coin_data = vec![0u8; 40];
        coin_data[32..40].copy_from_slice(&balance.to_le_bytes());
        let mut state = engine.state.write().unwrap_or_else(|e| e.into_inner());
        let previous_total = state.total_supply;
        let previous_visible = state
            .global_token_supplies
            .get(KANARI_TOKEN_TYPE)
            .copied()
            .unwrap_or(previous_total);

        let mut create_coin = ChangeSet::new();
        create_coin.created_objects.push((
            coin_object_id.to_string(),
            CreatedObject {
                owner,
                owner_kind: kanari_types::transaction::ObjectOwnerKind::AddressOwner(
                    owner.to_hex_literal(),
                ),
                uid: None,
                id: None,
                type_: format!("0x2::coin::Coin<{}>", KANARI_TOKEN_TYPE),
                data: coin_data,
                version: 1,
            },
        ));
        state
            .apply_changeset_without_supply_validation(&create_coin)
            .unwrap();

        let mut owner_state = state
            .get_owner_state(&owner)
            .unwrap_or_else(|| OwnerState::new(owner));
        owner_state.set_token_balance(
            KANARI_TOKEN_TYPE.to_string(),
            BalanceRecord::new(balance),
        );
        state.save_owner_state(&owner_state).unwrap();

        let updated_total = previous_total.saturating_add(balance);
        let updated_visible = previous_visible.saturating_add(balance);
        state.total_supply = updated_total;
        state.store.save(b"total_supply", &updated_total).unwrap();
        state
            .store
            .save(
                format!("supply:{}", KANARI_TOKEN_TYPE).as_bytes(),
                &TreasuryCap {
                    total_supply: updated_total,
                },
            )
            .unwrap();
        state
            .global_token_supplies
            .insert(KANARI_TOKEN_TYPE.to_string(), updated_visible);
        state
            .store
            .save(b"global_token_supplies", &state.global_token_supplies)
            .unwrap();
    }

    #[test]
    fn sync_checkpoint_from_data_rejects_empty_checkpoint() {
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

        let error = engine.sync_checkpoint_from_data(&sync_data).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Refusing to sync empty checkpoint")
        );
        assert_eq!(engine.get_stats().height, 0);
    }

    #[test]
    fn sync_checkpoint_from_data_rejects_root_mismatch() {
        let engine = BlockchainEngine::new_in_memory().unwrap();
        let prev_hash = {
            let chain = engine.blockchain.read().unwrap_or_else(|e| e.into_inner());
            chain.latest_checkpoint().hash().unwrap()
        };
        let signed_tx = signed_transfer(0);
        let sender =
            KanariAddress::parse_to_account_address(signed_tx.transaction.sender_address())
                .unwrap();
        fund_sender_with_coin(&engine, sender, "0xaaaa", 1_000_000);
        let checkpoint = Checkpoint::new(1, vec![], vec![signed_tx], vec![9u8; 32], 42, prev_hash);
        let sync_data = CheckpointSyncData { checkpoint };

        let error = engine.sync_checkpoint_from_data(&sync_data).unwrap_err();
        assert!(error.to_string().contains("state root mismatch"));
        assert_eq!(engine.get_stats().height, 0);
    }
}
