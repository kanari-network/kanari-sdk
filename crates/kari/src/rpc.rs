use futures::FutureExt;
use jsonrpc_core::{IoHandler, Params, Result as JsonRpcResult, Error as JsonRpcError};
use jsonrpc_http_server::{ServerBuilder, AccessControlAllowOrigin, DomainsValidation, Response};
use serde_json::{json, Value as JsonValue};
use crate::blockchain::{BLOCKCHAIN, save_blockchain};
use crate::CHAIN_ID;
use crate::block::Block;
use consensus_pos::Blake3Algorithm;
use crate::transaction::Transaction;

fn create_block(params: Params) -> JsonRpcResult<JsonValue> {
    // 1. Receive and parse the request parameters (if any)
    // let params: MyBlockCreationParams = params.parse().map_err(|e| jsonrpc_core::Error::invalid_params(e))?;

    // 2. Process the request
    let prev_block = unsafe { BLOCKCHAIN.back().unwrap() };
    let new_data = vec![0; 2_250_000]; // Replace with actual block data
    let new_block = Block::new(
        prev_block.index + 1,
        new_data,
        prev_block.hash.clone(),
        25, // Replace with actual token reward
        vec![Transaction { // Replace with actual transactions
            sender: "system".to_string(),
            receiver: "miner_address".to_string(), // Replace with actual miner address
            amount: 0,
            gas_cost: 0.00000150,
        }],
        "miner_address".to_string(), // Replace with actual miner address
        Blake3Algorithm,
    );

    // ... (Verify the block, add it to the blockchain, broadcast it)

    unsafe {
        BLOCKCHAIN.push_back(new_block.clone());
        save_blockchain();
    }

    // 3. Return a success response
    Ok(json!({ "message": "Block created successfully", "block": new_block }))
}

// RPC server
fn get_latest_block(_params: Params) -> JsonRpcResult<JsonValue> {
    unsafe {
        if let Some(block) = BLOCKCHAIN.back() {
            Ok(serde_json::to_value(block).unwrap())
        } else {
            Ok(JsonValue::Null)
        }
    }
}

fn get_chain_id(_params: Params) -> JsonRpcResult<JsonValue> {
    Ok(JsonValue::String(CHAIN_ID.to_string()))
}

fn get_block_by_index(params: Params) -> JsonRpcResult<JsonValue> {
    let index: u32 = params.parse().map_err(|e| jsonrpc_core::Error::invalid_params(format!("Invalid index parameter: {}", e)))?;

    unsafe {
        if let Some(block) = BLOCKCHAIN.iter().find(|b| b.index == index) {
            Ok(serde_json::to_value(block).unwrap())
        } else {
            Ok(JsonValue::Null)
        }
    }
}

pub async fn start_rpc_server() {
    let mut io = IoHandler::new();

    io.add_method("get_latest_block", |params| {
        futures::future::ready(get_latest_block(params)).boxed()
    });

    io.add_method("get_chain_id", |params| {
        futures::future::ready(get_chain_id(params)).boxed()
    });

    io.add_method("get_block_by_index", |params| {
        futures::future::ready(get_block_by_index(params)).boxed()
    });

    io.add_method("create_block", |params| {
        futures::future::ready(create_block(params)).boxed()
    });

    let server = ServerBuilder::new(io)
        .cors(DomainsValidation::AllowOnly(vec![AccessControlAllowOrigin::Any]))
        .start_http(&"127.0.0.1:3030".parse().unwrap())
        .expect("Unable to start RPC server");

    println!("RPC server running on http://127.0.0.1:3030");
    server.wait();
}
