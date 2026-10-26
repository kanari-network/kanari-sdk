// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Transaction read and list RPC handlers with cursor-paged detail walks.

use super::*;

pub(crate) fn tx_matches_owner(tx: &Transaction, owner_norm: Option<&str>) -> bool {
    let Some(owner) = owner_norm else {
        return true;
    };

    if let Some(native_call) = tx.native_call() {
        match native_call {
            NativeCall::Transfer { recipient, .. } => {
                return normalize_addr(tx.sender()) == owner || normalize_addr(&recipient) == owner;
            }
            NativeCall::Burn { .. } => {}
        }
    }

    // Framework object transfer (`0x2::transfer::transfer<T>(obj, recipient)`)
    // — the path every NFT transfer takes. The recipient lives in args[1] as a
    // raw BCS address and the object itself is an input (not an arg), so
    // without this branch the move never appears in the recipient's history:
    // coin enrichment skips non-coin types and the sender-only fallback below
    // only ever matches the sender.
    if let Transaction::ExecuteFunction {
        module,
        function,
        args,
        ..
    } = tx
        && module == &TransferModule::module_path()
        && function == TransferModule::ENTRY_TRANSFER
        && args.len() >= 2
        && let Ok(recipient) = AccountAddress::from_bytes(&args[1])
        && normalize_addr(&recipient.to_hex_literal()) == owner
    {
        return true;
    }

    normalize_addr(tx.sender()) == owner
}

/// Post-enrichment owner check: matches when the owner sent the transaction
/// or received any transfer in it. This is what makes incoming token
/// transfers visible in the recipient's activity â€” the pre-filter
/// ([`tx_matches_owner`]) cannot see recipients because entry-function calls
/// carry the coin in object inputs, not in args.
pub(crate) fn details_involve_owner(
    details: &TransactionDetails,
    owner_norm: Option<&str>,
) -> bool {
    let Some(owner) = owner_norm else {
        return true;
    };

    if normalize_addr(&details.sender) == owner {
        return true;
    }
    if details
        .sender_address
        .as_deref()
        .is_some_and(|sender| normalize_addr(sender) == owner)
    {
        return true;
    }
    details.transfers.as_ref().is_some_and(|transfers| {
        transfers.iter().any(|entry| {
            entry
                .recipient
                .as_deref()
                .is_some_and(|recipient| normalize_addr(recipient) == owner)
        })
    })
}

fn emit_unique_tx_details(
    emitted: &mut usize,
    seen_hashes: &mut HashSet<String>,
    limit: usize,
    details: TransactionDetails,
    on_row: &mut dyn FnMut(TransactionDetails) -> bool,
) -> bool {
    if *emitted >= limit {
        return false;
    }

    if !seen_hashes.insert(details.hash.to_lowercase()) {
        return true;
    }

    *emitted += 1;
    on_row(details)
}

fn extract_hash_param(params: &serde_json::Value) -> Option<String> {
    serde_json::from_value::<String>(params.clone())
        .ok()
        .or_else(|| {
            params
                .get("hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
}

/// Handle get transaction by hash request
pub async fn handle_get_transaction(state: &RpcServerState, request: &RpcRequest) -> RpcResponse {
    let hash_param = extract_hash_param(&request.params).unwrap_or_default();

    if hash_param.is_empty() {
        return invalid_params_response(request.id, "Invalid or missing 'hash' parameter");
    }

    let normalized = hash_param.trim_start_matches("0x").to_lowercase();
    let tx_hash_bytes = match hex::decode(&normalized) {
        Ok(bytes) => bytes,
        Err(_) => return invalid_params_response(request.id, "Invalid transaction hash hex"),
    };

    let chain = state.engine.blockchain.read().unwrap_or_else(|p| {
        error!("blockchain lock poisoned; recovering");
        p.into_inner()
    });

    if let Some((tx, height, state_root, effect)) =
        chain.get_transaction_location_with_effect(&tx_hash_bytes)
    {
        let mut details = map_transaction_to_details(
            state,
            &tx.transaction,
            &hex::encode(tx.transaction_hash()),
            "committed",
            Some(height),
            Some(hex::encode(state_root)),
        );
        apply_committed_effect(Some(state), &mut details, effect);
        return respond_with_serialize(request.id, details);
    }
    drop(chain);

    if let Some((tx, height, state_root)) = state
        .engine
        .get_committed_transaction_from_history(&tx_hash_bytes)
    {
        let mut details = map_transaction_to_details(
            state,
            &tx.transaction,
            &hex::encode(tx.transaction_hash()),
            "committed",
            Some(height),
            Some(hex::encode(state_root)),
        );
        let effect = state
            .engine
            .get_committed_transaction_effect_from_history(&tx_hash_bytes);
        apply_committed_effect(Some(state), &mut details, effect.as_ref());
        return respond_with_serialize(request.id, details);
    }

    if let Some(tx) = state.engine.find_pending_transaction(&tx_hash_bytes) {
        let tx_hash = hex::encode(tx.signed_tx.transaction_hash());
        let mut details = map_transaction_to_details(
            state,
            &tx.signed_tx.transaction,
            &tx_hash,
            pending_status(&tx),
            None,
            None,
        );
        apply_pending_preview_metadata(Some(state), &tx, &mut details);
        return respond_with_serialize(request.id, details);
    }

    internal_error_response(request.id, "Transaction not found")
}

/// Collects one page of transaction details, newest first.
///
/// `cursor` is the hash of the last row from the previous page: the walk
/// resumes strictly after it, so consecutive pages never repeat a row and
/// rows newer than the cursor (arrived between pages) are simply skipped.
/// A cursor that no longer exists in history yields an empty page instead of
/// replaying from the top.
fn walk_transaction_details<F, G>(
    state: &RpcServerState,
    limit: usize,
    cursor: Option<&[u8]>,
    matches_tx: F,
    matches_details: G,
    lock_context: &str,
    on_row: &mut dyn FnMut(TransactionDetails) -> bool,
) where
    F: Fn(&Transaction) -> bool,
    G: Fn(&Transaction, &TransactionDetails) -> bool,
{
    let mut emitted: usize = 0;
    let mut seen_hashes = HashSet::new();
    if let Some(cursor) = cursor {
        // The cursor row itself must never reappear, even if it moves from
        // the pending pool into a checkpoint between the two page requests.
        seen_hashes.insert(format!("0x{}", hex::encode(cursor)));
    }
    // Until the cursor is seen every candidate (all of them are newer) is
    // skipped; without a cursor the page starts at the newest transaction.
    // The pending pool reports its own cursor state: rows it returns are
    // already after the cursor when it is still pending, and an unset flag
    // means the cursor has committed (the pool is entirely newer than it).
    let (pending, cursor_passed) =
        state
            .engine
            .filter_pending_transaction_records(limit, cursor, |tx| {
                matches_tx(&tx.signed_tx.transaction)
            });
    let mut cursor_passed = cursor.is_none() || cursor_passed;

    for tx in &pending {
        let mut details = map_transaction_to_details(
            state,
            &tx.signed_tx.transaction,
            &hex::encode(tx.signed_tx.transaction_hash()),
            pending_status(tx),
            None,
            None,
        );
        apply_pending_preview_metadata(Some(state), tx, &mut details);
        if !matches_details(&tx.signed_tx.transaction, &details) {
            continue;
        }
        if !emit_unique_tx_details(&mut emitted, &mut seen_hashes, limit, details, on_row) {
            break;
        }
    }

    if emitted < limit {
        let chain = state.engine.blockchain.read().unwrap_or_else(|p| {
            error!("blockchain lock poisoned while {lock_context}; recovering");
            p.into_inner()
        });

        for checkpoint in chain.dag_checkpoints.iter().rev() {
            if emitted >= limit {
                break;
            }

            for (index, tx) in checkpoint.transactions.iter().enumerate().rev() {
                if !cursor_passed {
                    if cursor == Some(tx.transaction_hash()) {
                        cursor_passed = true;
                    }
                    continue;
                }
                if !matches_tx(&tx.transaction) {
                    continue;
                }

                let mut details = map_transaction_to_details(
                    state,
                    &tx.transaction,
                    &hex::encode(tx.transaction_hash()),
                    "committed",
                    Some(checkpoint.sequence),
                    Some(hex::encode(&checkpoint.state_root)),
                );
                apply_committed_effect(
                    Some(state),
                    &mut details,
                    checkpoint.transaction_effects.get(index),
                );
                if !matches_details(&tx.transaction, &details) {
                    continue;
                }
                if !emit_unique_tx_details(&mut emitted, &mut seen_hashes, limit, details, on_row) {
                    break;
                }
            }
        }
    }

    if emitted < limit {
        // The engine walks the same newest-to-oldest order and skips down to
        // the cursor itself, so deep history is paged without replaying rows
        // from earlier pages (and without depending on the retained window).
        for (tx, height, state_root) in
            state
                .engine
                .list_committed_transactions_from_history(limit, cursor, &matches_tx)
        {
            // The engine only returns rows strictly after the cursor, so no
            // further cursor bookkeeping is needed in this final phase.
            let tx_hash = tx.transaction_hash().to_vec();
            let effect = state
                .engine
                .get_committed_transaction_effect_from_history(&tx_hash);
            let mut details = map_transaction_to_details(
                state,
                &tx.transaction,
                &hex::encode(&tx_hash),
                "committed",
                Some(height),
                Some(hex::encode(state_root)),
            );
            apply_committed_effect(Some(state), &mut details, effect.as_ref());
            if !matches_details(&tx.transaction, &details) {
                continue;
            }
            if !emit_unique_tx_details(&mut emitted, &mut seen_hashes, limit, details, on_row) {
                break;
            }
        }
    }
}

fn collect_transaction_details<F, G>(
    state: &RpcServerState,
    limit: usize,
    cursor: Option<&[u8]>,
    matches_tx: F,
    matches_details: G,
    lock_context: &str,
) -> Vec<TransactionDetails>
where
    F: Fn(&Transaction) -> bool,
    G: Fn(&Transaction, &TransactionDetails) -> bool,
{
    let mut results: Vec<TransactionDetails> = Vec::new();
    walk_transaction_details(
        state,
        limit,
        cursor,
        matches_tx,
        matches_details,
        lock_context,
        &mut |details| {
            results.push(details);
            true
        },
    );
    results
}

/// Walks the full newest-to-oldest stream (pending first, then committed
/// history) and counts every distinct row the same filters would return
/// across all pages, without materialising the rows themselves.
fn count_transaction_details<F, G>(
    state: &RpcServerState,
    matches_tx: F,
    matches_details: G,
    lock_context: &str,
) -> u64
where
    F: Fn(&Transaction) -> bool,
    G: Fn(&Transaction, &TransactionDetails) -> bool,
{
    let mut count: u64 = 0;
    walk_transaction_details(
        state,
        usize::MAX,
        None,
        matches_tx,
        matches_details,
        lock_context,
        &mut |_| {
            count += 1;
            true
        },
    );
    count
}

/// Owner filter shared by the list and count handlers: normalized without
/// the 0x prefix, lowercased.
fn parse_owner_norm(params: &serde_json::Value) -> Option<String> {
    params
        .as_str()
        .or_else(|| {
            params
                .as_object()
                .and_then(|obj| obj.get("owner").and_then(|v| v.as_str()))
        })
        .map(|a| a.trim_start_matches("0x").to_lowercase())
}

/// Handle request to count all transactions matching the list filters
pub async fn handle_count_transactions(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let owner_norm = parse_owner_norm(&request.params);

    // Same pre-filter / post-filter pair as kanari_getAllTransactions, so
    // the count equals the number of rows a cursor walk would eventually
    // produce for identical filters.
    let count = count_transaction_details(
        state,
        |tx| {
            tx_matches_owner(tx, owner_norm.as_deref())
                || (owner_norm.is_some() && matches!(tx, Transaction::ExecuteFunction { .. }))
        },
        |tx, details| {
            tx_matches_owner(tx, owner_norm.as_deref())
                || details_involve_owner(details, owner_norm.as_deref())
        },
        "counting transactions",
    );
    respond_with_serialize(request.id, count)
}

/// Handle request to list all transactions (committed + pending)
pub async fn handle_get_all_transactions(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let limit = request
        .params
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(50)
        .min(500) as usize;

    let owner_norm = parse_owner_norm(&request.params);

    // Optional resume point: the hash of the last row the client already has
    // (with or without 0x). The page then starts strictly after it, so the
    // caller can walk the whole history `limit` rows at a time.
    let cursor = match request
        .params
        .as_object()
        .and_then(|params| params.get("cursor"))
    {
        None => None,
        Some(value) => {
            let Some(raw) = value.as_str() else {
                return invalid_params_response(
                    request.id,
                    "Invalid cursor: expected a transaction hash string",
                );
            };
            let stripped = raw
                .strip_prefix("0x")
                .or_else(|| raw.strip_prefix("0X"))
                .unwrap_or(raw);
            match hex::decode(stripped) {
                Ok(bytes) if !bytes.is_empty() => Some(bytes),
                _ => {
                    return invalid_params_response(
                        request.id,
                        "Invalid cursor: expected a hex transaction hash",
                    );
                }
            }
        }
    };

    // Pre-filter is deliberately wider than the old sender-only check when an
    // owner is set: entry-function token transfers carry the coin in object
    // inputs, so recipients are only known after enrichment. The post-filter
    // below keeps results exact (sender or transfer recipient).
    let results = collect_transaction_details(
        state,
        limit,
        cursor.as_deref(),
        |tx| {
            tx_matches_owner(tx, owner_norm.as_deref())
                || (owner_norm.is_some() && matches!(tx, Transaction::ExecuteFunction { .. }))
        },
        |tx, details| {
            tx_matches_owner(tx, owner_norm.as_deref())
                || details_involve_owner(details, owner_norm.as_deref())
        },
        "listing transactions",
    );
    respond_with_serialize(request.id, results)
}

pub async fn handle_get_fungible_asset_transactions(
    state: &RpcServerState,
    request: &RpcRequest,
) -> RpcResponse {
    let req_data: GetFungibleAssetTransactionsRequest = match parse_labeled_params(
        request.id,
        &request.params,
        "fungible asset transaction query",
    ) {
        Ok(data) => data,
        Err(response) => return *response,
    };

    let token_type = CoinModule::normalize_token_type(&req_data.token_type);
    let limit = req_data.limit.unwrap_or(50).min(500);
    let owner_norm = req_data
        .owner
        .as_deref()
        .map(|owner| owner.trim_start_matches("0x").to_lowercase());

    // Incoming token transfers must be visible in the recipient's view even
    // when the call goes through a router module that never mentions the
    // token at tx level: the post-filter matches (token, recipient) pairs
    // from enriched transfers. Ambiguous typeless entries never match.
    let results = collect_transaction_details(
        state,
        limit,
        None,
        |tx| {
            if owner_norm.is_none() {
                return tx_mentions_token_type(tx, &token_type);
            }
            (tx_mentions_token_type(tx, &token_type) && tx_matches_owner(tx, owner_norm.as_deref()))
                || matches!(tx, Transaction::ExecuteFunction { .. })
        },
        |tx, details| {
            if owner_norm.is_none() {
                return tx_mentions_token_type(tx, &token_type);
            }
            let owner = owner_norm.as_deref();
            (tx_mentions_token_type(tx, &token_type) && tx_matches_owner(tx, owner))
                || details.transfers.as_ref().is_some_and(|transfers| {
                    transfers.iter().any(|entry| {
                        entry
                            .transfer_token_type
                            .as_deref()
                            .is_some_and(|t| CoinModule::normalize_token_type(t) == token_type)
                            && entry.recipient.as_deref().is_some_and(|recipient| {
                                normalize_addr(recipient) == owner.unwrap_or_default()
                            })
                    })
                })
        },
        "listing asset transactions",
    );

    respond_with_serialize(
        request.id,
        FungibleAssetTransactionsResponse {
            token_type,
            transactions: results,
        },
    )
}
