// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Kanari RPC API Definitions
//!
//! Defines request/response types and RPC methods for Kanari blockchain

pub mod args;
pub mod error;
pub mod fungible;
pub mod network;
pub mod requests;
pub mod responses;
pub mod state;
pub mod transaction;
pub mod types;

pub use args::deserialize_args;
pub use error::*;
pub use fungible::*;
pub use network::*;
pub use requests::*;
pub use responses::*;
pub use state::*;
pub use transaction::*;
pub use types::*;

/// RPC Methods
#[kanari_open_rpc::open_rpc]
pub mod methods {
    use kanari_open_rpc::{
        object_schema, optional_schema, schema_array, schema_boolean, schema_integer,
        schema_object, schema_string,
    };

    // Owner & Balance
    #[open_rpc_method(
        summary = "Get owner info",
        description = "Returns owner data including nonce, balances, modules, and owned objects.",
        params = [("owner", "Target owner address.", true, schema_string())],
        result = ("owner", "Owner information object.", schema_object()),
        tags = ["owner", "balance"]
    )]
    pub const GET_OWNER: &str = "kanari_getOwner";
    #[open_rpc_method(
        summary = "Get one token balance",
        description = "Returns the balance of a specific token type for one owner. Includes on-chain CoinMetadata decimals when indexed, otherwise null. Clients must never assume a fallback decimals value.",
        params = [(
            "request",
            "Owner and token type payload.",
            true,
            object_schema(&[("owner", schema_string()), ("token_type", schema_string())])
        )],
        result = ("balance", "Token balance payload with raw balance and optional decimals.", schema_object()),
        tags = ["owner", "balance"]
    )]
    pub const GET_TOKEN_BALANCE: &str = "kanari_getTokenBalance";
    #[open_rpc_method(
        summary = "Get all balances",
        description = "Returns all visible balances for one owner. Each entry carries on-chain CoinMetadata decimals when indexed, otherwise null.",
        params = [(
            "request",
            "Owner payload.",
            true,
            object_schema(&[("owner", schema_string())])
        )],
        result = ("balances", "All owner balances.", schema_object()),
        tags = ["owner", "balance"]
    )]
    pub const GET_OWNER_BALANCES: &str = "kanari_getOwnerBalances";
    #[open_rpc_method(
        summary = "List tokens",
        description = "Returns visible token metadata and supply summaries.",
        params = [],
        result = ("tokens", "List of tracked token summaries.", schema_array(schema_object())),
        tags = ["balance"]
    )]
    pub const LIST_TOKENS: &str = "kanari_listTokens";
    #[open_rpc_method(
        summary = "Get fungible asset",
        description = "Returns metadata, supply, and holder count for one fungible asset token type. Decimals is present only when on-chain CoinMetadata is indexed.",
        params = [(
            "request",
            "Fungible asset token type payload.",
            true,
            object_schema(&[("token_type", schema_string())])
        )],
        result = ("asset", "Fungible asset summary.", schema_object()),
        tags = ["asset", "balance"]
    )]
    pub const GET_FUNGIBLE_ASSET: &str = "kanari_getFungibleAsset";
    #[open_rpc_method(
        summary = "Get fungible asset holders",
        description = "Returns wallets that currently hold a positive balance of one fungible asset.",
        params = [(
            "request",
            "Fungible asset holder query.",
            true,
            object_schema(&[
                ("token_type", schema_string()),
                ("limit", optional_schema(schema_integer())),
                ("cursor", optional_schema(object_schema(&[
                    ("balance", schema_integer()),
                    ("owner", schema_string()),
                ])))
            ])
        )],
        result = ("holders", "Fungible asset holders with optional next_cursor.", schema_object()),
        tags = ["asset", "balance"]
    )]
    pub const GET_FUNGIBLE_ASSET_HOLDERS: &str = "kanari_getFungibleAssetHolders";
    #[open_rpc_method(
        summary = "Get fungible asset transactions",
        description = "Returns recent committed and pending transactions that involve one fungible asset.",
        params = [(
            "request",
            "Fungible asset transaction query.",
            true,
            object_schema(&[
                ("token_type", schema_string()),
                ("owner", optional_schema(schema_string())),
                ("limit", optional_schema(schema_integer()))
            ])
        )],
        result = ("transactions", "Fungible asset transaction list. Each transfer carries transfer_decimals from on-chain CoinMetadata (null when unknown) and previous_owner when the effect records one; effect-derived entries have no transfer_amount. Clients must never assume fallback decimals.", schema_object()),
        tags = ["asset", "transaction"]
    )]
    pub const GET_FUNGIBLE_ASSET_TRANSACTIONS: &str = "kanari_getFungibleAssetTransactions";

    // Blocks & Transactions
    #[open_rpc_method(
        summary = "Get block",
        description = "Returns a simplified block view by height.",
        params = [("height", "Block height.", true, schema_integer())],
        result = ("block", "Block summary.", schema_object()),
        tags = ["block"]
    )]
    pub const GET_BLOCK: &str = "kanari_getBlock";
    #[open_rpc_method(
        summary = "Get full block",
        description = "Returns full block contents including transactions.",
        params = [("height", "Block height.", true, schema_integer())],
        result = ("block", "Full block payload.", schema_object()),
        tags = ["block"]
    )]
    pub const GET_FULL_BLOCK: &str = "kanari_getFullBlock";
    #[open_rpc_method(
        summary = "Get current block height",
        description = "Returns the latest committed block height.",
        params = [],
        result = ("height", "Current chain height.", schema_integer()),
        tags = ["block"]
    )]
    pub const GET_BLOCK_HEIGHT: &str = "kanari_getBlockHeight";
    #[open_rpc_method(
        summary = "Get transaction",
        description = "Returns a committed or pending transaction by hash.",
        params = [(
            "hash",
            "Transaction hash string, with or without 0x prefix.",
            true,
            schema_string()
        )],
        result = ("transaction", "Transaction details. Transfers carry transfer_decimals from on-chain CoinMetadata (null when unknown), previous_owner when recorded, and transfer_amount only when parsed from args. No fallback decimals are ever invented.", schema_object()),
        tags = ["transaction"]
    )]
    pub const GET_TRANSACTION: &str = "kanari_getTransaction";
    #[open_rpc_method(
        summary = "List transactions",
        description = "Returns committed and pending transactions, newest first, with optional filtering. `limit` caps one page (default 50, max 500); pass the hash of the last received row back as `cursor` to fetch the next older page. Pages continue until history is exhausted, so the full history is walkable at any depth.",
        params = [(
            "request",
            "Optional list options: limit (rows per page), owner filter, and cursor (hash of the last row from the previous page, with or without 0x).",
            false,
            object_schema(&[
                ("limit", optional_schema(schema_integer())),
                ("owner", optional_schema(schema_string())),
                ("cursor", optional_schema(schema_string())),
            ])
        )],
        result = ("transactions", "Transaction detail list, newest first. A page shorter than `limit` means the walk reached the end. Transfers carry transfer_decimals from on-chain CoinMetadata (null when unknown), previous_owner when recorded, and transfer_amount only when parsed from args. No fallback decimals are ever invented.", schema_array(schema_object())),
        tags = ["transaction"]
    )]
    pub const GET_ALL_TRANSACTIONS: &str = "kanari_getAllTransactions";
    #[open_rpc_method(
        summary = "Count transactions",
        description = "Returns how many committed and pending transactions match the same owner filter as kanari_getAllTransactions, walking the full newest-to-oldest history. Use it to size numbered pagination (total pages = ceil(count / page size)).",
        params = [(
            "request",
            "Optional filter options: owner filter only (no limit or cursor).",
            false,
            object_schema(&[("owner", optional_schema(schema_string()))])
        )],
        result = ("count", "Number of matching transactions.", schema_integer()),
        tags = ["transaction"]
    )]
    pub const COUNT_TRANSACTIONS: &str = "kanari_countTransactions";
    #[open_rpc_method(
        summary = "Submit transaction",
        description = "Submits a transfer or burn transaction to the mempool.",
        params = [("transaction", "Signed transaction payload.", true, schema_object())],
        result = ("submission", "Submission status payload.", schema_object()),
        tags = ["transaction"]
    )]
    pub const SUBMIT_OBJECT_TRANSFER: &str = "kanari_submitObjectTransfer";
    #[open_rpc_method(
        summary = "Build native transfer transaction",
        description = "Applies the shared NativeTransferPolicyContract: the backend selects canonical native coin refs, gas payment, and nonce, then returns a canonical unsigned transfer payload. One native coin may cover transfer plus gas; otherwise distinct transfer and gas coins are selected.",
        params = [("transaction", "Unsigned native transfer payload.", true, schema_object())],
        result = ("transaction", "Prepared object-transfer request.", schema_object()),
        tags = ["transaction"]
    )]
    pub const BUILD_NATIVE_TRANSFER: &str = "kanari_buildNativeTransfer";
    #[open_rpc_method(
        summary = "Build native coin consolidation step",
        description = "Builds one backend-approved coin::join_entry step when NativeTransferPolicyContract cannot yet be satisfied by current native coin layout.",
        params = [("transaction", "Unsigned native coin consolidation payload.", true, schema_object())],
        result = ("call", "Prepared call-function request for one native coin join step.", schema_object()),
        tags = ["transaction"]
    )]
    pub const BUILD_NATIVE_COIN_CONSOLIDATION: &str = "kanari_buildNativeCoinConsolidation";

    // Stats & Info
    #[open_rpc_method(
        summary = "Get chain stats",
        description = "Returns aggregate blockchain statistics.",
        params = [],
        result = ("stats", "Blockchain statistics.", schema_object()),
        tags = ["system"]
    )]
    pub const GET_STATS: &str = "kanari_getStats";
    #[open_rpc_method(
        summary = "Get sparse Merkle tree status",
        description = "Returns read-only incremental SMT roots, overlay counters, and schema versions. Set audit=true to perform an expensive full canonical consistency audit; this endpoint never repairs state.",
        params = [(
            "request",
            "Optional SMT diagnostics options.",
            false,
            object_schema(&[("audit", schema_boolean())])
        )],
        result = ("status", "Sparse Merkle tree diagnostics.", schema_object()),
        tags = ["system", "debug"]
    )]
    pub const GET_SMT_STATUS: &str = "kanari_getSmtStatus";
    #[open_rpc_method(
        summary = "Get canonical state snapshot diagnostics",
        description = "Returns canonical state key/value entries for the current node state, with optional prefix and limit filters, for divergence debugging.",
        params = [(
            "request",
            "Optional canonical snapshot diagnostics filter.",
            false,
            object_schema(&[])
        )],
        result = ("snapshot", "Canonical state snapshot diagnostics.", schema_object()),
        tags = ["system", "debug"]
    )]
    pub const GET_CANONICAL_STATE_SNAPSHOT: &str = "kanari_getCanonicalStateSnapshot";
    #[open_rpc_method(
        summary = "Compare canonical state snapshot diagnostics",
        description = "Compares current node canonical state against a caller-provided snapshot and returns the first divergent key/value.",
        params = [(
            "request",
            "Canonical snapshot entries captured from another node.",
            true,
            object_schema(&[("entries", schema_array(schema_object()))])
        )],
        result = ("diff", "Canonical state divergence diagnostics.", schema_object()),
        tags = ["system", "debug"]
    )]
    pub const COMPARE_CANONICAL_STATE_SNAPSHOT: &str = "kanari_compareCanonicalStateSnapshot";
    pub const ESTIMATE_GAS: &str = "kanari_estimateGas";
    #[open_rpc_method(
        summary = "Health check",
        description = "Returns runtime health and guard configuration.",
        params = [],
        result = ("health", "Health status payload.", schema_object()),
        tags = ["system"]
    )]
    pub const HEALTH: &str = "kanari_health";
    #[open_rpc_method(
        summary = "Get active gas model",
        description = "Returns the active gas policy and both requested and effective default gas prices.",
        params = [],
        result = ("gas", "Active gas policy metadata.", schema_object()),
        tags = ["system"]
    )]
    pub const GET_GAS_INFO: &str = "kanari_getGasInfo";
    #[open_rpc_method(
        summary = "Get network authority status",
        description = "Returns the configured validator authority set for this node.",
        params = [],
        result = ("network", "Network authority status payload.", schema_object()),
        tags = ["system"]
    )]
    pub const GET_NETWORK_STATUS: &str = "kanari_getNetworkStatus";

    // Module operations
    #[open_rpc_method(
        summary = "Publish or upgrade module",
        description = "Publishes Move module bytecode. If the module id already exists, this performs a compatibility-checked upgrade.",
        params = [("module", "Module publication payload.", true, schema_object())],
        result = ("publication", "Publish/upgrade execution or pending status.", schema_object()),
        tags = ["module"]
    )]
    pub const PUBLISH_MODULE: &str = "kanari_publishModule";
    #[open_rpc_method(
        summary = "Upgrade module",
        description = "Alias of kanari_publishModule for compatibility-checked module upgrades.",
        params = [("module", "Module upgrade payload.", true, schema_object())],
        result = ("upgrade", "Upgrade execution or pending status.", schema_object()),
        tags = ["module"]
    )]
    pub const UPGRADE_MODULE: &str = "kanari_upgradeModule";
    #[open_rpc_method(
        summary = "Build publish or upgrade module transaction",
        description = "Resolves nonce and gas payment for a module publication or compatibility-checked upgrade and returns a canonical unsigned request.",
        params = [("module", "Unsigned module publication/upgrade payload.", true, schema_object())],
        result = ("module", "Prepared publish/upgrade-module request.", schema_object()),
        tags = ["module"]
    )]
    pub const BUILD_PUBLISH_MODULE: &str = "kanari_buildPublishModule";
    #[open_rpc_method(
        summary = "Build upgrade module transaction",
        description = "Alias of kanari_buildPublishModule for compatibility-checked module upgrades.",
        params = [("module", "Unsigned module upgrade payload.", true, schema_object())],
        result = ("module", "Prepared upgrade-module request.", schema_object()),
        tags = ["module"]
    )]
    pub const BUILD_UPGRADE_MODULE: &str = "kanari_buildUpgradeModule";
    #[open_rpc_method(
        summary = "Publish or upgrade package",
        description = "Publishes multiple Move modules atomically in one package transaction. Existing module ids are compatibility-checked upgrades.",
        params = [("package", "Package publication payload.", true, schema_object())],
        result = ("publication", "Publish/upgrade execution or pending status.", schema_object()),
        tags = ["module"]
    )]
    pub const PUBLISH_PACKAGE: &str = "kanari_publishPackage";
    #[open_rpc_method(
        summary = "Upgrade package",
        description = "Alias of kanari_publishPackage for compatibility-checked atomic package upgrades.",
        params = [("package", "Package upgrade payload.", true, schema_object())],
        result = ("upgrade", "Upgrade execution or pending status.", schema_object()),
        tags = ["module"]
    )]
    pub const UPGRADE_PACKAGE: &str = "kanari_upgradePackage";
    #[open_rpc_method(
        summary = "Build publish or upgrade package transaction",
        description = "Resolves nonce and gas payment for an atomic package publication or compatibility-checked upgrade and returns a canonical unsigned request.",
        params = [("package", "Unsigned package publication/upgrade payload.", true, schema_object())],
        result = ("package", "Prepared publish/upgrade-package request.", schema_object()),
        tags = ["module"]
    )]
    pub const BUILD_PUBLISH_PACKAGE: &str = "kanari_buildPublishPackage";
    #[open_rpc_method(
        summary = "Build upgrade package transaction",
        description = "Alias of kanari_buildPublishPackage for compatibility-checked atomic package upgrades.",
        params = [("package", "Unsigned package upgrade payload.", true, schema_object())],
        result = ("package", "Prepared upgrade-package request.", schema_object()),
        tags = ["module"]
    )]
    pub const BUILD_UPGRADE_PACKAGE: &str = "kanari_buildUpgradePackage";
    #[open_rpc_method(
        summary = "Get module",
        description = "Returns module metadata by address and module name.",
        params = [(
            "request",
            "Address and module name payload.",
            true,
            object_schema(&[("address", schema_string()), ("name", schema_string())])
        )],
        result = ("module", "Module metadata.", schema_object()),
        tags = ["module"]
    )]
    pub const GET_MODULE: &str = "kanari_getModule";
    #[open_rpc_method(
        summary = "List modules",
        description = "Lists all known published modules.",
        params = [],
        result = ("modules", "Module metadata list.", schema_array(schema_object())),
        tags = ["module"]
    )]
    pub const LIST_MODULES: &str = "kanari_listModules";
    #[open_rpc_method(
        summary = "Verify module bytecode",
        description = "Deserializes module bytes and returns basic verification outcome.",
        params = [(
            "request",
            "Module byte payload.",
            true,
            object_schema(&[("module_bytes", schema_array(schema_integer()))])
        )],
        result = ("verification", "Verification result.", schema_object()),
        tags = ["module"]
    )]
    pub const VERIFY_MODULE: &str = "kanari_verifyModule";

    // Function calls
    #[open_rpc_method(
        summary = "Call function",
        description = "Submits or executes a Move entry function transaction.",
        params = [("call", "Function call payload.", true, schema_object())],
        result = ("call_result", "Execution or pending result.", schema_object()),
        tags = ["function"]
    )]
    pub const CALL_FUNCTION: &str = "kanari_callFunction";
    #[open_rpc_method(
        summary = "Build function call transaction",
        description = "Resolves object inputs, nonce, and gas payment for a Move entry function call and returns a canonical unsigned request.",
        params = [("call", "Unsigned function call payload.", true, schema_object())],
        result = ("call", "Prepared call-function request.", schema_object()),
        tags = ["function"]
    )]
    pub const BUILD_CALL_FUNCTION: &str = "kanari_buildCallFunction";
    #[open_rpc_method(
        summary = "Build token transfer transaction",
        description = "Selects a token coin object, resolves gas and nonce, and returns a canonical unsigned Move call for token transfer.",
        params = [("call", "Unsigned token transfer payload.", true, schema_object())],
        result = ("call", "Prepared token-transfer call request.", schema_object()),
        tags = ["function"]
    )]
    pub const BUILD_TOKEN_TRANSFER: &str = "kanari_buildTokenTransfer";
    #[open_rpc_method(
        summary = "Build NFT transfer transaction",
        description = "Resolves the owned NFT object input, gas, and nonce, and returns a canonical unsigned `0x2::transfer::public_transfer<T>` call.",
        params = [("call", "Unsigned NFT transfer payload.", true, schema_object())],
        result = ("call", "Prepared NFT-transfer call request.", schema_object()),
        tags = ["nft"]
    )]
    pub const BUILD_NFT_TRANSFER: &str = "kanari_buildNftTransfer";
    #[open_rpc_method(
        summary = "Build KRC-1155 transfer transaction",
        description = "Resolves the KRC-1155 Balance object inputs, gas, and nonce, and returns a canonical unsigned `kanari_system::krc_1155::transfer` call. Pass `object_id` to spend one specific Balance object, or `token_type` to have the server select up to three owned Balance objects of that type covering `amount`, using `transfer_two` or `transfer_three` when a single object is short.",
        params = [("call", "Unsigned KRC-1155 transfer payload.", true, schema_object())],
        result = ("call", "Prepared KRC-1155-transfer call request.", schema_object()),
        tags = ["krc1155"]
    )]
    pub const BUILD_KRC1155_TRANSFER: &str = "kanari_buildKrc1155Transfer";
    #[open_rpc_method(
        summary = "Claim 100 USD from the web faucet",
        description = "Mints exactly 100 USD as two 50 USD coin objects to an address, with per-address cooldown. The node sponsor pays gas.",
        params = [("request", "Faucet claim payload.", true, schema_object())],
        result = ("faucet_result", "Faucet claim result.", schema_object()),
        tags = ["faucet"]
    )]
    pub const REQUEST_USD_FAUCET: &str = "kanari_requestUsdFaucet";
    #[open_rpc_method(
        summary = "Get USD faucet status",
        description = "Returns the web faucet configuration and whether it is enabled.",
        params = [("request", "Empty payload.", false, schema_object())],
        result = ("faucet_status", "Faucet status payload.", schema_object()),
        tags = ["faucet"]
    )]
    pub const GET_USD_FAUCET_STATUS: &str = "kanari_getUsdFaucetStatus";
    #[open_rpc_method(
        summary = "View function",
        description = "Executes a read-only Move function without submitting a transaction.",
        params = [(
            "request",
            "Array containing one view-function payload.",
            true,
            schema_array(schema_object())
        )],
        result = ("view_result", "Read-only execution result.", schema_object()),
        tags = ["function"]
    )]
    pub const VIEW_FUNCTION: &str = "kanari_viewFunction";

    // Object queries
    #[open_rpc_method(
        summary = "Get object",
        description = "Returns one on-chain object by id.",
        params = [(
            "request",
            "Object id payload.",
            true,
            object_schema(&[("object_id", schema_string())])
        )],
        result = ("object", "Object info payload.", schema_object()),
        tags = ["object"]
    )]
    pub const GET_OBJECT: &str = "kanari_getObject";
    #[open_rpc_method(
        summary = "Get owned objects",
        description = "Returns owned objects for one address, optionally filtered by object type.",
        params = [(
            "request",
            "Owner and optional type filter payload.",
            true,
            object_schema(&[
                ("owner", schema_string()),
                ("object_type", optional_schema(schema_string()))
            ])
        )],
        result = ("objects", "Owned object list.", schema_object()),
        tags = ["object"]
    )]
    pub const GET_OWNED_OBJECTS: &str = "kanari_getOwnedObjects";
    #[open_rpc_method(
        summary = "Get objects by type",
        description = "Returns all known objects that match one exact object type string.",
        params = [(
            "request",
            "Object type payload.",
            true,
            object_schema(&[("object_type", schema_string())])
        )],
        result = ("objects", "Object list.", schema_object()),
        tags = ["object"]
    )]
    pub const GET_OBJECTS_BY_TYPE: &str = "kanari_getObjectsByType";
    #[open_rpc_method(
        summary = "Query objects",
        description = "Returns objects filtered by owner, owner kind, object type, and/or version range.",
        params = [(
            "request",
            "Object query filter payload.",
            true,
            object_schema(&[])
        )],
        result = ("objects", "Filtered object list.", schema_object()),
        tags = ["object"]
    )]
    pub const GET_OBJECTS: &str = "kanari_getObjects";
    #[open_rpc_method(
        summary = "Get object by ref",
        description = "Returns one object only if the full object ref (id, version, digest) matches the current canonical object.",
        params = [(
            "request",
            "Full object ref payload.",
            true,
            object_schema(&[("object_ref", schema_object())])
        )],
        result = ("object", "Exact object ref match payload.", schema_object()),
        tags = ["object"]
    )]
    pub const GET_OBJECT_BY_REF: &str = "kanari_getObjectByRef";

    // NFT queries
    #[open_rpc_method(
        summary = "Get owned NFTs",
        description = "Returns NFT-like objects owned by one owner.",
        params = [("owner", "Target owner address.", true, schema_string())],
        result = ("nfts", "Owned NFT list.", schema_array(schema_object())),
        tags = ["nft"]
    )]
    pub const GET_OWNED_NFTS: &str = "kanari_getOwnedNfts";

    // NFT collections
    #[open_rpc_method(
        summary = "List NFT collections",
        description = "Returns indexed NFT collections.",
        params = [],
        result = ("collections", "Collection list.", schema_array(schema_object())),
        tags = ["nft"]
    )]
    pub const LIST_COLLECTIONS: &str = "kanari_listCollections";
    #[open_rpc_method(
        summary = "Get NFTs by collection",
        description = "Returns NFTs that belong to one collection id.",
        params = [("id", "Collection id.", true, schema_string())],
        result = ("nfts", "Collection NFT list.", schema_array(schema_object())),
        tags = ["nft"]
    )]
    pub const GET_NFTS_BY_COLLECTION: &str = "kanari_getNftsByCollection";

    // KRC-1155 queries
    #[open_rpc_method(
        summary = "Get owned KRC-1155 balances",
        description = "Returns KRC-1155 Balance objects owned by one owner.",
        params = [("owner", "Target owner address.", true, schema_string())],
        result = ("balances", "Owned KRC-1155 balance list.", schema_array(schema_object())),
        tags = ["krc1155"]
    )]
    pub const GET_OWNED_KRC1155_BALANCES: &str = "kanari_getOwnedKrc1155Balances";
    #[open_rpc_method(
        summary = "Get KRC-1155 token type",
        description = "Returns a KRC-1155 TokenType object by id.",
        params = [("id", "Token type id.", true, schema_string())],
        result = ("token_type", "KRC-1155 token type.", schema_object()),
        tags = ["krc1155"]
    )]
    pub const GET_KRC1155_TOKEN_TYPE: &str = "kanari_getKrc1155TokenType";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_transfer_policy_contract_is_backend_canonical() {
        let policy = NativeTransferPolicyContract::kanari_native();

        assert!(policy.api_selects_objects);
        assert!(policy.canonical_object_refs_required);
        assert!(policy.gas_payment_must_be_native_kanari);
        assert!(policy.allows_single_coin_for_transfer_and_gas);
        assert!(policy.allows_distinct_transfer_and_gas_objects);
        assert!(policy.summary().contains("Coin<0x2::kanari::KANARI>"));
    }

    #[test]
    fn structured_transaction_errors_mark_policy_reasons_only() {
        assert!(
            TransactionErrorReason::InsufficientNativeCoinObjects.uses_native_transfer_policy()
        );
        assert!(
            TransactionErrorReason::NativeCoinConsolidationBlocked.uses_native_transfer_policy()
        );
        assert!(
            TransactionErrorReason::InsufficientTransferCoinBalance.uses_native_transfer_policy()
        );
        assert!(TransactionErrorReason::InsufficientGasCoinBalance.uses_native_transfer_policy());
        assert!(
            TransactionErrorReason::NativeTransferPolicyNotSatisfied.uses_native_transfer_policy()
        );

        assert!(!TransactionErrorReason::GasPaymentObjectOverlap.uses_native_transfer_policy());
        assert!(!TransactionErrorReason::GasPaymentDigestMismatch.uses_native_transfer_policy());
    }

    #[test]
    fn rpc_error_round_trips_structured_transaction_reason() {
        let error = RpcError::transaction_error_structured(
            "native transfer policy not satisfied",
            TransactionErrorData::with_native_transfer_policy(
                TransactionErrorReason::NativeTransferPolicyNotSatisfied,
            ),
        );

        assert_eq!(
            error.transaction_error_reason(),
            Some(TransactionErrorReason::NativeTransferPolicyNotSatisfied)
        );
        assert!(
            error
                .transaction_error_details()
                .and_then(|details| details.native_transfer_policy)
                .is_some()
        );
    }
}
