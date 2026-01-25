use super::{RpcError, RpcRequest, RpcResponse, RpcServerState, respond_with_serialize};
use kanari_rpc_api::{
    BatchMerkleProofRequest, BatchMerkleProofResponse, BlockInfo, CompressedMerkleProofRequest,
    CompressedMerkleProofResponse, RpcEvent, TransactionMerkleProof,
};
use serde_json;
use smt::{CompressedMerkleProof, compute_merkle_root};

/// Handle get block request
pub async fn handle_get_block(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let height: u64 = match serde_json::from_value(request.params.clone()) {
        Ok(h) => h,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    match state.engine.get_block(height) {
        Some(block) => {
            // Map runtime events to RPC events
            let rpc_events: Vec<RpcEvent> = block
                .events
                .into_iter()
                .map(|e| RpcEvent {
                    key: e.key,
                    sequence_number: e.sequence_number,
                    type_tag: e.type_tag,
                    event_data: e.event_data,
                })
                .collect();

            let block_info = BlockInfo {
                height: block.height,
                timestamp: block.timestamp,
                hash: block.hash.clone(),
                prev_hash: block.prev_hash,
                tx_count: block.tx_count,
                // `block.hash` is already a hex string; avoid double-encoding
                state_root: block.state_root.clone(),
                events: rpc_events,
            };
            respond_with_serialize(request.id, block_info)
        }
        None => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::internal_error("Block not found")),
            id: request.id,
        },
    }
}

/// Handle get state root request
pub async fn handle_get_state_root(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let req: kanari_rpc_api::GetStateRootRequest =
        match serde_json::from_value(request.params.clone()) {
            Ok(r) => r,
            Err(e) => {
                return RpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(RpcError::invalid_params(e.to_string())),
                    id: request.id,
                };
            }
        };

    let root = state.engine.get_state_root(req.height);
    match root {
        Some(r) => respond_with_serialize(request.id, serde_json::json!(r)),
        None => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::internal_error("State root not available")),
            id: request.id,
        },
    }
}

/// Handle account proof request
pub async fn handle_get_account_proof(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let req: kanari_rpc_api::GetAccountProofRequest =
        match serde_json::from_value(request.params.clone()) {
            Ok(r) => r,
            Err(e) => {
                return RpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(RpcError::invalid_params(e.to_string())),
                    id: request.id,
                };
            }
        };
    // If a height is provided, attempt historical proof using snapshot
    if let Some(h) = req.height {
        match state.engine.get_account_proof_at_height(h, &req.address) {
            Ok(Some((is_member, leaf, siblings))) => {
                let state_root = state.engine.get_state_root(Some(h)).unwrap_or_default();
                let proof = kanari_rpc_api::AccountProof {
                    state_root,
                    is_member,
                    leaf_hash: leaf,
                    siblings,
                };
                return respond_with_serialize(request.id, proof);
            }
            Ok(None) => {
                return RpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(RpcError::internal_error(
                        "Historical proof not available (no snapshot)",
                    )),
                    id: request.id,
                };
            }
            Err(e) => {
                return RpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(RpcError::internal_error(format!(
                        "Proof generation failed: {}",
                        e
                    ))),
                    id: request.id,
                };
            }
        }
    }

    // Latest proof
    match state.engine.get_account_proof(&req.address) {
        Ok(Some((is_member, leaf, siblings))) => {
            let state_root = state.engine.get_state_root(None).unwrap_or_default();
            let proof = kanari_rpc_api::AccountProof {
                state_root,
                is_member,
                leaf_hash: leaf,
                siblings,
            };
            respond_with_serialize(request.id, proof)
        }
        Ok(None) => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::internal_error(
                "Proof not available (SMT not configured or key missing)",
            )),
            id: request.id,
        },
        Err(e) => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::internal_error(format!(
                "Proof generation failed: {}",
                e
            ))),
            id: request.id,
        },
    }
}

/// Handle get block height request
pub async fn handle_get_block_height(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let stats = state.engine.get_stats();
    RpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::json!(stats.height)),
        error: None,
        id: request.id,
    }
}

/// Handle get stats request
pub async fn handle_get_stats(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let stats = state.engine.get_stats();
    let blockchain_stats = kanari_rpc_api::BlockchainStats {
        height: stats.height,
        total_blocks: stats.total_blocks as u64,
        total_transactions: stats.total_transactions as u64,
        pending_transactions: stats.pending_transactions,
        total_accounts: stats.total_accounts,
        total_supply: stats.total_supply,
    };
    respond_with_serialize(request.id, blockchain_stats)
}

/// Handle produce block request (force block production now)
pub async fn handle_produce_block(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    match state.engine.produce_block() {
        Ok(info) => {
            // Serialize the core BlockInfo returned from engine
            respond_with_serialize(request.id, info)
        }
        Err(e) => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::internal_error(format!(
                "Produce block failed: {}",
                e
            ))),
            id: request.id,
        },
    }
}

/// Handle get transaction merkle proof request
pub async fn handle_get_transaction_merkle_proof(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let req: kanari_rpc_api::GetTransactionMerkleProofRequest =
        match serde_json::from_value(request.params.clone()) {
            Ok(r) => r,
            Err(e) => {
                return RpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(RpcError::invalid_params(e.to_string())),
                    id: request.id,
                };
            }
        };

    match state
        .engine
        .get_transaction_merkle_proof(req.block_height, req.tx_index)
    {
        Ok(Some((tx_hash, proof))) => {
            // Get block to get merkle root
            match state.engine.get_block(req.block_height) {
                Some(_block) => {
                    // Note: We need to add merkle_root to BlockData in engine
                    // For now, use a placeholder
                    let merkle_root =
                        if let Some(full_block) = state.engine.get_full_block(req.block_height) {
                            // Compute merkle root from transactions

                            let tx_hashes: Vec<Vec<u8>> =
                                full_block.transactions.iter().map(|tx| tx.hash()).collect();
                            hex::encode(compute_merkle_root(&tx_hashes))
                        } else {
                            String::new()
                        };

                    let proof_response = TransactionMerkleProof {
                        tx_hash,
                        tx_index: req.tx_index,
                        merkle_root,
                        proof: proof.iter().map(hex::encode).collect(),
                    };
                    respond_with_serialize(request.id, proof_response)
                }
                None => RpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(RpcError::internal_error("Block not found")),
                    id: request.id,
                },
            }
        }
        Ok(None) => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::internal_error(
                "Transaction merkle proof not available",
            )),
            id: request.id,
        },
        Err(e) => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::internal_error(format!(
                "Merkle proof generation failed: {}",
                e
            ))),
            id: request.id,
        },
    }
}

/// Handle batch merkle proof request
pub async fn handle_get_batch_merkle_proofs(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let req: BatchMerkleProofRequest = match serde_json::from_value(request.params.clone()) {
        Ok(r) => r,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    // Get merkle root from block
    let merkle_root = if let Some(full_block) = state.engine.get_full_block(req.block_height) {
        let tx_hashes: Vec<Vec<u8>> = full_block.transactions.iter().map(|tx| tx.hash()).collect();
        hex::encode(compute_merkle_root(&tx_hashes))
    } else {
        return RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::internal_error("Block not found")),
            id: request.id,
        };
    };

    let mut proofs = Vec::new();

    for &tx_index in &req.tx_indices {
        match state
            .engine
            .get_transaction_merkle_proof(req.block_height, tx_index)
        {
            Ok(Some((tx_hash, proof))) => {
                proofs.push(TransactionMerkleProof {
                    tx_hash,
                    tx_index,
                    merkle_root: merkle_root.clone(),
                    proof: proof.iter().map(hex::encode).collect(),
                });
            }
            Ok(None) | Err(_) => {
                return RpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(RpcError::internal_error(format!(
                        "Failed to generate proof for transaction at index {}",
                        tx_index
                    ))),
                    id: request.id,
                };
            }
        }
    }

    let response = BatchMerkleProofResponse {
        block_height: req.block_height,
        merkle_root,
        proofs,
    };

    respond_with_serialize(request.id, response)
}

/// Handle compressed merkle proof request
pub async fn handle_get_compressed_merkle_proof(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let req: CompressedMerkleProofRequest = match serde_json::from_value(request.params.clone()) {
        Ok(r) => r,
        Err(e) => {
            return RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(RpcError::invalid_params(e.to_string())),
                id: request.id,
            };
        }
    };

    match state
        .engine
        .get_transaction_merkle_proof(req.block_height, req.tx_index)
    {
        Ok(Some((tx_hash, proof))) => {
            // Get merkle root
            let merkle_root =
                if let Some(full_block) = state.engine.get_full_block(req.block_height) {
                    let tx_hashes: Vec<Vec<u8>> =
                        full_block.transactions.iter().map(|tx| tx.hash()).collect();
                    hex::encode(compute_merkle_root(&tx_hashes))
                } else {
                    return RpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(RpcError::internal_error("Block not found")),
                        id: request.id,
                    };
                };

            let tx_hash_bytes = hex::decode(&tx_hash).unwrap_or_default();
            let compressed =
                CompressedMerkleProof::from_proof(&tx_hash_bytes, req.tx_index, &proof);

            let compressed_bytes = compressed.to_bytes();
            let original_size =
                proof.iter().map(|p| p.len()).sum::<usize>() + std::mem::size_of::<usize>(); // Include index size

            use base64::{Engine as _, engine::general_purpose};
            let response = CompressedMerkleProofResponse {
                tx_hash,
                tx_index: req.tx_index,
                merkle_root,
                compressed_proof: general_purpose::STANDARD.encode(&compressed_bytes),
                original_size,
                compressed_size: compressed_bytes.len(),
            };

            respond_with_serialize(request.id, response)
        }
        Ok(None) => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::internal_error(
                "Transaction merkle proof not available",
            )),
            id: request.id,
        },
        Err(e) => RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError::internal_error(format!(
                "Merkle proof generation failed: {}",
                e
            ))),
            id: request.id,
        },
    }
}
