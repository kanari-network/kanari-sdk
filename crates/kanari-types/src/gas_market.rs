// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! USD-numeraire gas market (Phase 1).
//!
//! Mirrors `kanari_system::gas_market` (Move) in pure integer math so the
//! wallet/RPC can quote off-chain exactly what the on-chain `quote_in_token`
//! would charge. No floats anywhere: USD is micro-USD (1e6), prices are
//! micro-USD per whole token, token amounts are base units.
//!
//! Settlement rule: `token_amount = ceil(usd_fee * 10^decimals / price)` with
//! a floor of 1 base unit for any non-zero fee (same zero-cost guard as the
//! priced gas models in [`crate::gas`]).

use crate::address::Address;
use crate::coin::CoinModule;
use crate::gas_coin::{GAS_COIN, GasModule};
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One whitelisted gas coin and its USD price tick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GasCoinEntry {
    /// Canonical token type, e.g. `0x2::kanari::KANARI`.
    pub coin_type: String,
    pub decimals: u8,
    pub active: bool,
    /// Micro-USD per whole token. Zero = no price published yet.
    pub price_usd_micros: u64,
    pub price_version: u64,
}

/// On-chain `PriceTable` snapshot used for deterministic quoting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GasPriceTable {
    pub version: u64,
    pub usd_per_gas_unit_micros: u64,
    pub max_staleness_versions: u64,
    pub entries: Vec<GasCoinEntry>,
}

impl Default for GasPriceTable {
    fn default() -> Self {
        Self {
            version: 1,
            usd_per_gas_unit_micros: GasMarketModule::DEFAULT_USD_PER_GAS_UNIT_MICROS,
            max_staleness_versions: 600,
            entries: vec![
                GasCoinEntry {
                    coin_type: GAS_COIN.to_string(),
                    decimals: GasModule::KANARI_DECIMALS as u8,
                    active: true,
                    // Default $2.00/KANARI seed; the feeder overwrites on-chain.
                    price_usd_micros: 2_000_000,
                    price_version: 1,
                },
                GasCoinEntry {
                    coin_type: crate::usd_coin::USD_COIN.to_string(),
                    decimals: crate::usd_coin::UsdModule::USD_DECIMALS as u8,
                    active: true,
                    // Stablecoin peg; the feeder overwrites on-chain.
                    price_usd_micros: 1_000_000,
                    price_version: 1,
                },
            ],
        }
    }
}

impl GasPriceTable {
    /// Retained for API compatibility. Gas settlement is limited to KANARI and
    /// USD; arbitrary node-local coin configuration is intentionally ignored.
    pub fn from_env_or_default() -> Self {
        Self::default()
    }

    /// Canonical lookup (accepts `0x02..` style spellings).
    pub fn entry_for(&self, coin_type: &str) -> Option<&GasCoinEntry> {
        let wanted = CoinModule::normalize_token_type(coin_type);
        if !CoinModule::is_native_token_type(&wanted) && wanted != crate::usd_coin::USD_COIN {
            return None;
        }
        self.entries.iter().find(|entry| {
            entry.active && CoinModule::normalize_token_type(&entry.coin_type) == wanted
        })
    }

    /// Whitelist check used by admission/validation.
    pub fn is_supported(&self, coin_type: &str) -> bool {
        self.entry_for(coin_type)
            .is_some_and(|entry| entry.price_usd_micros > 0)
    }

    /// Pin check: the transaction's price version must not be stale.
    pub fn check_version(&self, tx_version: Option<u64>) -> Result<(), String> {
        let Some(pinned) = tx_version else {
            return Ok(());
        };
        if pinned > self.version {
            return Err(format!(
                "gas price version {pinned} is newer than committed {}",
                self.version
            ));
        }
        if self.version.saturating_sub(pinned) > self.max_staleness_versions {
            return Err(format!(
                "gas price version {pinned} is stale (table v{})",
                self.version
            ));
        }
        Ok(())
    }

    /// Full quote for `gas_units` settled in `coin_type`.
    pub fn quote_in_token(&self, gas_units: u64, coin_type: &str) -> Result<u64, String> {
        let entry = self
            .entry_for(coin_type)
            .ok_or_else(|| format!("unsupported gas coin for settlement: {coin_type}"))?;
        if entry.price_usd_micros == 0 {
            return Err(format!("no USD price published for {coin_type}"));
        }
        Ok(GasMarketModule::quote_in_token(
            gas_units,
            self.usd_per_gas_unit_micros,
            entry.price_usd_micros,
            entry.decimals,
        ))
    }
}

/// Gas market module constants and utilities, mirroring `kanari_system::gas_market`.
pub struct GasMarketModule;

impl GasMarketModule {
    pub const GAS_MARKET_MODULE: &'static str = "gas_market";

    /// Micro-USD scale (6 decimals, USDC-style).
    pub const USD_SCALE: u64 = 1_000_000;
    /// Fixed-point price scale (matches MIST 1e9 granularity).
    pub const PRICE_SCALE: u64 = 1_000_000_000;
    /// Default USD price per gas unit: $0.000001 (1 micro-USD).
    pub const DEFAULT_USD_PER_GAS_UNIT_MICROS: u64 = 1;

    /// Abort codes from `gas_market.move` (`EUNSUPPORTED_GAS_COIN`, ...).
    pub const EUNSUPPORTED_GAS_COIN: u64 = 1;
    pub const ESTALE_PRICE: u64 = 2;
    pub const ESLIPPAGE_EXCEEDED: u64 = 3;
    pub const EZERO_INPUT: u64 = 4;
    pub const ENOT_AUTHORIZED: u64 = 5;
    pub const EOVERFLOW_QUOTE: u64 = 6;
    pub const ELENGTH_MISMATCH: u64 = 7;
    pub const EINVALID_DECIMALS: u64 = 8;

    /// Get the module ID for `kanari_system::gas_market`
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;

        let module_name =
            Identifier::new(Self::GAS_MARKET_MODULE).context("Invalid gas_market module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Compatibility alias for [`Self::get_module_id`].
    pub fn module_id() -> Result<ModuleId> {
        Self::get_module_id()
    }

    /// Fully qualified Move module path for `0x2::gas_market`.
    pub fn module_path() -> String {
        format!(
            "{}::{}",
            Address::KANARI_SYSTEM_ADDRESS,
            Self::GAS_MARKET_MODULE
        )
    }

    /// USD fee for `gas_units`, in micro-USD.
    pub fn quote_usd_fee_micros(gas_units: u64, usd_per_gas_unit_micros: u64) -> u64 {
        gas_units.saturating_mul(usd_per_gas_unit_micros)
    }

    /// Convert a USD fee (micro-USD) into token base units. Integer-only, rounds
    /// UP, floors non-zero fees to 1 base unit. Mirrors Move `usd_to_token_amount`.
    pub fn usd_to_token_amount(usd_fee_micros: u64, price_usd_micros: u64, decimals: u8) -> u64 {
        assert!(price_usd_micros > 0, "gas price must be non-zero");
        if usd_fee_micros == 0 {
            return 0;
        }
        let scale = 10u128.pow(decimals.min(18) as u32);
        let numerator = (usd_fee_micros as u128).saturating_mul(scale);
        let denominator = price_usd_micros as u128;
        let quoted = numerator.div_ceil(denominator);
        quoted.max(1).min(u64::MAX as u128) as u64
    }

    /// Full quote: gas units -> token base units. Mirrors Move `quote_in_token`.
    pub fn quote_in_token(
        gas_units: u64,
        usd_per_gas_unit_micros: u64,
        price_usd_micros: u64,
        decimals: u8,
    ) -> u64 {
        let fee = Self::quote_usd_fee_micros(gas_units, usd_per_gas_unit_micros);
        Self::usd_to_token_amount(fee, price_usd_micros, decimals)
    }

    /// Format a micro-USD fee as `$X.YYYYYY` (integer math, trimmed).
    pub fn format_usd_micros(micros: u64) -> String {
        let dollars = micros / Self::USD_SCALE;
        let cents = micros % Self::USD_SCALE;
        if cents == 0 {
            return format!("${dollars}");
        }
        let frac = format!("{cents:06}").trim_end_matches('0').to_string();
        format!("${dollars}.{frac}")
    }

    /// Auto-select settlement coin: "If your wallet holds a particular coin, use that coin for gas.".
    ///
    /// Preference order: USD first, then native KANARI. Returns the coin type and
    /// the max-cost quote in that coin.
    pub fn select_gas_coin(
        balances: &BTreeMap<String, u64>,
        gas_limit: u64,
        table: &GasPriceTable,
        transfer_needs: &BTreeMap<String, u64>,
    ) -> Option<(String, u64)> {
        if let Some((coin, balance)) = balances
            .iter()
            .find(|(coin, _)| CoinModule::normalize_token_type(coin) == crate::usd_coin::USD_COIN)
            && table.is_supported(coin)
        {
            let quote = table.quote_in_token(gas_limit, coin).ok()?;
            let need = quote.saturating_add(transfer_needs.get(coin).copied().unwrap_or(0));
            if *balance >= need {
                return Some((coin.clone(), quote));
            }
        }

        let mut stables: Vec<(&String, &u64)> = Vec::new();
        let mut others: Vec<(&String, &u64)> = Vec::new();

        for (coin, balance) in balances {
            if !table.is_supported(coin) {
                continue;
            }
            let entry = table.entry_for(coin)?;
            if CoinModule::is_native_token_type(&entry.coin_type)
                || CoinModule::is_native_token_type(coin)
            {
                let quote = table.quote_in_token(gas_limit, coin).ok()?;
                let need = quote.saturating_add(transfer_needs.get(coin).copied().unwrap_or(0));
                if *balance >= need {
                    return Some((coin.clone(), quote));
                }
                continue;
            }
            if entry.decimals == 6 {
                stables.push((coin, balance));
            } else {
                others.push((coin, balance));
            }
        }

        // Stablecoins sorted by balance (proxy for affordability), then others by
        // quoted USD value descending.
        stables.sort_by_key(|(_, balance)| std::cmp::Reverse(**balance));
        others.sort_by_key(|(coin, _)| {
            std::cmp::Reverse(
                table
                    .quote_in_token(u64::MAX / 1_000_000, coin)
                    .unwrap_or(0),
            )
        });

        for (coin, balance) in stables.into_iter().chain(others) {
            let quote = table.quote_in_token(gas_limit, coin).ok()?;
            let need = quote.saturating_add(
                transfer_needs
                    .iter()
                    .find(|(token, _)| {
                        CoinModule::normalize_token_type(token)
                            == CoinModule::normalize_token_type(coin)
                    })
                    .map(|(_, amount)| *amount)
                    .unwrap_or(0),
            );
            if *balance >= need {
                return Some((coin.clone(), quote));
            }
        }
        None
    }

    /// Canonical function names used in `gas_market.move`.
    pub fn function_names() -> GasMarketFunctions {
        GasMarketFunctions {
            quote_usd_fee_micros: "quote_usd_fee_micros",
            usd_to_token_amount: "usd_to_token_amount",
            quote_in_token: "quote_in_token",
            register_gas_coin: "register_gas_coin",
            set_coin_active: "set_coin_active",
            set_usd_per_gas_unit: "set_usd_per_gas_unit",
            publish_prices: "publish_prices",
            quote_for_coin: "quote_for_coin",
            version: "version",
            usd_per_gas_unit: "usd_per_gas_unit",
            num_coins: "num_coins",
            usd_scale: "usd_scale",
            assert_authorized: "assert_authorized",
        }
    }
}

/// Compatibility alias functions maintained for top-level usage.
pub fn gas_market_module_path() -> String {
    GasMarketModule::module_path()
}

pub fn quote_usd_fee_micros(gas_units: u64, usd_per_gas_unit_micros: u64) -> u64 {
    GasMarketModule::quote_usd_fee_micros(gas_units, usd_per_gas_unit_micros)
}

pub fn usd_to_token_amount(usd_fee_micros: u64, price_usd_micros: u64, decimals: u8) -> u64 {
    GasMarketModule::usd_to_token_amount(usd_fee_micros, price_usd_micros, decimals)
}

pub fn quote_in_token(
    gas_units: u64,
    usd_per_gas_unit_micros: u64,
    price_usd_micros: u64,
    decimals: u8,
) -> u64 {
    GasMarketModule::quote_in_token(
        gas_units,
        usd_per_gas_unit_micros,
        price_usd_micros,
        decimals,
    )
}

pub fn format_usd_micros(micros: u64) -> String {
    GasMarketModule::format_usd_micros(micros)
}

pub fn select_gas_coin(
    balances: &BTreeMap<String, u64>,
    gas_limit: u64,
    table: &GasPriceTable,
    transfer_needs: &BTreeMap<String, u64>,
) -> Option<(String, u64)> {
    GasMarketModule::select_gas_coin(balances, gas_limit, table, transfer_needs)
}

/// Gas market function names, mirroring the public API of `gas_market.move`.
pub struct GasMarketFunctions {
    pub quote_usd_fee_micros: &'static str,
    pub usd_to_token_amount: &'static str,
    pub quote_in_token: &'static str,
    pub register_gas_coin: &'static str,
    pub set_coin_active: &'static str,
    pub set_usd_per_gas_unit: &'static str,
    pub publish_prices: &'static str,
    pub quote_for_coin: &'static str,
    pub version: &'static str,
    pub usd_per_gas_unit: &'static str,
    pub num_coins: &'static str,
    pub usd_scale: &'static str,
    pub assert_authorized: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usd_quote_matches_move_math() {
        // 100 units * $0.00002 = $0.002 = 2000 micros.
        assert_eq!(GasMarketModule::quote_usd_fee_micros(100, 20), 2000);
        // KANARI $2.00, 9 decimals: $0.002 -> 0.001 KANARI = 1_000_000 MIST.
        assert_eq!(
            GasMarketModule::usd_to_token_amount(2000, 2_000_000, 9),
            1_000_000
        );
        // USDC $1.00, 6 decimals: $0.002 -> 2000 base units.
        assert_eq!(
            GasMarketModule::usd_to_token_amount(2000, 1_000_000, 6),
            2000
        );
        // Rounds up, never zero for non-zero fees.
        assert_eq!(
            GasMarketModule::usd_to_token_amount(1, 1_000_000_000_000, 9),
            1
        );
        assert_eq!(GasMarketModule::usd_to_token_amount(0, 2_000_000, 9), 0);
    }

    #[test]
    fn default_table_quotes_kanari() {
        let table = GasPriceTable::default();
        assert!(table.is_supported(GAS_COIN));
        assert!(table.is_supported(crate::usd_coin::USD_COIN));
        assert!(!table.is_supported("0x2::usdc::USDC"));
        assert_eq!(table.usd_per_gas_unit_micros, 1);
        assert_eq!(table.quote_in_token(100, GAS_COIN).unwrap(), 50_000);
        assert_eq!(
            table
                .quote_in_token(100_000, crate::usd_coin::USD_COIN)
                .unwrap(),
            100_000
        );

        let custom_table = GasPriceTable {
            entries: vec![GasCoinEntry {
                coin_type: "0x2::usdc::USDC".to_string(),
                decimals: 6,
                active: true,
                price_usd_micros: 1_000_000,
                price_version: 1,
            }],
            ..table
        };
        assert!(!custom_table.is_supported("0x2::usdc::USDC"));
        assert!(custom_table.quote_in_token(100, "0x2::usdc::USDC").is_err());
    }

    #[test]
    fn version_pin_rejects_future_and_stale() {
        let table = GasPriceTable::default();
        assert!(table.check_version(None).is_ok());
        assert!(table.check_version(Some(1)).is_ok());
        assert!(table.check_version(Some(2)).is_err());
        let stale = GasPriceTable {
            version: 1000,
            ..GasPriceTable::default()
        };
        assert!(stale.check_version(Some(1)).is_err());
    }

    #[test]
    fn selects_usd_first_when_available() {
        let table = GasPriceTable::default();
        let balances = BTreeMap::from([
            (GAS_COIN.to_string(), 10_000_000),
            (crate::usd_coin::USD_COIN.to_string(), 10_000_000),
        ]);
        let (coin, quote) =
            GasMarketModule::select_gas_coin(&balances, 100, &table, &BTreeMap::new()).unwrap();
        assert_eq!(coin, crate::usd_coin::USD_COIN);
        assert_eq!(quote, 100);
    }

    #[test]
    fn falls_back_to_kanari_when_usd_is_unavailable() {
        let table = GasPriceTable::default();
        let balances = BTreeMap::from([(GAS_COIN.to_string(), 10_000_000)]);
        let (coin, quote) =
            GasMarketModule::select_gas_coin(&balances, 100, &table, &BTreeMap::new()).unwrap();
        assert_eq!(coin, GAS_COIN);
        assert_eq!(quote, 50_000);
    }

    #[test]
    fn skips_unsupported_and_unaffordable_coins() {
        let table = GasPriceTable::default();
        let balances = BTreeMap::from([
            ("0x2::shit::COIN".to_string(), u64::MAX),
            (GAS_COIN.to_string(), 1),
        ]);
        assert!(
            GasMarketModule::select_gas_coin(&balances, 100, &table, &BTreeMap::new()).is_none()
        );
    }

    #[test]
    fn usd_formatting_is_exact() {
        assert_eq!(GasMarketModule::format_usd_micros(0), "$0");
        assert_eq!(GasMarketModule::format_usd_micros(2_000_000), "$2");
        assert_eq!(GasMarketModule::format_usd_micros(2000), "$0.002");
    }
}
