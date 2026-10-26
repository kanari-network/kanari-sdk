// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Token-transfer interpretation and enrichment of transaction details.

use super::*;

pub(crate) fn token_transfer_details(
    state: &RpcServerState,
    tx: &Transaction,
) -> Option<(String, u64, Option<String>, String)> {
    let Transaction::ExecuteFunction {
        function,
        args,
        module,
        type_args,
        ..
    } = tx
    else {
        return None;
    };

    if function != UsdModule::function_names().transfer_amount
        && function != GasModule::function_names().transfer
        || args.len() < 3
    {
        return None;
    }
    // Native KANARI transfers go through `native_call`; don't double-report here.
    if module == &GasModule::module_path() {
        return None;
    }

    let coin_object_id =
        object_id_for_address(&AccountAddress::from_bytes(args.first()?.as_slice()).ok()?);
    let amount = bcs::from_bytes::<u64>(args.get(1)?).ok()?;
    let recipient =
        object_id_for_address(&AccountAddress::from_bytes(args.get(2)?.as_slice()).ok()?);

    // 1. Prefer explicit type args when present.
    if let Some(first) = type_args.first()
        && first.contains("::")
    {
        return Some((
            recipient,
            amount,
            Some(CoinModule::normalize_token_type(first)),
            coin_object_id,
        ));
    }

    // 2. Live object lookup (works for pending + recent txs).
    if let Ok(Some(object)) = state.engine.state_read().get_object(&coin_object_id)
        && let Some(token_type) = coin_token_type_from_object_type(&object.type_)
    {
        return Some((recipient, amount, Some(token_type), coin_object_id));
    }

    // 3. Return recipient + amount even when the coin object already moved
    // and the live lookup fails. Token type is backfilled from effects later.
    Some((recipient, amount, None, coin_object_id))
}

pub(crate) fn lookup_token_decimals(
    state: Option<&RpcServerState>,
    token_type: Option<&str>,
) -> Option<u8> {
    let token_type = token_type?;
    let normalized = CoinModule::normalize_token_type(token_type);
    if let Some(decimals) = CoinModule::settlement_token_decimals(&normalized) {
        return Some(decimals);
    }
    let state = state?;
    // StateManager already falls back across normalized/raw keys; keep this
    // lookup infallible so a missing index never breaks transaction display.
    let guard = state.engine.state_read();
    guard
        .get_token_decimals(&normalized)
        .ok()
        .flatten()
        .or_else(|| guard.get_token_decimals(token_type).ok().flatten())
}

/// Normalize a hex object id for comparison: arg-parsed ids come from
/// `to_hex_literal` (lowercase `0xâ€¦`) while effect ids are verbatim and may
/// differ in case. Lowercasing can only merge spellings of the same id.
fn norm_hex_id(id: &str) -> String {
    id.trim().to_lowercase()
}

pub(crate) fn transfers_from_effect(
    state: Option<&RpcServerState>,
    effect: &kanari_types::transaction::TransactionEffects,
) -> Vec<kanari_rpc_api::TransferEntry> {
    // Collect every identifiable coin movement. Prefer `transferred`,
    // then `mutated`, then `created`, then the flat list. Dedup by
    // (normalized object_id, normalized owner, token_type) so the same
    // change appearing in two buckets is reported once.
    use std::collections::HashSet;
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let buckets = [
        &effect.transferred,
        &effect.mutated,
        &effect.created,
        &effect.object_changes,
    ];
    for bucket in buckets {
        for change in bucket.iter() {
            let Some(type_) = change.type_.as_deref() else {
                continue;
            };
            let Some(token_type) = coin_token_type_from_object_type(type_) else {
                continue;
            };
            let recipient = match &change.owner {
                Some(ObjectOwnerKind::AddressOwner(addr)) => Some(addr.clone()),
                _ => None,
            };
            let key = (
                norm_hex_id(&change.object_ref.object_id),
                recipient.as_deref().unwrap_or("").to_lowercase(),
                token_type.clone(),
            );
            if !seen.insert(key) {
                continue;
            }
            // Only report coin changes that actually have a new owner.
            // Amount comes from the effect itself (post-move coin balance =
            // what the new owner holds); arg-parsed amounts still win at
            // merge time. Entries without any amount stay unknown.
            if recipient.is_some() {
                let transfer_decimals = lookup_token_decimals(state, Some(&token_type));
                let previous_owner = match &change.previous_owner {
                    Some(ObjectOwnerKind::AddressOwner(addr)) => Some(addr.clone()),
                    _ => None,
                };
                out.push(kanari_rpc_api::TransferEntry {
                    recipient,
                    transfer_amount: change.amount,
                    transfer_token_type: Some(token_type),
                    transfer_decimals,
                    previous_owner,
                    coin_object_id: Some(change.object_ref.object_id.clone()),
                });
            }
        }
    }
    out
}

pub(crate) fn push_transfer_entry(
    details: &mut TransactionDetails,
    entry: kanari_rpc_api::TransferEntry,
) {
    let transfers = details.transfers.get_or_insert_with(Vec::new);
    // Merge by normalized coin_object_id so arg-parsed amount and
    // effect-parsed owner/type combine into one entry instead of two even
    // when hex spellings differ (to_hex_literal vs verbatim).
    if let Some(coin_id) = entry.coin_object_id.clone()
        && let Some(existing) = transfers.iter_mut().find(|e| {
            e.coin_object_id
                .as_deref()
                .is_some_and(|have| norm_hex_id(have) == norm_hex_id(&coin_id))
        })
    {
        if existing.recipient.is_none() {
            existing.recipient = entry.recipient;
        }
        if existing.transfer_amount.is_none() {
            existing.transfer_amount = entry.transfer_amount;
        }
        if existing.transfer_token_type.is_none() {
            existing.transfer_token_type = entry.transfer_token_type;
        }
        if existing.transfer_decimals.is_none() {
            existing.transfer_decimals = entry.transfer_decimals;
        }
        if existing.previous_owner.is_none() {
            existing.previous_owner = entry.previous_owner;
        }
        return;
    }
    transfers.push(entry);
}

/// Normalized address equality for noise filtering.
fn same_addr(a: Option<&str>, b: &str) -> bool {
    a.is_some_and(|x| x.trim().to_lowercase() == b.trim().to_lowercase())
}

pub(crate) fn enrich_transfer_from_effect(
    state: Option<&RpcServerState>,
    details: &mut TransactionDetails,
    effect: &kanari_types::transaction::TransactionEffects,
) {
    let sender = details
        .sender_address
        .as_deref()
        .unwrap_or(details.sender.as_str())
        .to_owned();
    for entry in transfers_from_effect(state, effect) {
        // Drop sender change-back noise: effect-only entries (no amount)
        // where the coin didn't actually change hands. Arg-parsed entries
        // (with amount) always survive, so real self-transfers are kept.
        if entry.transfer_amount.is_none()
            && let Some(recipient) = entry.recipient.as_deref()
            && same_addr(Some(recipient), &sender)
            && (entry.previous_owner.is_none()
                || same_addr(entry.previous_owner.as_deref(), recipient))
        {
            continue;
        }
        push_transfer_entry(details, entry);
    }
    // Backfill decimals for arg-parsed entries whose token type was known
    // but whose metadata lookup was deferred.
    if let Some(transfers) = details.transfers.as_mut() {
        for transfer in transfers.iter_mut() {
            if transfer.transfer_decimals.is_none() {
                transfer.transfer_decimals =
                    lookup_token_decimals(state, transfer.transfer_token_type.as_deref());
            }
        }
        // Backfill token type for arg-parsed entries that never resolved one:
        // entry-function calls (e.g. `token::transfer_amount`) carry the coin
        // in object inputs, not in args, so the source-object lookup can miss
        // on committed txs while the split coin (different id) carries the
        // type in effects. Match on (recipient, amount) and adopt the type
        // only when exactly one donor matches, so same-amount ambiguity can
        // never mislabel funds.
        let donors: Vec<(String, u64, String, Option<u8>)> = transfers
            .iter()
            .filter_map(|t| {
                match (
                    t.recipient.as_deref(),
                    t.transfer_amount,
                    t.transfer_token_type.as_deref(),
                ) {
                    (Some(recipient), Some(amount), Some(token_type)) => Some((
                        recipient.to_lowercase(),
                        amount,
                        token_type.to_string(),
                        t.transfer_decimals,
                    )),
                    _ => None,
                }
            })
            .collect();
        for transfer in transfers.iter_mut() {
            if transfer.transfer_amount.is_none() || transfer.transfer_token_type.is_some() {
                continue;
            }
            let (Some(recipient), Some(amount)) =
                (transfer.recipient.as_deref(), transfer.transfer_amount)
            else {
                continue;
            };
            let recipient_norm = recipient.to_lowercase();
            let mut matches = donors
                .iter()
                .filter(|(r, a, _, _)| *r == recipient_norm && *a == amount);
            if let Some((_, _, token_type, decimals)) = matches.next()
                && matches.next().is_none()
            {
                transfer.transfer_token_type = Some(token_type.clone());
                if transfer.transfer_decimals.is_none() {
                    transfer.transfer_decimals = *decimals;
                }
            }
        }
    }
}

fn token_module_path(token_type: &str) -> Option<String> {
    let token_type = CoinModule::normalize_token_type(token_type);
    let mut parts = token_type.split("::");
    let address = parts.next()?;
    let module = parts.next()?;
    let _struct_name = parts.next()?;
    Some(format!("{address}::{module}"))
}

pub(crate) fn tx_mentions_token_type(tx: &Transaction, token_type: &str) -> bool {
    let token_type = CoinModule::normalize_token_type(token_type);
    let target_module = token_module_path(&token_type);

    match tx {
        Transaction::ExecuteFunction {
            module,
            type_args,
            object_inputs,
            ..
        } => {
            if CoinModule::is_native_token_type(&token_type) && module == &GasModule::module_path()
            {
                return true;
            }

            if let Some(target_module) = &target_module
                && module == target_module
            {
                return true;
            }

            if type_args
                .iter()
                .any(|arg| CoinModule::normalize_token_type(arg) == token_type)
            {
                return true;
            }

            // NOTE: object inputs carry object ids, never token types, so an
            // `object_id.contains(token_type)` check can never match. Asset
            // relevance via object inputs is resolved from execution effects
            // (see transfers_from_effect), not here.
            let _ = object_inputs;
            false
        }
        Transaction::PublishModule { .. }
        | Transaction::PublishPackage { .. }
        | Transaction::UpgradeModule { .. }
        | Transaction::UpgradePackage { .. } => false,
    }
}
