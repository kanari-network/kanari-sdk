// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    first_array_param, internal_error_response, invalid_params_response, parse_labeled_params,
    respond_with_serialize,
};

use super::{RpcError, RpcRequest, RpcResponse, RpcServerState};
use kanari_core::engine::{PendingTransactionMetadata, PendingTransactionRecord};
use kanari_move_runtime_v2::changeset::ChangeSet;
use kanari_rpc_api::{
    BuildCallFunctionRequest, BuildNativeCoinConsolidationRequest, BuildNativeTransferRequest,
    BuildNftTransferRequest, BuildPublishModuleRequest, BuildPublishPackageRequest,
    BuildTokenTransferRequest, CallFunctionRequest, FungibleAssetTransactionsResponse,
    GetFungibleAssetTransactionsRequest, ObjectTransferData, PublishModuleRequest,
    PublishPackageRequest, TransactionDetails, TransactionErrorData, TransactionErrorReason,
    ViewFunctionRequest, methods,
};
use kanari_types::address::Address;
use kanari_types::coin::CoinModule;
use kanari_types::gas_coin::{GAS_COIN, GasModule};
use kanari_types::object::{object_id_for_address, object_id_from_address_bytes};
use kanari_types::transaction::{
    GasPayment, NativeCall, ObjectInput, ObjectOwnerKind, ObjectRef, PublishedModule,
    SignedTransaction, Transaction,
};
use kanari_types::transfer::TransferModule;
use kanari_types::usd_coin::UsdModule;
use kanari_types::{effective_gas_price, gas_price_is_valid};
use move_binary_format::{
    CompiledModule,
    file_format::{SignatureToken, StructHandleIndex},
};
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::TypeTag;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, error, info};

mod build;
mod bytecode;
mod details;
mod error;
/// In-node USD faucet: each web claim mints 100 USD as one coin object.
pub mod faucet;
mod gas_selection;
mod object_inputs;
mod query;
mod submit;
mod transfers;
mod validation;
mod view;

pub use build::{
    handle_build_call_function, handle_build_native_coin_consolidation,
    handle_build_native_transfer, handle_build_nft_transfer, handle_build_publish_module,
    handle_build_publish_package, handle_build_token_transfer,
};
pub(crate) use bytecode::{
    extract_functions_from_bytes, function_object_param_indices, lookup_module_functions,
};
pub(crate) use details::{
    apply_committed_effect, apply_pending_preview_metadata, format_changeset_json,
    map_transaction_to_details, pending_status,
};
pub(crate) use error::transaction_error_with_reason;
pub(crate) use gas_selection::{
    build_call_native_burn_amount, select_coin_object_for_token, select_gas_payment_auto,
    select_native_coin_consolidation_step, select_native_gas_payment,
    select_native_transfer_and_gas_payment,
};
pub(crate) use object_inputs::{
    build_object_input, coin_token_type_from_object_type, infer_object_inputs,
    infer_view_object_inputs,
};
pub use query::{
    handle_count_transactions, handle_get_all_transactions, handle_get_fungible_asset_transactions,
    handle_get_transaction,
};
pub(crate) use submit::build_call_signed_tx;
pub use submit::{
    handle_call_function, handle_publish_module, handle_publish_package,
    handle_submit_object_transfer,
};
pub(crate) use transfers::{
    enrich_transfer_from_effect, lookup_token_decimals, push_transfer_entry,
    token_transfer_details, tx_mentions_token_type,
};
pub(crate) use validation::{
    validate_object_inputs_and_gas, validate_object_inputs_match_state,
    validate_object_ref_completeness,
};
pub use view::handle_view_function;

// The unit tests mounted below reach these through this module's namespace;
// production code uses them directly inside their own submodules.
#[cfg(test)]
pub(crate) use build::fresh_nonce;
#[cfg(test)]
pub(crate) use details::{base_transaction_details, derive_transaction_state_flags};
#[cfg(test)]
pub(crate) use error::classify_transaction_error_data;
#[cfg(test)]
pub(crate) use query::{details_involve_owner, tx_matches_owner};
#[cfg(test)]
pub(crate) use transfers::transfers_from_effect;

// Normalize address strings for comparison (converts to raw hex address)
fn normalize_addr(s: &str) -> String {
    use std::str::FromStr;
    Address::from_str(s)
        .map(|addr| addr.to_hex())
        .unwrap_or_else(|_| s.trim_start_matches("0x").to_lowercase())
}

// The canonical spelling of object ids lives in exactly one place for the
// whole SDK — `kanari_types::object` (full 64-hex lowercase). Build handlers
// apply it before hashing so clients that pad or echo ids reconstruct the
// exact transaction they signed; the submit path must NOT re-spell payloads.
// An id that does not canonicalize never names an object.
pub(crate) use kanari_types::object::canonical_object_id;

pub(super) fn parse_hex_address(
    id: u64,
    raw: &str,
    field: &str,
) -> Result<Address, Box<RpcResponse>> {
    Address::from_hex_literal(raw).map_err(|e| {
        Box::new(invalid_params_response(
            id,
            format!("Invalid {}: {}", field, e),
        ))
    })
}

#[cfg(test)]
#[path = "../../tests/unit/transaction_tests.rs"]
mod tests;
