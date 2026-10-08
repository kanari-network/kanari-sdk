//! Uniswap-style constant-product AMM exercised end to end.
//!
//! Proves the v2 runtime supports DeFi workloads: LP share accounting,
//! exact-in swaps with slippage protection, full draining of coin inputs
//! through `&mut` bindings, by-value receipt consumption, event emission,
//! view queries, and abort safety — all observed from freshly constructed
//! runtime instances sharing one store.

use anyhow::{Context, Result};
use kanari_move_runtime_v2::changeset::ChangeSet;
use kanari_move_runtime_v2::move_runtime::{EntryFunctionObjectContext, MoveRuntime};
use kanari_move_runtime_v2::storage::persistent_store::PersistentStore;
use kanari_types::transaction::{ObjectInput, ObjectOwnerKind, ObjectRef};
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::{ModuleId, StructTag, TypeTag};
use move_package::BuildConfig;
use serde_json::Value as JsonValue;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use tempfile::tempdir;

#[path = "support/move_manifest.rs"]
mod move_manifest;

const TESTER_ADDR: &str = "0x4c";
const MODULE_NAME: &str = "uniswap_amm";

const MOVE_SOURCE: &str = include_str!("move_packages/uniswap_like_amm/sources/uniswap_amm.move");

const USD_TYPE: &str = "0x4c::uniswap_amm::USD_MOCK";
const KAN_TYPE: &str = "0x4c::uniswap_amm::KAN_MOCK";

/// Input-side fee kept by the pool (9970 bps = 0.30%), mirrored from Move.
const FEE_BPS: u128 = 9970;

const USD_INITIAL: u64 = 10_000_000;
const KAN_INITIAL: u64 = 20_000_000;
const SWAP_IN: u64 = 100_000;

#[test]
fn uniswap_like_amm_full_cycle() -> Result<()> {
    let package_dir = create_test_package()?;
    let (module_id, module_bytes) = compile_test_module(&package_dir)?;
    let sender = *module_id.address();
    assert_eq!(sender, AccountAddress::from_hex_literal(TESTER_ADDR)?);

    let store = Arc::new(PersistentStore::open_in_memory()?);
    let mut nonce = Nonce::default();
    let writer = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;

    let _ = writer
        .publish_module(module_bytes, sender, None, None)
        .context("publish uniswap-like amm module")?;

    // Faucet both sides of the pool.
    let usd_coin = mint(&writer, &module_id, USD_TYPE, USD_INITIAL, &mut nonce)?;
    let kan_coin = mint(&writer, &module_id, KAN_TYPE, KAN_INITIAL, &mut nonce)?;

    // Empty pool.
    let create = entry(
        &writer,
        &module_id,
        "create_pool",
        type_tags(&[USD_TYPE, KAN_TYPE])?,
        vec![],
        vec![],
        &mut nonce,
    )?;
    let pool = find_created(&create, |t| t.contains("::uniswap_amm::Pool"))?;
    let pool_id = pool.id;

    // Seed liquidity: both faucet coins are drained to zero via writeback,
    // and an LP receipt appears.
    let add = entry(
        &writer,
        &module_id,
        "add_liquidity",
        type_tags(&[USD_TYPE, KAN_TYPE])?,
        vec![
            object_arg(&pool_id)?,
            object_arg(&usd_coin)?,
            object_arg(&kan_coin)?,
        ],
        vec![
            owned_input(&pool_id, sender, true),
            owned_input(&usd_coin, sender, true),
            owned_input(&kan_coin, sender, true),
        ],
        &mut nonce,
    )?;
    let usd_drained = find_created(&add, |t| t.contains("coin::Coin") && t.contains("USD_MOCK"))?;
    assert_eq!(
        coin_balance(&usd_drained.data),
        0,
        "add_liquidity must drain the USD coin"
    );
    let kan_drained = find_created(&add, |t| t.contains("coin::Coin") && t.contains("KAN_MOCK"))?;
    assert_eq!(
        coin_balance(&kan_drained.data),
        0,
        "add_liquidity must drain the KAN coin"
    );
    let receipt = find_created(&add, |t| t.contains("::uniswap_amm::Receipt"))?;
    assert_eq!(
        receipt.owner, sender,
        "LP receipt must be owned by the sender"
    );
    let initial_shares = initial_lp_shares();
    let add_data = find_event_data(&add, "::AddLiquidityEvent")?;
    assert_eq!(add_data.len(), 40, "AddLiquidityEvent is address + u64");
    assert_eq!(event_address(add_data), sender.to_vec());
    assert_eq!(
        event_u64(add_data, 32),
        initial_shares,
        "first deposit must mint sqrt(x * y) shares"
    );

    // A fresh runtime instance reads the persisted pool.
    let reader = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    let reserves = reserves_view(&reader, &pool_id)?;
    assert_eq!(reserves, json!([USD_INITIAL, KAN_INITIAL]));
    let lp = lp_view(&reader, &pool_id)?;
    assert_eq!(lp, JsonValue::from(initial_shares));

    let expected_out = expected_swap_out(SWAP_IN, USD_INITIAL, KAN_INITIAL);
    let quoted = quote_view(&reader, &pool_id, SWAP_IN)?;
    assert_eq!(
        quoted, expected_out,
        "quote view must match the Rust-side formula"
    );

    // Exact-in swap at the quoted rate.
    let swap_in_coin = mint(&writer, &module_id, USD_TYPE, SWAP_IN, &mut nonce)?;
    let swap = entry(
        &writer,
        &module_id,
        "swap_exact_x_for_y",
        type_tags(&[USD_TYPE, KAN_TYPE])?,
        vec![
            object_arg(&pool_id)?,
            object_arg(&swap_in_coin)?,
            bcs::to_bytes(&expected_out)?,
        ],
        vec![
            owned_input(&pool_id, sender, true),
            owned_input(&swap_in_coin, sender, true),
        ],
        &mut nonce,
    )?;
    let swap_drained = find_created(&swap, |t| {
        t.contains("coin::Coin") && t.contains("USD_MOCK")
    })?;
    assert_eq!(
        coin_balance(&swap_drained.data),
        0,
        "swap must drain the input coin"
    );
    let out_coin = find_created(&swap, |t| {
        t.contains("coin::Coin") && t.contains("KAN_MOCK")
    })?;
    assert_eq!(
        coin_balance(&out_coin.data),
        expected_out,
        "output coin must carry the quoted amount"
    );
    assert_eq!(out_coin.owner, sender, "output coin pays the sender");
    let swap_data = find_event_data(&swap, "::SwapEvent")?;
    assert_eq!(swap_data.len(), 48, "SwapEvent is address + 2 u64");
    assert_eq!(event_address(swap_data), sender.to_vec());
    assert_eq!(event_u64(swap_data, 32), SWAP_IN);
    assert_eq!(
        event_u64(swap_data, 40),
        expected_out,
        "SwapEvent must report the executed output"
    );

    let usd_after = USD_INITIAL + SWAP_IN;
    let kan_after = KAN_INITIAL - expected_out;
    let k_before = USD_INITIAL as u128 * KAN_INITIAL as u128;
    let k_after = usd_after as u128 * kan_after as u128;
    assert!(
        k_after >= k_before,
        "collected fees must not shrink the x * k invariant"
    );

    let reader2 = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    assert_eq!(
        reserves_view(&reader2, &pool_id)?,
        json!([usd_after, kan_after]),
        "a fresh instance must observe the post-swap reserves"
    );

    // Slippage floor above the new quote aborts and leaves state untouched.
    let fail_coin = mint(&writer, &module_id, USD_TYPE, SWAP_IN, &mut nonce)?;
    let too_much = expected_out + 1;
    let err = entry(
        &writer,
        &module_id,
        "swap_exact_x_for_y",
        type_tags(&[USD_TYPE, KAN_TYPE])?,
        vec![
            object_arg(&pool_id)?,
            object_arg(&fail_coin)?,
            bcs::to_bytes(&too_much)?,
        ],
        vec![
            owned_input(&pool_id, sender, true),
            owned_input(&fail_coin, sender, true),
        ],
        &mut nonce,
    )
    .expect_err("swap below min_out must abort");
    let err_debug = format!("{err:?}");
    assert!(
        err_debug.contains("exec error"),
        "slippage failure must surface as a VM abort, got: {err_debug}"
    );
    assert_eq!(
        reserves_view(&reader2, &pool_id)?,
        json!([usd_after, kan_after]),
        "an aborted swap must not mutate the pool"
    );

    // Withdraw everything: the receipt is consumed and both reserves return.
    let remove = entry(
        &writer,
        &module_id,
        "remove_liquidity",
        type_tags(&[USD_TYPE, KAN_TYPE])?,
        vec![object_arg(&receipt.id)?, object_arg(&pool_id)?],
        vec![
            owned_input(&receipt.id, sender, true),
            owned_input(&pool_id, sender, true),
        ],
        &mut nonce,
    )?;
    assert!(
        remove.deleted_objects.contains(&receipt.id),
        "remove_liquidity must consume the receipt"
    );
    let usd_back = find_created(&remove, |t| {
        t.contains("coin::Coin") && t.contains("USD_MOCK")
    })?;
    let kan_back = find_created(&remove, |t| {
        t.contains("coin::Coin") && t.contains("KAN_MOCK")
    })?;
    assert_ne!(usd_back.id, kan_back.id, "distinct output coins");
    assert_eq!(
        coin_balance(&usd_back.data),
        usd_after,
        "the only LP withdraws the full USD reserve"
    );
    assert_eq!(
        coin_balance(&kan_back.data),
        kan_after,
        "the only LP withdraws the full KAN reserve"
    );
    let remove_data = find_event_data(&remove, "::RemoveLiquidityEvent")?;
    assert_eq!(
        remove_data.len(),
        56,
        "RemoveLiquidityEvent is addr + 3 u64"
    );
    assert_eq!(event_u64(remove_data, 32), initial_shares);
    assert_eq!(event_u64(remove_data, 40), usd_after);
    assert_eq!(event_u64(remove_data, 48), kan_after);

    // Final fresh instance observes the drained pool.
    let final_reader = MoveRuntime::new_with_kanari_natives_and_store(store)?;
    assert_eq!(reserves_view(&final_reader, &pool_id)?, json!([0, 0]));
    assert_eq!(lp_view(&final_reader, &pool_id)?, JsonValue::from(0u64));

    Ok(())
}

// ---------------------------------------------------------------------------
// Runtime call helpers
// ---------------------------------------------------------------------------

/// Monotonic transaction nonce: object ids derive from `hash(tx_hash || n)`,
/// so every transaction that creates objects needs its own `tx_hash`.
#[derive(Default)]
struct Nonce(u64);

impl Nonce {
    fn next_bytes(&mut self) -> Vec<u8> {
        self.0 += 1;
        self.0.to_le_bytes().to_vec()
    }
}

struct Created {
    id: String,
    owner: AccountAddress,
    data: Vec<u8>,
}

/// Execute an entry function, self-persisting on success (`persist_runtime_state`).
fn entry(
    runtime: &MoveRuntime,
    module_id: &ModuleId,
    function: &str,
    type_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
    object_inputs: Vec<ObjectInput>,
    nonce: &mut Nonce,
) -> Result<ChangeSet> {
    runtime
        .execute_entry_function_with_object_context_and_persistence(
            module_id,
            function,
            type_args,
            args,
            EntryFunctionObjectContext {
                object_inputs,
                sender: Some(*module_id.address()),
                gas_info: None,
                timestamp: None,
                tx_hash: Some(nonce.next_bytes()),
                persist_runtime_state: true,
                state_overlay: None,
            },
        )
        .with_context(|| format!("entry {function}"))
}

/// Mint `amount` of `token` from the test faucet and return the new coin id.
fn mint(
    runtime: &MoveRuntime,
    module_id: &ModuleId,
    token: &str,
    amount: u64,
    nonce: &mut Nonce,
) -> Result<String> {
    let changes = entry(
        runtime,
        module_id,
        "mint_test_coin",
        type_tags(&[token])?,
        vec![bcs::to_bytes(&amount)?],
        vec![],
        nonce,
    )?;
    let coin = find_created(&changes, |t| t.contains("coin::Coin") && t.contains(token))?;
    assert_eq!(
        coin_balance(&coin.data),
        amount,
        "faucet must mint the requested amount"
    );
    assert_eq!(
        coin.owner,
        *module_id.address(),
        "faucet must pay the sender"
    );
    Ok(coin.id)
}

fn reserves_view(runtime: &MoveRuntime, pool_id: &str) -> Result<JsonValue> {
    view(
        runtime,
        "get_reserves",
        &[USD_TYPE, KAN_TYPE],
        &[object_arg(pool_id)?],
        &[immutable_pool_input(pool_id)?],
    )
}

fn lp_view(runtime: &MoveRuntime, pool_id: &str) -> Result<JsonValue> {
    view(
        runtime,
        "lp_total",
        &[USD_TYPE, KAN_TYPE],
        &[object_arg(pool_id)?],
        &[immutable_pool_input(pool_id)?],
    )
}

fn quote_view(runtime: &MoveRuntime, pool_id: &str, amount_in: u64) -> Result<u64> {
    let value = view(
        runtime,
        "quote_swap",
        &[USD_TYPE, KAN_TYPE],
        &[object_arg(pool_id)?, bcs::to_bytes(&amount_in)?],
        &[immutable_pool_input(pool_id)?],
    )?;
    value.as_u64().context("quote_swap must return a number")
}

fn view(
    runtime: &MoveRuntime,
    function: &str,
    type_args: &[&str],
    args: &[Vec<u8>],
    object_inputs: &[ObjectInput],
) -> Result<JsonValue> {
    let type_args: Vec<String> = type_args.iter().map(|t| (*t).to_string()).collect();
    runtime.execute_view_function(
        TESTER_ADDR,
        MODULE_NAME,
        function,
        &type_args,
        args,
        object_inputs,
    )
}

// ---------------------------------------------------------------------------
// ChangeSet / argument helpers
// ---------------------------------------------------------------------------

fn find_created(cs: &ChangeSet, pred: impl Fn(&str) -> bool) -> Result<Created> {
    cs.created_objects
        .iter()
        .find(|(_, created)| pred(&created.type_))
        .map(|(id, created)| Created {
            id: id.clone(),
            owner: created.owner,
            data: created.data.clone(),
        })
        .context("expected created object not found")
}

fn find_event_data<'a>(cs: &'a ChangeSet, type_suffix: &str) -> Result<&'a [u8]> {
    cs.events
        .iter()
        .find(|event| event.type_tag.ends_with(type_suffix))
        .map(|event| event.event_data.as_slice())
        .with_context(|| format!("event {type_suffix} must be emitted"))
}

fn event_u64(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        data[offset..offset + 8]
            .try_into()
            .expect("u64 event field in range"),
    )
}

fn event_address(data: &[u8]) -> Vec<u8> {
    data[..32].to_vec()
}

fn coin_balance(data: &[u8]) -> u64 {
    assert!(data.len() >= 40, "coin data too short: {}", data.len());
    u64::from_le_bytes(data[32..40].try_into().expect("u64 coin balance in range"))
}

fn type_tags(tokens: &[&str]) -> Result<Vec<TypeTag>> {
    tokens
        .iter()
        .map(|token| Ok::<_, anyhow::Error>(TypeTag::Struct(Box::new(StructTag::from_str(token)?))))
        .collect()
}

fn object_arg(object_id: &str) -> Result<Vec<u8>> {
    let clean = object_id.strip_prefix("0x").unwrap_or(object_id);
    hex::decode(clean).context("decode object id argument")
}

fn owned_input(id: &str, owner: AccountAddress, mutable: bool) -> ObjectInput {
    ObjectInput {
        object_ref: ObjectRef::new(id.to_string(), None, None),
        owner: Some(ObjectOwnerKind::AddressOwner(owner.to_hex_literal())),
        mutable,
    }
}

fn immutable_pool_input(pool_id: &str) -> Result<ObjectInput> {
    Ok(owned_input(
        pool_id,
        AccountAddress::from_hex_literal(TESTER_ADDR)?,
        false,
    ))
}

// ---------------------------------------------------------------------------
// Expected-value math (mirrors the Move formulas exactly)
// ---------------------------------------------------------------------------

/// `floor(sqrt(USD_INITIAL * KAN_INITIAL))` via integer Newton iteration.
fn initial_lp_shares() -> u64 {
    let n = USD_INITIAL as u128 * KAN_INITIAL as u128;
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x as u64
}

/// Mirror of `bps_mul_floor_u128` + `quote_amount_out`.
fn expected_swap_out(amount_in: u64, reserve_in: u64, reserve_out: u64) -> u64 {
    let fee_in = amount_in as u128 * FEE_BPS / 10_000;
    (fee_in * reserve_out as u128 / (reserve_in as u128 + fee_in)) as u64
}

// ---------------------------------------------------------------------------
// Package build helpers (same temp-dir flow as the dynamic-field harness)
// ---------------------------------------------------------------------------

fn create_test_package() -> Result<PathBuf> {
    let dir = tempdir()?;
    let package_dir = dir.keep();
    fs::create_dir_all(package_dir.join("sources"))?;

    let dependency_path = move_manifest::kanari_system_package_path()?;
    let manifest = format!(
        "[package]\nname = \"UniswapLikeAmm\"\n\n[dependencies]\nKanariSystem = {{ local = \"{}\" }}\n\n[addresses]\ntester = \"{}\"\n",
        dependency_path, TESTER_ADDR,
    );
    fs::write(package_dir.join("Move.toml"), manifest)?;
    fs::write(
        package_dir.join("sources").join("uniswap_amm.move"),
        MOVE_SOURCE,
    )?;

    Ok(package_dir)
}

fn compile_test_module(package_dir: &Path) -> Result<(ModuleId, Vec<u8>)> {
    let package = BuildConfig::default().compile_package(package_dir, &mut Vec::new())?;
    let unit = package
        .root_modules()
        .next()
        .context("compiled package should contain a root module")?;
    let module = &unit.unit;
    let module_id = ModuleId::new(
        AccountAddress::new(module.address.into_bytes()),
        Identifier::new(module.name.to_string())?,
    );
    Ok((module_id, module.serialize(None)))
}
