// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! The framework Move sources are the single source of truth for abort codes
//! and protocol constants. This test reads
//! `crates/kanari-frameworks/packages/kanari-system/sources/*.move` and checks
//! every value mirrored in `kanari-types` against it, so a drift in the Move
//! sources fails the Rust test suite instead of silently shipping a stale
//! mirror.

use kanari_types::address::{Address, AddressModule};
use kanari_types::balance::BalanceModule;
use kanari_types::clock::ClockModule;
use kanari_types::coin::CoinModule;
use kanari_types::collection::CollectionModule;
use kanari_types::deny_list::DenyListModule;
use kanari_types::gas_coin::GasModule;
use kanari_types::gas_market::GasMarketModule;
use kanari_types::pay::PayModule;
use kanari_types::transfer::TransferModule;
use kanari_types::tx_context::TxContextModule;
use kanari_types::usd_coin::UsdModule;
use std::path::Path;

/// `(Move source file, &[(Move const name, mirrored Rust value)])`.
const MIRRORED: &[(&str, &[(&str, u64)])] = &[
    (
        "address.move",
        &[
            ("LENGTH", Address::LENGTH as u64),
            ("EAddressParseError", AddressModule::E_ADDRESS_PARSE_ERROR),
        ],
    ),
    (
        "balance.move",
        &[
            (
                "ERR_INSUFFICIENT_BALANCE",
                BalanceModule::ERR_INSUFFICIENT_BALANCE,
            ),
            ("ERR_OVERFLOW", BalanceModule::ERR_OVERFLOW),
            ("ERR_ZERO_AMOUNT", BalanceModule::ERR_ZERO_AMOUNT),
            (
                "ERR_INSUFFICIENT_SUPPLY",
                BalanceModule::ERR_INSUFFICIENT_SUPPLY,
            ),
        ],
    ),
    (
        "clock.move",
        &[
            ("E_NOT_SYSTEM_ADDRESS", ClockModule::E_NOT_SYSTEM_ADDRESS),
            (
                "E_TIMESTAMP_NOT_MONOTONIC",
                ClockModule::E_TIMESTAMP_NOT_MONOTONIC,
            ),
        ],
    ),
    (
        "coin.move",
        &[
            ("EInvalidArg", CoinModule::E_INVALID_ARG),
            ("ENotEnough", CoinModule::E_NOT_ENOUGH),
            ("EZERO_AMOUNT", CoinModule::E_ZERO_AMOUNT),
            ("EOVERFLOW", CoinModule::E_OVERFLOW),
            ("EUNDERFLOW", CoinModule::E_UNDERFLOW),
            ("EINVALID_DECIMALS", CoinModule::E_INVALID_DECIMALS),
            ("ETOO_MANY_PARTS", CoinModule::E_TOO_MANY_PARTS),
            (
                "EGlobalPauseNotAllowed",
                CoinModule::E_GLOBAL_PAUSE_NOT_ALLOWED,
            ),
            ("MAX_SPLIT_PARTS", CoinModule::MAX_SPLIT_PARTS),
        ],
    ),
    (
        "collection.move",
        &[
            ("E_NO_SUPPLY", CollectionModule::E_NO_SUPPLY),
            (
                "E_NOT_COLLECTION_CREATOR",
                CollectionModule::E_NOT_COLLECTION_CREATOR,
            ),
            (
                "E_CAP_COLLECTION_MISMATCH",
                CollectionModule::E_CAP_COLLECTION_MISMATCH,
            ),
        ],
    ),
    (
        "deny_list.move",
        &[
            ("ENotSystemAddress", DenyListModule::E_NOT_SYSTEM_ADDRESS),
            ("ENotDenied", DenyListModule::E_NOT_DENIED),
            ("EInvalidAddress", DenyListModule::E_INVALID_ADDRESS),
            ("COIN_INDEX", DenyListModule::COIN_INDEX),
        ],
    ),
    (
        "gas_market.move",
        &[
            ("USD_SCALE", GasMarketModule::USD_SCALE),
            (
                "DEFAULT_USD_PER_GAS_UNIT",
                GasMarketModule::DEFAULT_USD_PER_GAS_UNIT_MICROS,
            ),
            (
                "EUNSUPPORTED_GAS_COIN",
                GasMarketModule::EUNSUPPORTED_GAS_COIN,
            ),
            ("ESTALE_PRICE", GasMarketModule::ESTALE_PRICE),
            ("ESLIPPAGE_EXCEEDED", GasMarketModule::ESLIPPAGE_EXCEEDED),
            ("EZERO_INPUT", GasMarketModule::EZERO_INPUT),
            ("ENOT_AUTHORIZED", GasMarketModule::ENOT_AUTHORIZED),
            ("EOVERFLOW_QUOTE", GasMarketModule::EOVERFLOW_QUOTE),
            ("ELENGTH_MISMATCH", GasMarketModule::ELENGTH_MISMATCH),
            ("EINVALID_DECIMALS", GasMarketModule::EINVALID_DECIMALS),
        ],
    ),
    (
        "kanari.move",
        &[
            ("MIST_PER_KANARI", GasModule::MIST_PER_GAS),
            ("TOTAL_SUPPLY_KANARI", GasModule::TOTAL_SUPPLY_GAS),
            ("TOTAL_SUPPLY_MIST", GasModule::TOTAL_SUPPLY_MIST),
            ("DEV_GAS_RESERVE_MIST", GasModule::DEV_GAS_RESERVE_MIST),
        ],
    ),
    (
        "pay.move",
        &[
            ("ENoCoins", PayModule::E_NO_COINS),
            ("EDENIED", PayModule::EDENIED),
            ("ETOO_MANY_COINS", PayModule::E_TOO_MANY_COINS),
            ("ESELF_PAY", PayModule::E_SELF_PAY),
            ("EZERO_AMOUNT", PayModule::E_ZERO_AMOUNT),
            ("EDIVISION_BY_ZERO", PayModule::E_DIVISION_BY_ZERO),
        ],
    ),
    (
        "transfer.move",
        &[
            ("ERR_INVALID_AMOUNT", TransferModule::ERR_INVALID_AMOUNT),
            ("ERR_SAME_ADDRESS", TransferModule::ERR_SAME_ADDRESS),
            ("ERR_OVERFLOW", TransferModule::ERR_OVERFLOW),
        ],
    ),
    (
        "tx_context.move",
        &[
            ("TX_HASH_LENGTH", TxContextModule::TX_HASH_LENGTH),
            ("EBadTxHashLength", TxContextModule::E_BAD_TX_HASH_LENGTH),
            ("ENoIDsCreated", TxContextModule::E_NO_IDS_CREATED),
        ],
    ),
    ("usd.move", &[("UNITS_PER_USD", UsdModule::UNITS_PER_USD)]),
];

/// Locate the framework sources next to this crate in the monorepo. Returns
/// `None` (and the test skips) when kanari-frameworks is not checked out.
fn move_sources_dir() -> Option<std::path::PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("kanari-frameworks")
        .join("packages")
        .join("kanari-system")
        .join("sources");
    dir.is_dir().then_some(dir)
}

/// Read `const NAME: <type> = <value>;` out of a Move source, handling both
/// same-line and next-line value spellings. Returns `None` when absent.
fn move_const_value(source: &str, name: &str) -> Option<u64> {
    let lines: Vec<&str> = source.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("const ") else {
            continue;
        };
        let Some(rest) = rest.strip_prefix(name) else {
            continue;
        };
        if !rest.starts_with(':') {
            continue;
        }
        let after_eq = rest.split_once('=').map(|(_, v)| v).unwrap_or("");
        let candidate = if after_eq.trim().is_empty() {
            lines.get(i + 1).map(|l| l.trim()).unwrap_or("")
        } else {
            after_eq.trim()
        };
        let digits: String = candidate
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '_')
            .collect();
        return digits.replace('_', "").parse().ok();
    }
    None
}

#[test]
fn mirrored_framework_constants_match_move_sources() {
    let Some(dir) = move_sources_dir() else {
        eprintln!("kanari-frameworks sources not found; skipping drift check");
        return;
    };

    for (file, consts) in MIRRORED {
        let source = std::fs::read_to_string(dir.join(file))
            .unwrap_or_else(|e| panic!("failed to read {file}: {e}"));
        for (move_name, rust_value) in *consts {
            let Some(move_value) = move_const_value(&source, move_name) else {
                panic!("{file} no longer defines const {move_name}; update kanari-types");
            };
            assert_eq!(
                *rust_value, move_value,
                "{file}::{move_name} drifted: framework says {move_value}, kanari-types says {rust_value}"
            );
        }
    }
}
