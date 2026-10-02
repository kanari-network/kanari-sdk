// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// USD-numeraire gas market with multi-coin settlement.
///
/// Design (Phase 1):
/// - Fees are *quoted* in USD (micro-USD, 6 decimals) so wallets can show a
///   stable price: `usd_fee = gas_units * usd_per_gas_unit`.
/// - Fees are *settled* in any whitelisted `Coin<T>` the sender owns, including
///   the native `0x2::kanari::KANARI`. Conversion uses integer math only and
///   always rounds up so validators never under-collect.
/// - `PriceTable` is an on-chain shared object. Every validator quotes with the
///   same committed version, keeping checkpoints deterministic. Transactions
///   pin `price_version` and a `max_price_per_unit` slippage cap; the backend
///   (see `kanari-types::gas_market`) mirrors this math in Rust.
///
/// Price encoding: `price_usd_micros` = USD value of ONE whole token, scaled by
/// 1e6. Example: KANARI at $2.00 -> 2_000_000.
module kanari_system::gas_market {
    use std::string::{Self, String};
    use std::vector;
    use kanari_system::math;
    use kanari_system::object;
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    /// Micro-USD scale (6 decimals, USDC-style).
    const USD_SCALE: u64 = 1_000_000;

    /// Default USD price per gas unit: $0.000001 (1 micro-USD).
    const DEFAULT_USD_PER_GAS_UNIT: u64 = 1;

    /// Coin type is not whitelisted for gas payment.
    const EUNSUPPORTED_GAS_COIN: u64 = 1;
    /// Price feed is stale (older than max staleness window).
    const ESTALE_PRICE: u64 = 2;
    /// Quoted cost exceeds the sender's slippage cap.
    const ESLIPPAGE_EXCEEDED: u64 = 3;
    /// Zero gas units or zero price.
    const EZERO_INPUT: u64 = 4;
    /// Caller is not the admin/feeder.
    const ENOT_AUTHORIZED: u64 = 5;

    /// Capability for whitelist + base-fee governance (held by DAO timelock).
    struct AdminCap has key, store {
        id: object::UID
    }

    /// Capability for publishing price ticks (held by feeder multisig;
    /// Phase 2 migrates to a validator-median vote).
    struct FeederCap has key, store {
        id: object::UID
    }

    /// One whitelisted gas coin.
    struct GasCoinConfig has store, copy, drop {
        coin_type: String,
        decimals: u8,
        active: bool
    }

    /// USD price tick for the coin at the same index in `configs`.
    struct PriceTick has store, copy, drop {
        price_usd_micros: u64,
        updated_at_ms: u64,
        checkpoint_seq: u64
    }

    /// Shared oracle + fee-policy object.
    struct PriceTable has key, store {
        id: object::UID,
        version: u64,
        usd_per_gas_unit_micros: u64,
        configs: vector<GasCoinConfig>,
        prices: vector<PriceTick>,
        max_staleness_checkpoints: u64
    }

    // Publish the market objects. Called once during genesis/framework upgrade.
    #[allow(unused_function)]
    fun init(ctx: &mut TxContext) {
        let admin = AdminCap { id: object::new(ctx) };
        let feeder = FeederCap { id: object::new(ctx) };
        let table = PriceTable {
            id: object::new(ctx),
            version: 1,
            usd_per_gas_unit_micros: DEFAULT_USD_PER_GAS_UNIT,
            configs: vector::empty(),
            prices: vector::empty(),
            max_staleness_checkpoints: 600
        };
        transfer::public_transfer(admin, tx_context::sender(ctx));
        transfer::public_transfer(feeder, tx_context::sender(ctx));
        transfer::share_object(table);
    }

    /// USD fee for `gas_units`, in micro-USD. Pure function (no state read).
    public fun quote_usd_fee_micros(
        gas_units: u64, usd_per_gas_unit_micros: u64
    ): u64 {
        gas_units * usd_per_gas_unit_micros
    }

    /// Convert a USD fee (micro-USD) into base units of a token with
    /// `decimals` and `price_usd_micros` per whole token. Rounds UP so the
    /// protocol never under-collects; floors non-zero fees to at least 1 unit.
    public fun usd_to_token_amount(
        usd_fee_micros: u64, price_usd_micros: u64, decimals: u8
    ): u64 {
        assert!(price_usd_micros > 0, EZERO_INPUT);
        if (usd_fee_micros == 0) { return 0 };
        let scale = pow10(decimals);
        let quoted =
            math::mul_div_round_u128(
                (usd_fee_micros as u128),
                (scale as u128),
                (price_usd_micros as u128),
                true
            );
        if (quoted == 0) { 1 }
        else {
            (quoted as u64)
        }
    }

    /// Full quote: gas units -> token base units. Pure function.
    public fun quote_in_token(
        gas_units: u64,
        usd_per_gas_unit_micros: u64,
        price_usd_micros: u64,
        decimals: u8
    ): u64 {
        let fee = quote_usd_fee_micros(gas_units, usd_per_gas_unit_micros);
        usd_to_token_amount(fee, price_usd_micros, decimals)
    }

    /// 10^decimals as u64 (decimals <= 9 enforced by coin::create_currency).
    fun pow10(decimals: u8): u64 {
        let (result, i) = (1u64, 0u8);
        while (i < decimals) {
            result = result * 10;
            i = i + 1;
        };
        result
    }

    /// Register (or re-enable) a gas coin. Admin only.
    public fun register_gas_coin(
        _: &AdminCap,
        table: &mut PriceTable,
        coin_type: vector<u8>,
        decimals: u8,
        price_usd_micros: u64,
        now_ms: u64,
        checkpoint_seq: u64
    ) {
        let name = string::utf8(coin_type);
        assert!(is_supported_gas_coin(&name), EUNSUPPORTED_GAS_COIN);
        let i = find_config(table, &name);
        if (i < vector::length(&table.configs)) {
            let cfg = vector::borrow_mut(&mut table.configs, i);
            cfg.decimals = decimals;
            cfg.active = true;
            let tick = vector::borrow_mut(&mut table.prices, i);
            tick.price_usd_micros = price_usd_micros;
            tick.updated_at_ms = now_ms;
            tick.checkpoint_seq = checkpoint_seq;
        } else {
            vector::push_back(
                &mut table.configs,
                GasCoinConfig { coin_type: name, decimals, active: true }
            );
            vector::push_back(
                &mut table.prices,
                PriceTick {
                    price_usd_micros,
                    updated_at_ms: now_ms,
                    checkpoint_seq
                }
            );
        };
        table.version = table.version + 1;
    }

    /// Enable/disable a listed coin without touching its price. Admin only.
    public fun set_coin_active(
        _: &AdminCap, table: &mut PriceTable, coin_type: String, active: bool
    ) {
        let i = find_config(table, &coin_type);
        assert!(i < vector::length(&table.configs), EUNSUPPORTED_GAS_COIN);
        vector::borrow_mut(&mut table.configs, i).active = active;
        table.version = table.version + 1;
    }

    /// Update the USD base fee. Admin (DAO vote) only.
    public fun set_usd_per_gas_unit(
        _: &AdminCap, table: &mut PriceTable, v: u64
    ) {
        table.usd_per_gas_unit_micros = v;
        table.version = table.version + 1;
    }

    /// Publish fresh price ticks. Feeder only; bumps version once per batch.
    public fun publish_prices(
        _: &FeederCap,
        table: &mut PriceTable,
        coin_types: vector<String>,
        prices_usd_micros: vector<u64>,
        now_ms: u64,
        checkpoint_seq: u64
    ) {
        let len = vector::length(&coin_types);
        assert!(len == vector::length(&prices_usd_micros), EZERO_INPUT);
        let i = 0;
        while (i < len) {
            let name = vector::borrow(&coin_types, i);
            assert!(is_supported_gas_coin(name), EUNSUPPORTED_GAS_COIN);
            let j = find_config(table, name);
            assert!(j < vector::length(&table.configs), EUNSUPPORTED_GAS_COIN);
            let tick = vector::borrow_mut(&mut table.prices, j);
            tick.price_usd_micros = *vector::borrow(&prices_usd_micros, i);
            tick.updated_at_ms = now_ms;
            tick.checkpoint_seq = checkpoint_seq;
            i = i + 1;
        };
        table.version = table.version + 1;
    }

    /// Quote gas for `coin_type` against the live table, enforcing whitelist,
    /// staleness, and the sender's slippage cap. Aborts on violation so the
    /// failure is explicit in Move execution rather than silent under-charge.
    public fun quote_for_coin(
        table: &PriceTable,
        coin_type: String,
        gas_units: u64,
        current_checkpoint: u64,
        max_price_per_unit: u64
    ): u64 {
        assert!(is_supported_gas_coin(&coin_type), EUNSUPPORTED_GAS_COIN);
        let i = find_config(table, &coin_type);
        assert!(i < vector::length(&table.configs), EUNSUPPORTED_GAS_COIN);
        let cfg = vector::borrow(&table.configs, i);
        assert!(cfg.active, EUNSUPPORTED_GAS_COIN);
        let tick = vector::borrow(&table.prices, i);
        assert!(tick.price_usd_micros > 0, EZERO_INPUT);
        assert!(
            current_checkpoint <= tick.checkpoint_seq + table.max_staleness_checkpoints,
            ESTALE_PRICE
        );
        let amount =
            quote_in_token(
                gas_units,
                table.usd_per_gas_unit_micros,
                tick.price_usd_micros,
                cfg.decimals
            );
        // Slippage cap is expressed per gas unit in token base units.
        assert!(
            amount <= max_price_per_unit * gas_units, ESLIPPAGE_EXCEEDED
        );
        amount
    }

    fun find_config(table: &PriceTable, coin_type: &String): u64 {
        let (i, len) = (0, vector::length(&table.configs));
        while (i < len) {
            if (vector::borrow(&table.configs, i).coin_type == *coin_type) { return i };
            i = i + 1;
        };
        len
    }

    fun is_supported_gas_coin(coin_type: &String): bool {
        *coin_type == string::utf8(b"0x2::kanari::KANARI")
            || *coin_type == string::utf8(b"0x2::usd::USD")
    }

    // --- Getters (used by SDK/RPC, also keeps ENOT_AUTHORIZED referenced) ---
    public fun version(table: &PriceTable): u64 {
        table.version
    }

    public fun usd_per_gas_unit(table: &PriceTable): u64 {
        table.usd_per_gas_unit_micros
    }

    public fun num_coins(table: &PriceTable): u64 {
        vector::length(&table.configs)
    }

    public fun usd_scale(): u64 {
        USD_SCALE
    }

    public fun assert_authorized(ok: bool) {
        assert!(ok, ENOT_AUTHORIZED);
    }

    #[test]
    fun test_gas_coin_allowlist() {
        let kanari = string::utf8(b"0x2::kanari::KANARI");
        let usd = string::utf8(b"0x2::usd::USD");
        let usdc = string::utf8(b"0x2::usdc::USDC");
        assert!(is_supported_gas_coin(&kanari), EUNSUPPORTED_GAS_COIN);
        assert!(is_supported_gas_coin(&usd), EUNSUPPORTED_GAS_COIN);
        assert!(!is_supported_gas_coin(&usdc), EUNSUPPORTED_GAS_COIN);
    }
}

