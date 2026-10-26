// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Deterministic gas-payment and coin-object selection.

use super::*;

pub(crate) fn select_native_gas_payment(
    owned_objects: &[kanari_rpc_api::ObjectInfo],
    sender: &str,
    required_amount: u64,
    gas_limit: u64,
    gas_price: u64,
    exclude_object_ids: &[String],
    pending_access_keys: &HashSet<String>,
) -> anyhow::Result<GasPayment> {
    let excluded = exclude_object_ids
        .iter()
        .map(|id| normalize_addr(id))
        .collect::<HashSet<_>>();
    let mut best: Option<(ObjectRef, u64)> = None;
    for object in owned_objects {
        if excluded.contains(&normalize_addr(&object.id)) {
            continue;
        }
        if pending_access_keys.contains(&format!("mut:gas:{}", object.id)) {
            continue;
        }
        if object.type_ != CoinModule::coin_type(GAS_COIN) {
            continue;
        }
        let Some(balance) = CoinModule::read_balance(&object.data) else {
            continue;
        };
        if balance < required_amount {
            continue;
        }
        match &best {
            Some((_, current)) if *current <= balance => {}
            _ => {
                best = Some((
                    ObjectRef::new(
                        object.id.clone(),
                        Some(object.version),
                        object.digest.clone(),
                    ),
                    balance,
                ))
            }
        }
    }

    let (payment_object, _) = best.ok_or_else(|| {
        anyhow::anyhow!(
            "No spendable native gas coin object found with required balance {}",
            required_amount
        )
    })?;

    Ok(GasPayment {
        payment_objects: vec![payment_object],
        owner: sender.to_string(),
        budget: gas_limit,
        price: gas_price,
        coin_type: None,
        price_version: None,
    })
}

/// Node-local gas price table for multi-coin quotation (see
/// `kanari_types::gas_market::GasPriceTable::from_env_or_default`).
pub(crate) fn node_gas_price_table() -> kanari_types::gas_market::GasPriceTable {
    kanari_types::gas_market::GasPriceTable::from_env_or_default()
}

/// Auto-select gas payment: USD first, then native KANARI, then the best
/// remaining whitelisted coin the wallet can afford.
///
/// Returns the payment plus the effective per-unit price in settlement-token
/// base units, which callers must echo into the built transaction's
/// `gas_price` so engine accounting (`units * price`) matches the quote.
#[allow(clippy::too_many_arguments)]
pub(crate) fn select_gas_payment_auto(
    owned_objects: &[kanari_rpc_api::ObjectInfo],
    sender: &str,
    gas_limit: u64,
    gas_price: u64,
    exclude_object_ids: &[String],
    pending_access_keys: &HashSet<String>,
    transfer_needs: &std::collections::BTreeMap<String, u64>,
) -> anyhow::Result<(GasPayment, u64)> {
    let table = node_gas_price_table();
    select_gas_payment_with_table(
        owned_objects,
        sender,
        gas_limit,
        gas_price,
        exclude_object_ids,
        pending_access_keys,
        transfer_needs,
        &table,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn select_gas_payment_with_table(
    owned_objects: &[kanari_rpc_api::ObjectInfo],
    sender: &str,
    gas_limit: u64,
    gas_price: u64,
    exclude_object_ids: &[String],
    pending_access_keys: &HashSet<String>,
    transfer_needs: &std::collections::BTreeMap<String, u64>,
    table: &kanari_types::gas_market::GasPriceTable,
) -> anyhow::Result<(GasPayment, u64)> {
    let native_required = gas_limit.saturating_mul(effective_gas_price(gas_price));
    let excluded: HashSet<String> = exclude_object_ids
        .iter()
        .map(|id| normalize_addr(id))
        .collect();
    let mut balances = std::collections::BTreeMap::<String, u64>::new();
    for object in owned_objects {
        if excluded.contains(&normalize_addr(&object.id)) {
            continue;
        }
        if pending_access_keys.contains(&format!("mut:gas:{}", object.id)) {
            continue;
        }
        let Some(token) = CoinModule::token_type_of_coin_object(&object.type_) else {
            continue;
        };
        let Some(balance) = CoinModule::read_balance(&object.data) else {
            continue;
        };
        let normalized = CoinModule::normalize_token_type(&token);
        let entry = balances.entry(normalized).or_insert(0);
        *entry = entry.saturating_add(balance);
    }
    let (coin, quote) =
        kanari_types::gas_market::select_gas_coin(&balances, gas_limit, table, transfer_needs)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No spendable gas coin object found (tried KANARI and {} whitelisted coin(s))",
                    table.entries.len().saturating_sub(1)
                )
            })?;

    if CoinModule::is_native_token_type(&coin) {
        // Native was affordable only with a different object than the first
        // attempt allowed (e.g. exclusions); reselect honoring them.
        let payment = select_native_gas_payment(
            owned_objects,
            sender,
            native_required,
            gas_limit,
            gas_price,
            exclude_object_ids,
            pending_access_keys,
        )?;
        return Ok((payment, gas_price));
    }

    let mut candidates: Vec<(ObjectRef, u64)> = owned_objects
        .iter()
        .filter(|object| !excluded.contains(&normalize_addr(&object.id)))
        .filter(|object| !pending_access_keys.contains(&format!("mut:gas:{}", object.id)))
        .filter(|object| {
            CoinModule::token_type_of_coin_object(&object.type_)
                .is_some_and(|token| CoinModule::normalize_token_type(&token) == coin)
        })
        .filter_map(|object| {
            CoinModule::read_balance(&object.data).map(|balance| {
                (
                    ObjectRef::new(
                        object.id.clone(),
                        Some(object.version),
                        object.digest.clone(),
                    ),
                    balance,
                )
            })
        })
        .filter(|(_, balance)| *balance >= quote)
        .collect();
    candidates.sort_by_key(|(_, balance)| *balance);
    let (payment_ref, _) = candidates.into_iter().next().ok_or_else(|| {
        anyhow::anyhow!("Selected gas coin {coin} has no single object covering {quote} base units")
    })?;
    // Per-unit settlement price, rounded up so `units * price >= quote`.
    let unit_price = quote.div_ceil(gas_limit.max(1)).max(1);
    Ok((
        GasPayment {
            payment_objects: vec![payment_ref],
            owner: sender.to_string(),
            budget: gas_limit,
            price: unit_price,
            coin_type: Some(coin),
            price_version: Some(table.version),
        },
        unit_price,
    ))
}

pub(crate) fn build_call_native_burn_amount(build_data: &BuildCallFunctionRequest) -> Option<u64> {
    if normalize_addr(&build_data.package) != normalize_addr(Address::KANARI_SYSTEM_ADDRESS) {
        return None;
    }
    if build_data.module != GasModule::GAS_MODULE
        || build_data.function != GasModule::function_names().burn
    {
        return None;
    }
    if build_data.args.len() != 1 {
        return None;
    }

    bcs::from_bytes::<u64>(&build_data.args[0]).ok()
}

pub(crate) fn select_native_transfer_and_gas_payment(
    owned_objects: &[kanari_rpc_api::ObjectInfo],
    sender: &str,
    transfer_amount: u64,
    gas_limit: u64,
    gas_price: u64,
    pending_access_keys: &HashSet<String>,
    excluded_object_ids: &HashSet<String>,
) -> anyhow::Result<(ObjectRef, GasPayment)> {
    let native_coin_type = CoinModule::coin_type(GAS_COIN);
    let required_gas = gas_limit.saturating_mul(effective_gas_price(gas_price));
    let mut native_coins: Vec<(ObjectRef, u64)> = owned_objects
        .iter()
        .filter(|object| object.type_ == native_coin_type)
        .filter(|object| !excluded_object_ids.contains(&object.id))
        .filter(|object| !pending_access_keys.contains(&format!("mut:object:{}", object.id)))
        .filter(|object| !pending_access_keys.contains(&format!("mut:gas:{}", object.id)))
        .filter_map(|object| {
            CoinModule::read_balance(&object.data).map(|balance| {
                (
                    ObjectRef::new(
                        object.id.clone(),
                        Some(object.version),
                        object.digest.clone(),
                    ),
                    balance,
                )
            })
        })
        .collect();

    native_coins.sort_by_key(|(_, balance)| *balance);

    // Match Sui's native transfer behavior: prefer the gas coin as the
    // transfer coin when it can cover both the amount and maximum gas budget.
    if let Some((coin_ref, _)) = native_coins
        .iter()
        .filter(|(_, balance)| {
            *balance >= transfer_amount && balance.saturating_sub(transfer_amount) >= required_gas
        })
        .min_by_key(|(_, balance)| *balance)
    {
        return Ok((
            coin_ref.clone(),
            GasPayment {
                payment_objects: vec![coin_ref.clone()],
                owner: sender.to_string(),
                budget: gas_limit,
                price: gas_price,
                coin_type: None,
                price_version: None,
            },
        ));
    }

    let mut best_pair: Option<((ObjectRef, u64), (ObjectRef, u64))> = None;
    for (transfer_ref, transfer_balance) in native_coins
        .iter()
        .filter(|(_, balance)| *balance >= transfer_amount)
    {
        let Some((gas_ref, gas_balance)) = native_coins
            .iter()
            .filter(|(gas_ref, gas_balance)| {
                gas_ref.object_id != transfer_ref.object_id && *gas_balance >= required_gas
            })
            .min_by_key(|(_, gas_balance)| *gas_balance)
            .cloned()
        else {
            continue;
        };

        let candidate = (
            (transfer_ref.clone(), *transfer_balance),
            (gas_ref, gas_balance),
        );
        // Keep the smallest sufficient coin as the long-lived gas reserve.
        // Preferring the smallest transfer coin can consume it completely and
        // strand the wallet with one funded coin plus one zero-balance coin,
        // making every subsequent native transfer violate the distinct-gas
        // policy.
        match &best_pair {
            Some(((best_transfer_ref, best_transfer_balance), (_, best_gas_balance)))
                if (
                    *best_gas_balance,
                    *best_transfer_balance,
                    &best_transfer_ref.object_id,
                ) <= (candidate.1.1, candidate.0.1, &candidate.0.0.object_id) => {}
            _ => best_pair = Some(candidate),
        }
    }

    if let Some(((coin_object_ref, _), (gas_object_ref, _))) = best_pair {
        return Ok((
            coin_object_ref,
            GasPayment {
                payment_objects: vec![gas_object_ref],
                owner: sender.to_string(),
                budget: gas_limit,
                price: gas_price,
                coin_type: None,
                price_version: None,
            },
        ));
    }

    let has_transfer_coin = native_coins
        .iter()
        .any(|(_, balance)| *balance >= transfer_amount);
    if !has_transfer_coin {
        anyhow::bail!(
            "No single Coin<{}> object can cover requested amount {}",
            GAS_COIN,
            transfer_amount
        );
    }

    let has_gas_coin = native_coins
        .iter()
        .any(|(_, balance)| *balance >= required_gas);
    if !has_gas_coin {
        anyhow::bail!(
            "No spendable native gas coin object found with at least {} Mist",
            required_gas
        );
    }

    anyhow::bail!(
        "Native transfer needs one Coin<{}> with enough balance for transfer plus gas, or two distinct coins with enough balance for each",
        GAS_COIN
    )
}

pub(crate) fn select_native_coin_consolidation_step(
    owned_objects: &[kanari_rpc_api::ObjectInfo],
    sender: &str,
    required_amount: u64,
    gas_limit: u64,
    gas_price: u64,
) -> anyhow::Result<(
    kanari_rpc_api::ObjectInfo,
    kanari_rpc_api::ObjectInfo,
    GasPayment,
)> {
    let native_coin_type = CoinModule::coin_type(GAS_COIN);
    let required_gas = gas_limit.saturating_mul(effective_gas_price(gas_price));
    let native_coins: Vec<(kanari_rpc_api::ObjectInfo, u64)> = owned_objects
        .iter()
        .filter(|object| object.type_ == native_coin_type)
        .filter_map(|object| {
            CoinModule::read_balance(&object.data).map(|balance| (object.clone(), balance))
        })
        .filter(|(_, balance)| *balance > 0)
        .collect();

    if native_coins.len() < 2 {
        anyhow::bail!(
            "Native coin consolidation requires at least two spendable Coin<{}> objects; found {}",
            GAS_COIN,
            native_coins.len()
        );
    }

    let mut gas_candidates: Vec<(kanari_rpc_api::ObjectInfo, u64)> = native_coins
        .iter()
        .filter(|(_, balance)| *balance >= required_gas)
        .cloned()
        .collect();
    gas_candidates.sort_by_key(|(_, balance)| *balance);

    for (gas_object, _) in gas_candidates {
        let mut remaining: Vec<(kanari_rpc_api::ObjectInfo, u64)> = native_coins
            .iter()
            .filter(|(object, _)| object.id != gas_object.id)
            .cloned()
            .collect();

        let remaining_total = remaining
            .iter()
            .fold(0u64, |sum, (_, balance)| sum.saturating_add(*balance));
        if remaining_total < required_amount || remaining.len() < 2 {
            continue;
        }

        remaining.sort_by_key(|(_, balance)| std::cmp::Reverse(*balance));
        let (primary_object, primary_balance) = remaining[0].clone();
        if primary_balance >= required_amount {
            continue;
        }
        let (merge_object, _) = remaining[1].clone();

        let gas_payment = GasPayment {
            payment_objects: vec![ObjectRef::new(
                gas_object.id.clone(),
                Some(gas_object.version),
                gas_object.digest.clone(),
            )],
            owner: sender.to_string(),
            budget: gas_limit,
            price: gas_price,
            coin_type: None,
            price_version: None,
        };

        return Ok((primary_object, merge_object, gas_payment));
    }

    anyhow::bail!(
        "Native coin consolidation could not reserve a separate gas coin while preserving enough non-gas balance to reach {} Mist",
        required_amount
    )
}

pub(crate) fn select_coin_object_for_token(
    owned_objects: &[kanari_rpc_api::ObjectInfo],
    token_type: &str,
    required_amount: u64,
) -> anyhow::Result<ObjectRef> {
    let wanted_token = CoinModule::normalize_token_type(token_type);
    let mut best: Option<(ObjectRef, u64)> = None;
    let mut largest: Option<(ObjectRef, u64)> = None;

    for object in owned_objects {
        let Some(obj_token) = coin_token_type_from_object_type(&object.type_) else {
            continue;
        };
        if obj_token != wanted_token {
            continue;
        }
        let Some(balance) = CoinModule::read_balance(&object.data) else {
            continue;
        };
        let object_ref = ObjectRef::new(
            object.id.clone(),
            Some(object.version),
            object.digest.clone(),
        );
        if balance >= required_amount {
            match &best {
                Some((_, current)) if *current <= balance => {}
                _ => best = Some((object_ref.clone(), balance)),
            }
        }
        match &largest {
            Some((_, current)) if *current >= balance => {}
            _ => largest = Some((object_ref, balance)),
        }
    }

    let (selected, selected_balance) = best
        .or(largest)
        .ok_or_else(|| anyhow::anyhow!("No spendable Coin<{}> object found", wanted_token))?;
    if selected_balance < required_amount {
        anyhow::bail!(
            "No single Coin<{}> object can cover requested amount {}",
            wanted_token,
            required_amount
        );
    }
    Ok(selected)
}
