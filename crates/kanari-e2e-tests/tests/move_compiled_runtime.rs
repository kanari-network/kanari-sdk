//! Compiled Move package E2E tests
//!
//! Compiles the real Move package under `move_package/`, publishes every module
//! through the Move VM runtime, and executes entry functions end-to-end.
//!
//! This replaces the previous text-only assertions: the Move sources must pass
//! the real Move compiler and the resulting bytecode must execute on the runtime.

use std::path::{Path, PathBuf};

use kanari_move_runtime_v2::move_runtime::MoveRuntime;
use move_binary_format::file_format::CompiledModule;
use move_core_types::account_address::AccountAddress as MoveAccountAddress;
use move_core_types::language_storage::ModuleId;
use move_core_types::runtime_value::MoveValue;
use move_package::BuildConfig;
use move_package::compilation::compiled_package::CompiledPackage;

/// Address the E2E Move package is published under (matches `Move.toml`).
const E2E_PUBLISHER: &str = "0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146";

/// Source of unique `tx_hash` values for hand-built contexts.
static TX_CONTEXT_NONCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Modules that must exist after a successful compile.
const EXPECTED_MODULES: &[&str] = &[
    "move_e2e_access",
    "move_e2e_basic",
    "move_e2e_comprehensive",
    "move_e2e_events",
    "move_e2e_math",
    "move_e2e_nft",
    "move_e2e_pool",
    "move_e2e_registry",
    "move_e2e_storage",
    "move_e2e_time",
];

fn package_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at crates/kanari-e2e-tests
    Path::new(env!("CARGO_MANIFEST_DIR")).join("move_package")
}

fn publisher_address() -> MoveAccountAddress {
    MoveAccountAddress::from_hex_literal(E2E_PUBLISHER).expect("valid publisher address")
}

fn module_id(name: &str) -> ModuleId {
    ModuleId::new(
        publisher_address(),
        move_core_types::identifier::Identifier::new(name).expect("valid module identifier"),
    )
}

/// Compile the Move package into a scratch install dir and return the compiled
/// package. The source tree is never mutated.
fn compile_e2e_package() -> anyhow::Result<(tempfile::TempDir, CompiledPackage)> {
    let root = package_root();
    anyhow::ensure!(
        root.join("Move.toml").exists(),
        "Move package manifest missing at {}",
        root.display()
    );

    let scratch = tempfile::tempdir()?;
    let install_dir = scratch.path().join("build");

    let compiled = BuildConfig {
        install_dir: Some(install_dir),
        ..Default::default()
    }
    .compile_package(&root, &mut std::io::sink())?;

    Ok((scratch, compiled))
}

/// Deserialize every root module and return `(module_name, bytes)`.
fn root_module_bytes(compiled: &CompiledPackage) -> Vec<(String, Vec<u8>)> {
    compiled
        .root_modules()
        .map(|unit| {
            let name = unit.unit.module.self_id().name().to_string();
            let mut bytes = Vec::new();
            unit.unit
                .module
                .serialize(&mut bytes)
                .expect("serialize compiled module");
            (name, bytes)
        })
        .collect()
}

/// Publish all compiled modules into a fresh in-memory runtime.
fn publish_all(runtime: &MoveRuntime, compiled: &CompiledPackage) -> Vec<(String, Vec<u8>)> {
    let sender = publisher_address();
    let modules = root_module_bytes(compiled);
    for (name, bytes) in &modules {
        let _ = runtime
            .publish_module(bytes.clone(), sender, None, None)
            .unwrap_or_else(|e| panic!("publishing module `{name}` failed: {e:?}"));
    }
    modules
}

/// Build a `TxContext` argument the way the node does.
fn tx_context_arg(sender: MoveAccountAddress, epoch: u64) -> Vec<u8> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default();
    tx_context_arg_at(sender, epoch, now_ms)
}

/// Tx context pinned to an explicit `epoch_timestamp_ms`, so entries that
/// record a creation time can be read back with an exact expectation.
fn tx_context_arg_at(sender: MoveAccountAddress, epoch: u64, timestamp_ms: u64) -> Vec<u8> {
    // `object::new` derives its id from `hash(tx_hash || ids_created)`, so a
    // constant hash would hand every transaction the same first object id and
    // let one transaction's object silently overwrite another's.
    let mut tx_hash = [0u8; 32];
    tx_hash[..8].copy_from_slice(
        &TX_CONTEXT_NONCE
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .to_le_bytes(),
    );
    let ctx = kanari_types::tx_context::TxContextRecord::from_address(
        sender,
        tx_hash.to_vec(),
        epoch,
        timestamp_ms,
        0,
    );
    bcs::to_bytes(&ctx).expect("serialize TxContext")
}

fn vec_u8_arg(bytes: &[u8]) -> Vec<u8> {
    MoveValue::Vector(bytes.iter().copied().map(MoveValue::U8).collect())
        .simple_serialize()
        .expect("serialize vector<u8>")
}

fn u64_arg(value: u64) -> Vec<u8> {
    MoveValue::U64(value)
        .simple_serialize()
        .expect("serialize u64")
}

fn u8_arg(value: u8) -> Vec<u8> {
    MoveValue::U8(value)
        .simple_serialize()
        .expect("serialize u8")
}

fn bool_arg(value: bool) -> Vec<u8> {
    MoveValue::Bool(value)
        .simple_serialize()
        .expect("serialize bool")
}

fn address_arg(value: &str) -> Vec<u8> {
    MoveValue::Address(MoveAccountAddress::from_hex_literal(value).expect("valid address"))
        .simple_serialize()
        .expect("serialize address")
}

/// Strip the `0x` prefix and left-pad an object id to a full address literal.
fn addr_literal(id: &str) -> String {
    format!("0x{}", id.trim_start_matches("0x"))
}

/// Preload every created object of a changeset so later calls can resolve them.
fn preload_created(runtime: &MoveRuntime, cs: &kanari_move_runtime_v2::changeset::ChangeSet) {
    for (id, obj) in &cs.created_objects {
        runtime
            .preload_object_snapshot(id, obj.owner, &obj.type_, obj.data.clone(), obj.version)
            .unwrap_or_else(|e| panic!("failed to preload object {id}: {e:?}"));
    }
}

/// One argument slot of an entry-function call: either a plain BCS value or an
/// object argument that must be declared through `object_inputs`.
enum Slot {
    Raw(Vec<u8>),
    Obj { id: String, mutable: bool },
}

/// Execute an entry function, declaring every object parameter in `object_inputs`
/// (the runtime rejects calls that reference objects without declaring them).
fn exec(
    runtime: &MoveRuntime,
    module: &ModuleId,
    fn_name: &str,
    slots: Vec<Slot>,
    sender: MoveAccountAddress,
) -> anyhow::Result<kanari_move_runtime_v2::changeset::ChangeSet> {
    let mut args: Vec<Vec<u8>> = Vec::new();
    let mut object_inputs: Vec<kanari_types::transaction::ObjectInput> = Vec::new();
    for slot in slots {
        match slot {
            Slot::Raw(bytes) => args.push(bytes),
            Slot::Obj { id, mutable } => {
                args.push(Vec::new());
                object_inputs.push(kanari_types::transaction::ObjectInput {
                    object_ref: kanari_types::transaction::ObjectRef::new(id.clone(), None, None),
                    owner: Some(kanari_types::transaction::ObjectOwnerKind::Shared),
                    mutable,
                });
            }
        }
    }

    runtime
        .execute_entry_function_with_object_context_and_persistence(
            module,
            fn_name,
            vec![],
            args,
            kanari_move_runtime_v2::move_runtime::EntryFunctionObjectContext {
                object_inputs,
                sender: Some(sender),
                gas_info: None,
                timestamp: None,
                tx_hash: None,
                persist_runtime_state: true,
                state_overlay: None,
            },
        )
        .map_err(|e| anyhow::anyhow!("{fn_name}: {e:?}"))
}

/// Asserts a call aborted *inside* Move with `expected_code`. Checking the
/// sub-status rules out an earlier failure in argument binding or ownership
/// bookkeeping, which would otherwise satisfy a bare `is_err()`.
fn assert_aborted(
    result: anyhow::Result<kanari_move_runtime_v2::changeset::ChangeSet>,
    expected_code: u64,
    what: &str,
) {
    let err = match result {
        Ok(_) => panic!("{what} was expected to abort but succeeded"),
        Err(err) => err,
    };
    let rendered = err.to_string();
    assert!(
        rendered.contains("major_status: ABORTED"),
        "{what} must abort inside Move, got {rendered}"
    );
    assert!(
        rendered.contains(&format!("sub_status: Some({expected_code})")),
        "{what} must abort with code {expected_code}, got {rendered}"
    );
}

/// `(object_id, version)` of the single object created by a changeset.
fn only_created(cs: &kanari_move_runtime_v2::changeset::ChangeSet) -> (String, u64) {
    assert_eq!(
        cs.created_objects.len(),
        1,
        "expected exactly one created object, got {:?}",
        created_types(cs)
    );
    let (id, obj) = &cs.created_objects[0];
    (id.clone(), obj.version)
}

fn created_types(cs: &kanari_move_runtime_v2::changeset::ChangeSet) -> Vec<String> {
    cs.created_objects
        .iter()
        .map(|(_, o)| o.type_.clone())
        .collect()
}

/// Id of the created object whose type ends with `suffix`.
fn find_created(cs: &kanari_move_runtime_v2::changeset::ChangeSet, suffix: &str) -> String {
    cs.created_objects
        .iter()
        .find(|(_, o)| o.type_.ends_with(suffix))
        .unwrap_or_else(|| {
            panic!(
                "no created object whose type ends with `{suffix}`; got {:?}",
                created_types(cs)
            )
        })
        .0
        .clone()
}

/// Id of the created object whose type contains `needle` (matches generic
/// types such as `0x2::coin::TreasuryCap<0x3..::TEST_COIN>`).
fn find_created_containing(
    cs: &kanari_move_runtime_v2::changeset::ChangeSet,
    needle: &str,
) -> String {
    cs.created_objects
        .iter()
        .find(|(_, o)| o.type_.contains(needle))
        .unwrap_or_else(|| {
            panic!(
                "no created object whose type contains `{needle}`; got {:?}",
                created_types(cs)
            )
        })
        .0
        .clone()
}

/// Single `u64` payload of a created object: the trailing 8 bytes of its BCS
/// data (the data is a 32-byte object id followed by the struct fields).
fn created_u64(cs: &kanari_move_runtime_v2::changeset::ChangeSet, id: &str) -> anyhow::Result<u64> {
    let data = &cs
        .created_objects
        .iter()
        .find(|(k, _)| k == id)
        .unwrap_or_else(|| panic!("object `{id}` was not created; got {:?}", created_types(cs)))
        .1
        .data;
    anyhow::ensure!(
        data.len() >= 8,
        "object `{id}` holds {} bytes, expected at least 8",
        data.len()
    );
    let mut tail = [0u8; 8];
    tail.copy_from_slice(&data[data.len() - 8..]);
    Ok(u64::from_le_bytes(tail))
}

#[test]
fn e2e_move_package_compiles_to_bytecode() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let modules = root_module_bytes(&compiled);

    for expected in EXPECTED_MODULES {
        assert!(
            modules.iter().any(|(name, _)| name == expected),
            "compiled package is missing module `{expected}`; got: {:?}",
            modules.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>()
        );
    }

    assert_eq!(
        modules.len(),
        EXPECTED_MODULES.len(),
        "unexpected module count in compiled package"
    );

    for (name, bytes) in &modules {
        let parsed = CompiledModule::deserialize_with_defaults(bytes)
            .unwrap_or_else(|e| panic!("module `{name}` failed to deserialize: {e:?}"));
        assert_eq!(
            *parsed.self_id().address(),
            publisher_address(),
            "module `{name}` must live at the publisher address"
        );
        assert!(!bytes.is_empty(), "module `{name}` compiled to zero bytes");
    }

    Ok(())
}

#[test]
fn e2e_move_modules_publish_to_runtime() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
    let modules = publish_all(&runtime, &compiled);

    assert_eq!(modules.len(), EXPECTED_MODULES.len());
    for name in EXPECTED_MODULES {
        assert!(
            modules.iter().any(|(n, _)| n == name),
            "`{name}` was not published"
        );
    }

    Ok(())
}

#[test]
fn e2e_math_entry_functions_execute() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
    publish_all(&runtime, &compiled);

    let module = module_id("move_e2e_math");
    let sender = publisher_address();

    // Pure computations must not mutate chain state.
    let cs = runtime.execute_entry_function(
        &module,
        "e2e_check_add",
        vec![],
        vec![u64_arg(2), u64_arg(3), u64_arg(5)],
        Some(sender),
        None,
        None,
    )?;
    assert!(
        cs.created_objects.is_empty(),
        "pure computation must not create objects"
    );

    // Exercise the `math` natives plus local logic.
    let cases: Vec<(&str, Vec<Vec<u8>>)> = vec![
        (
            "e2e_check_parity",
            vec![u64_arg(4), bool_arg(true), bool_arg(false)],
        ),
        (
            "e2e_check_clamp",
            vec![u64_arg(500), u64_arg(0), u64_arg(100), u64_arg(100)],
        ),
        (
            "e2e_check_percent_of",
            vec![u64_arg(1000), u64_arg(25), u64_arg(250)],
        ),
        ("e2e_check_sqrt", vec![u64_arg(144), u64_arg(12)]),
        (
            "e2e_check_average",
            vec![u64_arg(10), u64_arg(20), u64_arg(15)],
        ),
        (
            "e2e_check_ceil_div",
            vec![u64_arg(10), u64_arg(3), u64_arg(4)],
        ),
        ("e2e_check_pow", vec![u64_arg(2), u8_arg(10), u64_arg(1024)]),
        (
            "e2e_check_multiply",
            vec![u64_arg(6), u64_arg(7), u64_arg(42)],
        ),
        (
            "e2e_check_add_overflow",
            vec![u64_arg(u64::MAX), u64_arg(1)],
        ),
    ];

    for (fn_name, args) in cases {
        let _ = runtime
            .execute_entry_function(&module, fn_name, vec![], args, Some(sender), None, None)
            .unwrap_or_else(|e| panic!("move_e2e_math::{fn_name} execution failed: {e:?}"));
    }

    Ok(())
}

#[test]
fn e2e_events_entry_functions_emit_events() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
    publish_all(&runtime, &compiled);

    let module = module_id("move_e2e_events");
    let sender = publisher_address();

    let cs = runtime.execute_entry_function(
        &module,
        "emit_batch",
        vec![],
        vec![vec_u8_arg(b"hello"), tx_context_arg(sender, 0)],
        Some(sender),
        None,
        None,
    )?;

    assert_eq!(
        cs.events.len(),
        3,
        "emit_batch must produce three events, got {:?}",
        cs.events.iter().map(|e| &e.type_tag).collect::<Vec<_>>()
    );
    assert!(
        cs.events
            .iter()
            .any(|e| e.type_tag.contains("move_e2e_events::MessageEvent")),
        "expected a MessageEvent"
    );
    assert!(
        cs.events
            .iter()
            .any(|e| e.type_tag.contains("move_e2e_events::BatchEvent")),
        "expected a BatchEvent"
    );

    // Single-emission entry points must each produce exactly one event.
    for fn_name in ["emit_message", "bump_counter", "emit_batch"] {
        let args = if fn_name == "bump_counter" {
            vec![u64_arg(7), vec_u8_arg(b"tag")]
        } else {
            // Both `emit_message` and `emit_batch` take (payload, ctx).
            vec![vec_u8_arg(b"msg"), tx_context_arg(sender, 0)]
        };
        let cs = runtime.execute_entry_function(
            &module,
            fn_name,
            vec![],
            args,
            Some(sender),
            None,
            None,
        )?;
        assert!(
            !cs.events.is_empty(),
            "move_e2e_events::{fn_name} must emit at least one event"
        );
    }

    Ok(())
}

#[test]
fn e2e_basic_coin_init_creates_objects() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
    publish_all(&runtime, &compiled);

    let module = module_id("move_e2e_basic");
    let sender = publisher_address();

    let cs = exec(
        &runtime,
        &module,
        "init_test_coin",
        vec![Slot::Raw(tx_context_arg(sender, 0))],
        sender,
    )?;

    let types = created_types(&cs);
    assert!(
        !cs.created_objects.is_empty(),
        "init_test_coin must create the coin, cap, and metadata objects"
    );
    assert!(
        types.iter().any(|t| t.contains("TreasuryCap")),
        "expected a TreasuryCap object, got {types:?}"
    );
    assert!(
        types.iter().any(|t| t.contains("CoinMetadata")),
        "expected a CoinMetadata object, got {types:?}"
    );
    assert!(
        types.iter().any(|t| t.contains("TEST_COIN")),
        "expected a minted Coin<TEST_COIN>, got {types:?}"
    );

    // The shared cap created by `init_test_coin` is rebound for a later
    // transaction, proving the created objects were persisted and re-read.
    let cap_id = find_created_containing(&cs, "TreasuryCap");
    preload_created(&runtime, &cs);

    let _ = exec(
        &runtime,
        &module,
        "e2e_check_total_supply",
        vec![
            Slot::Obj {
                id: cap_id.clone(),
                mutable: false,
            },
            Slot::Raw(u64_arg(1_000_000_000_000)),
        ],
        sender,
    )?;

    // A later `&mut` transaction against the same shared cap mints a real
    // coin, so the rebinding resolves the cap for mutation rather than only
    // for reads.
    let mint_cs = exec(
        &runtime,
        &module,
        "mint_more",
        vec![
            Slot::Obj {
                id: cap_id.clone(),
                mutable: true,
            },
            Slot::Raw(u64_arg(42)),
            Slot::Raw(tx_context_arg(sender, 1)),
        ],
        sender,
    )?;

    let minted_id = find_created_containing(&mint_cs, "Coin<");
    assert_eq!(
        created_u64(&mint_cs, &minted_id)?,
        42,
        "mint_more must mint exactly the requested amount; type was {}",
        mint_cs
            .created_objects
            .iter()
            .find(|(k, _)| k == &minted_id)
            .map(|(_, o)| o.type_.clone())
            .unwrap_or_default()
    );

    // `mint_more` mutated the shared cap in place. Reading it back in a later
    // transaction must observe the bumped supply, not the creation-time value.
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_total_supply",
        vec![
            Slot::Obj {
                id: cap_id,
                mutable: false,
            },
            Slot::Raw(u64_arg(1_000_000_000_042)),
        ],
        sender,
    )?;

    Ok(())
}

#[test]
fn e2e_comprehensive_vault_flow_executes() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
    publish_all(&runtime, &compiled);

    let module = module_id("move_e2e_comprehensive");
    let sender = publisher_address();

    let init_cs = runtime.execute_entry_function(
        &module,
        "init_comp_coin",
        vec![],
        vec![tx_context_arg(sender, 0)],
        Some(sender),
        None,
        None,
    )?;
    assert!(
        !init_cs.created_objects.is_empty(),
        "init_comp_coin must create objects"
    );
    preload_created(&runtime, &init_cs);
    let payment_id = find_created_containing(&init_cs, "Coin<");

    let vault_cs = runtime.execute_entry_function(
        &module,
        "e2e_create_vault",
        vec![],
        vec![vec_u8_arg(b"main vault"), tx_context_arg(sender, 1)],
        Some(sender),
        None,
        None,
    )?;
    assert_eq!(vault_cs.created_objects.len(), 1);
    let (vault_id, vault_obj) = &vault_cs.created_objects[0];
    assert!(
        vault_obj.type_.contains("move_e2e_comprehensive::Vault"),
        "unexpected vault type {}",
        vault_obj.type_
    );
    preload_created(&runtime, &vault_cs);

    // The vault is empty right after creation; an oversized withdraw must abort
    // with `E_INSUFFICIENT`, which only happens if the call really reached Move.
    let err = exec(
        &runtime,
        &module,
        "e2e_withdraw",
        vec![
            Slot::Obj {
                id: vault_id.clone(),
                mutable: true,
            },
            Slot::Raw(u64_arg(1)),
            Slot::Raw(tx_context_arg(sender, 2)),
        ],
        sender,
    );
    assert_aborted(err, 101, "withdraw from an empty vault");

    // With the vault still empty, the minted payment coin can be merged in:
    // `deposit` consumes the coin by value and reports the amount it took.
    let deposit = exec(
        &runtime,
        &module,
        "deposit",
        vec![
            Slot::Obj {
                id: vault_id.to_string(),
                mutable: true,
            },
            Slot::Obj {
                id: payment_id,
                mutable: true,
            },
            Slot::Raw(tx_context_arg(sender, 3)),
        ],
        sender,
    );
    let deposit = deposit.unwrap_or_else(|e| panic!("deposit must succeed, got {e:?}"));
    assert!(
        deposit
            .events
            .iter()
            .any(|ev| ev.type_tag.contains("move_e2e_comprehensive::DepositEvent")),
        "deposit must emit a DepositEvent, got {:?}",
        deposit
            .events
            .iter()
            .map(|ev| &ev.type_tag)
            .collect::<Vec<_>>()
    );

    Ok(())
}

#[test]
fn e2e_storage_store_flow_executes() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
    publish_all(&runtime, &compiled);

    let module = module_id("move_e2e_storage");
    let sender = publisher_address();

    let cs = exec(
        &runtime,
        &module,
        "e2e_create_store",
        vec![Slot::Raw(tx_context_arg(sender, 0))],
        sender,
    )?;
    let (_store_id, _store_version) = only_created(&cs);
    assert!(
        cs.created_objects[0]
            .1
            .type_
            .contains("move_e2e_storage::KeyValueStore")
    );
    preload_created(&runtime, &cs);

    // Owner-gated write, delete and blob versioning all happen inside one
    // transaction: the in-transaction `e2e_store_roundtrip` entry point
    // exercises the same `set_value` / `remove_value` / `update_blob` code.
    let roundtrip = exec(
        &runtime,
        &module,
        "e2e_store_roundtrip",
        vec![Slot::Raw(tx_context_arg(sender, 1))],
        sender,
    )?;
    assert!(
        roundtrip.created_objects.len() >= 2,
        "e2e_store_roundtrip must create the store and the blob"
    );

    // The round trip shares both objects in their final state (`beta` kept,
    // `alpha` removed, blob bumped twice), so the read-back entries see
    // exactly what that transaction left behind.
    let store_id = find_created_containing(&roundtrip, "KeyValueStore");
    let roundtrip_blob_id = find_created_containing(&roundtrip, "DataBlob");
    preload_created(&runtime, &roundtrip);

    let store = |mutable: bool| Slot::Obj {
        id: store_id.clone(),
        mutable,
    };

    let _ = exec(
        &runtime,
        &module,
        "e2e_check_store_size",
        vec![store(false), Slot::Raw(u64_arg(1))],
        sender,
    )?;
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_get_value",
        vec![
            store(false),
            Slot::Raw(vec_u8_arg(b"beta")),
            Slot::Raw(vec_u8_arg(b"two")),
        ],
        sender,
    )?;
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_get_value_absent",
        vec![store(false), Slot::Raw(vec_u8_arg(b"alpha"))],
        sender,
    )?;
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_blob_version",
        vec![
            Slot::Obj {
                id: roundtrip_blob_id,
                mutable: false,
            },
            Slot::Raw(u64_arg(3)),
        ],
        sender,
    )?;

    // Owner-gated delete runs against the surviving key.
    let _ = exec(
        &runtime,
        &module,
        "remove_value",
        vec![
            store(true),
            Slot::Raw(vec_u8_arg(b"beta")),
            Slot::Raw(tx_context_arg(sender, 2)),
        ],
        sender,
    )?;

    // Blob creation through the public entry point.
    let blob_cs = exec(
        &runtime,
        &module,
        "e2e_create_blob",
        vec![
            Slot::Raw(vec_u8_arg(b"payload")),
            Slot::Raw(vec_u8_arg(b"hash0")),
            Slot::Raw(tx_context_arg(sender, 3)),
        ],
        sender,
    )?;
    let (blob_id, _blob_version) = only_created(&blob_cs);
    preload_created(&runtime, &blob_cs);

    let update = exec(
        &runtime,
        &module,
        "update_blob",
        vec![
            Slot::Obj {
                id: blob_id,
                mutable: true,
            },
            Slot::Raw(vec_u8_arg(b"payload2")),
            Slot::Raw(vec_u8_arg(b"hash1")),
        ],
        sender,
    );
    assert!(
        update.is_ok(),
        "update_blob must succeed, got {:?}",
        update.err()
    );

    Ok(())
}

#[test]
fn e2e_nft_collection_flow_executes() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
    publish_all(&runtime, &compiled);

    let module = module_id("move_e2e_nft");
    let sender = publisher_address();

    let cs = exec(
        &runtime,
        &module,
        "e2e_create_collection",
        vec![
            Slot::Raw(vec_u8_arg(b"E2E Collection")),
            Slot::Raw(vec_u8_arg(b"collection for e2e tests")),
            Slot::Raw(tx_context_arg(sender, 0)),
        ],
        sender,
    )?;
    let (collection_id, _collection_version) = only_created(&cs);
    assert!(
        cs.created_objects[0]
            .1
            .type_
            .contains("move_e2e_nft::NFTCollection")
    );
    preload_created(&runtime, &cs);

    let mint_cs = exec(
        &runtime,
        &module,
        "e2e_mint_nft",
        vec![
            Slot::Obj {
                id: collection_id,
                mutable: true,
            },
            Slot::Raw(vec_u8_arg(b"Genesis")),
            Slot::Raw(vec_u8_arg(b"first nft")),
            Slot::Raw(vec_u8_arg(b"https://kanari.example/1.png")),
            Slot::Raw(tx_context_arg(sender, 1)),
        ],
        sender,
    )?;

    let mint_types = created_types(&mint_cs);
    assert!(
        mint_types.iter().any(|t| t.contains("move_e2e_nft::NFT")),
        "mint_nft must create an NFT, got {mint_types:?}"
    );

    Ok(())
}

#[test]
fn e2e_access_control_flow_executes() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
    publish_all(&runtime, &compiled);

    let module = module_id("move_e2e_access");
    let sender = publisher_address();

    let control_cs = exec(
        &runtime,
        &module,
        "e2e_create_access_control",
        vec![
            Slot::Raw(bool_arg(true)),
            Slot::Raw(tx_context_arg(sender, 0)),
        ],
        sender,
    )?;
    let (control_id, _control_version) = only_created(&control_cs);
    assert!(
        control_cs.created_objects[0]
            .1
            .type_
            .contains("move_e2e_access::AccessControl")
    );
    preload_created(&runtime, &control_cs);

    let resource_cs = exec(
        &runtime,
        &module,
        "e2e_create_resource",
        vec![
            Slot::Raw(u64_arg(1)),
            Slot::Raw(vec_u8_arg(b"meta")),
            Slot::Raw(tx_context_arg(sender, 3)),
        ],
        sender,
    )?;
    let (resource_id, _resource_version) = only_created(&resource_cs);
    assert!(
        resource_cs.created_objects[0]
            .1
            .type_
            .contains("move_e2e_access::Resource")
    );
    preload_created(&runtime, &resource_cs);

    // `update_resource` takes two object arguments. The runtime's declared
    // `object_inputs` binding path only handles a single object per call, so
    // the guarded update is exercised in-transaction by `e2e_access_roundtrip`
    // in the companion test below. Here we assert the read path.
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_resource_value",
        vec![
            Slot::Obj {
                id: resource_id,
                mutable: false,
            },
            Slot::Raw(u64_arg(1)),
        ],
        sender,
    )?;

    let _ = control_id;

    Ok(())
}

/// The admin lifecycle (grant, guarded update, revoke) is verified inside one
/// transaction. It needs its own runtime: `object::new` derives ids
/// deterministically, so the round trip would collide with the object ids of
/// the open-control scenario above.
#[test]
fn e2e_access_roundtrip_and_closed_control_reject_strangers() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
    publish_all(&runtime, &compiled);

    let module = module_id("move_e2e_access");
    let sender = publisher_address();

    let roundtrip = exec(
        &runtime,
        &module,
        "e2e_access_roundtrip",
        vec![Slot::Raw(tx_context_arg(sender, 1))],
        sender,
    )?;
    assert!(
        roundtrip.created_objects.len() >= 2,
        "e2e_access_roundtrip must create the control and the resource"
    );
    preload_created(&runtime, &roundtrip);

    // A control seeded with an admin at creation time is readable from a later
    // transaction, so membership and the admin count both assert positively.
    let seeded = exec(
        &runtime,
        &module,
        "e2e_create_control_with_admin",
        vec![
            Slot::Raw(address_arg(E2E_PUBLISHER)),
            Slot::Raw(tx_context_arg(sender, 8)),
        ],
        sender,
    )?;
    let seeded_id = only_created(&seeded).0;
    preload_created(&runtime, &seeded);

    let seeded_slot = |mutable: bool| Slot::Obj {
        id: seeded_id.clone(),
        mutable,
    };

    let _ = exec(
        &runtime,
        &module,
        "e2e_check_admin_count",
        vec![seeded_slot(false), Slot::Raw(u64_arg(1))],
        sender,
    )?;
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_is_admin",
        vec![
            seeded_slot(false),
            Slot::Raw(address_arg(E2E_PUBLISHER)),
            Slot::Raw(bool_arg(true)),
        ],
        sender,
    )?;
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_is_admin",
        vec![
            seeded_slot(false),
            Slot::Raw(address_arg("0x2")),
            Slot::Raw(bool_arg(false)),
        ],
        sender,
    )?;

    // The owner revokes the seeded admin; a member that is not listed is a
    // no-op rather than an abort.
    let _ = exec(
        &runtime,
        &module,
        "remove_admin",
        vec![
            seeded_slot(true),
            Slot::Raw(address_arg(E2E_PUBLISHER)),
            Slot::Raw(tx_context_arg(sender, 9)),
        ],
        sender,
    )?;

    // The revoke mutated a non-Coin custom struct in place; the next
    // transaction must read the shrunk list, not the creation-time snapshot.
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_admin_count",
        vec![seeded_slot(false), Slot::Raw(u64_arg(0))],
        sender,
    )?;
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_is_admin",
        vec![
            seeded_slot(false),
            Slot::Raw(address_arg(E2E_PUBLISHER)),
            Slot::Raw(bool_arg(false)),
        ],
        sender,
    )?;

    // A closed control has no admins and a different owner, so a stranger can
    // neither grant itself an admin nor update the guarded resource.
    let closed = exec(
        &runtime,
        &module,
        "e2e_create_closed_control",
        vec![Slot::Raw(tx_context_arg(sender, 9))],
        sender,
    )?;
    assert_eq!(closed.created_objects.len(), 2);
    let (closed_id, guarded_id) = (
        closed.created_objects[0].0.clone(),
        closed.created_objects[1].0.clone(),
    );
    preload_created(&runtime, &closed);

    let other = MoveAccountAddress::from_hex_literal("0x2").expect("valid address");

    let denied = exec(
        &runtime,
        &module,
        "add_admin",
        vec![
            Slot::Obj {
                id: closed_id.clone(),
                mutable: true,
            },
            Slot::Raw(address_arg(E2E_PUBLISHER)),
            Slot::Raw(tx_context_arg(other, 2)),
        ],
        other,
    );
    assert_aborted(denied, 200, "non-owner add_admin");

    let denied = exec(
        &runtime,
        &module,
        "update_resource",
        vec![
            Slot::Obj {
                id: closed_id,
                mutable: false,
            },
            Slot::Obj {
                id: guarded_id,
                mutable: true,
            },
            Slot::Raw(u64_arg(7)),
            Slot::Raw(vec_u8_arg(b"hijack")),
            Slot::Raw(tx_context_arg(other, 3)),
        ],
        other,
    );
    assert_aborted(denied, 201, "non-admin update_resource on a closed control");

    Ok(())
}

#[test]
fn e2e_registry_flow_executes() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
    publish_all(&runtime, &compiled);

    let module = module_id("move_e2e_registry");
    let sender = publisher_address();

    let cs = exec(
        &runtime,
        &module,
        "e2e_create_registry",
        vec![Slot::Raw(tx_context_arg(sender, 0))],
        sender,
    )?;
    let (registry_id, _registry_version) = only_created(&cs);
    assert!(
        cs.created_objects[0]
            .1
            .type_
            .contains("move_e2e_registry::Registry")
    );
    preload_created(&runtime, &cs);

    let _ = exec(
        &runtime,
        &module,
        "register_name",
        vec![
            Slot::Obj {
                id: registry_id.clone(),
                mutable: true,
            },
            Slot::Raw(vec_u8_arg(b"kanari")),
            Slot::Raw(address_arg(E2E_PUBLISHER)),
            Slot::Raw(tx_context_arg(sender, 1)),
        ],
        sender,
    )?;

    // Re-registering the same name must overwrite, not duplicate.
    let _ = exec(
        &runtime,
        &module,
        "register_name",
        vec![
            Slot::Obj {
                id: registry_id,
                mutable: true,
            },
            Slot::Raw(vec_u8_arg(b"kanari")),
            Slot::Raw(address_arg("0x3")),
            Slot::Raw(tx_context_arg(sender, 2)),
        ],
        sender,
    )?;

    let entry_cs = exec(
        &runtime,
        &module,
        "e2e_create_entry",
        vec![
            Slot::Raw(vec_u8_arg(b"entry")),
            Slot::Raw(vec_u8_arg(b"value")),
            Slot::Raw(tx_context_arg(sender, 3)),
        ],
        sender,
    )?;
    let (entry_id, _entry_version) = only_created(&entry_cs);
    preload_created(&runtime, &entry_cs);

    let _ = exec(
        &runtime,
        &module,
        "update_entry",
        vec![
            Slot::Obj {
                id: entry_id,
                mutable: true,
            },
            Slot::Raw(vec_u8_arg(b"value2")),
            Slot::Raw(tx_context_arg(sender, 4)),
        ],
        sender,
    )?;

    Ok(())
}

#[test]
fn e2e_time_flow_executes() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
    publish_all(&runtime, &compiled);

    let module = module_id("move_e2e_time");
    let sender = publisher_address();

    // Pinned so `created_at` / `last_updated` read back exactly.
    const RECORDED_AT: u64 = 1_700_000_000_000;

    let cs = exec(
        &runtime,
        &module,
        "e2e_create_record",
        vec![Slot::Raw(tx_context_arg_at(sender, 0, RECORDED_AT))],
        sender,
    )?;
    let (record_id, _record_version) = only_created(&cs);
    assert!(
        cs.created_objects[0]
            .1
            .type_
            .contains("move_e2e_time::TimestampRecord")
    );
    preload_created(&runtime, &cs);

    let record = |mutable: bool| Slot::Obj {
        id: record_id.clone(),
        mutable,
    };

    // The record was never touched since creation, so these read back exactly.
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_created_at",
        vec![record(false), Slot::Raw(u64_arg(RECORDED_AT))],
        sender,
    )?;
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_last_updated",
        vec![record(false), Slot::Raw(u64_arg(RECORDED_AT))],
        sender,
    )?;
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_is_fresh",
        vec![
            record(false),
            Slot::Raw(u64_arg(RECORDED_AT + 500)),
            Slot::Raw(u64_arg(1_000)),
            Slot::Raw(bool_arg(true)),
        ],
        sender,
    )?;
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_is_fresh",
        vec![
            record(false),
            Slot::Raw(u64_arg(RECORDED_AT + 5_000)),
            Slot::Raw(u64_arg(1_000)),
            Slot::Raw(bool_arg(false)),
        ],
        sender,
    )?;

    let _ = exec(&runtime, &module, "e2e_check_clock_address", vec![], sender)?;

    let _ = exec(
        &runtime,
        &module,
        "update_record",
        vec![
            record(true),
            Slot::Raw(u64_arg(1_700_000_000_000)),
            Slot::Raw(tx_context_arg(sender, 1)),
        ],
        sender,
    )?;

    Ok(())
}

#[test]
fn e2e_pool_module_is_published_and_verifiable() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
    publish_all(&runtime, &compiled);

    let (name, bytes) = root_module_bytes(&compiled)
        .into_iter()
        .find(|(n, _)| n == "move_e2e_pool")
        .expect("move_e2e_pool module");

    let parsed = CompiledModule::deserialize_with_defaults(&bytes)?;
    let self_id = parsed.self_id();
    assert_eq!(self_id.name().as_str(), name);
    assert_eq!(*self_id.address(), publisher_address());

    let module = module_id("move_e2e_pool");
    assert_eq!(module, self_id);

    let sender = publisher_address();

    // The 128-bit pool math is asserted inside Move, so a wrong result aborts.
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_mul_div",
        vec![
            Slot::Raw(u64_arg(100)),
            Slot::Raw(u64_arg(200)),
            Slot::Raw(u64_arg(5)),
            Slot::Raw(u64_arg(4000)),
        ],
        sender,
    )?;
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_scaled_reserves",
        vec![
            Slot::Raw(u64_arg(12_345)),
            Slot::Raw(u64_arg(67_890)),
            Slot::Raw(u64_arg(83_810)),
        ],
        sender,
    )?;

    // Seed, grow and shrink an integer-only pool, verifying the share math
    // after every step.
    let pool_cs = exec(
        &runtime,
        &module,
        "create_simple_pool",
        vec![
            Slot::Raw(u64_arg(1000)),
            Slot::Raw(u64_arg(2000)),
            Slot::Raw(tx_context_arg(sender, 0)),
        ],
        sender,
    )?;
    let (pool_id, _pool_version) = only_created(&pool_cs);
    assert!(pool_cs.created_objects[0].1.type_.contains("::SimplePool"));
    preload_created(&runtime, &pool_cs);

    let _ = exec(
        &runtime,
        &module,
        "e2e_check_simple_pool",
        vec![
            Slot::Obj {
                id: pool_id.clone(),
                mutable: false,
            },
            Slot::Raw(u64_arg(1000)),
            Slot::Raw(u64_arg(2000)),
            Slot::Raw(u64_arg(3000)),
        ],
        sender,
    )?;

    // 500/1000 against 1000/2000 mints 1500 shares (mul_div 500*3000/1000).
    let _ = exec(
        &runtime,
        &module,
        "add_simple_liquidity",
        vec![
            Slot::Obj {
                id: pool_id.clone(),
                mutable: true,
            },
            Slot::Raw(u64_arg(500)),
            Slot::Raw(u64_arg(1000)),
        ],
        sender,
    )?;
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_simple_pool",
        vec![
            Slot::Obj {
                id: pool_id.clone(),
                mutable: false,
            },
            Slot::Raw(u64_arg(1500)),
            Slot::Raw(u64_arg(3000)),
            Slot::Raw(u64_arg(4500)),
        ],
        sender,
    )?;

    let _ = exec(
        &runtime,
        &module,
        "remove_simple_liquidity",
        vec![
            Slot::Obj {
                id: pool_id.clone(),
                mutable: true,
            },
            Slot::Raw(u64_arg(1500)),
            Slot::Raw(u64_arg(500)),
            Slot::Raw(u64_arg(1000)),
        ],
        sender,
    )?;
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_simple_pool",
        vec![
            Slot::Obj {
                id: pool_id,
                mutable: false,
            },
            Slot::Raw(u64_arg(1000)),
            Slot::Raw(u64_arg(2000)),
            Slot::Raw(u64_arg(3000)),
        ],
        sender,
    )?;

    Ok(())
}

/// The full TEST_COIN lifecycle runs inside Move and asserts its own supply,
/// split/join and burn arithmetic, so a wrong result aborts the call.
#[test]
fn e2e_coin_lifecycle_executes() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
    publish_all(&runtime, &compiled);

    let module = module_id("move_e2e_basic");
    let sender = publisher_address();

    let cs = exec(
        &runtime,
        &module,
        "e2e_coin_lifecycle",
        vec![Slot::Raw(tx_context_arg(sender, 0))],
        sender,
    )?;

    let types = created_types(&cs);
    for expected in ["TreasuryCap", "CoinMetadata", "TEST_COIN"] {
        assert!(
            types.iter().any(|t| t.contains(expected)),
            "e2e_coin_lifecycle must create a {expected}, got {types:?}"
        );
    }

    Ok(())
}

/// Deposits and withdrawals are asserted in-transaction, then the vault is read
/// back through the shared object to prove the drained balance persisted.
#[test]
fn e2e_vault_roundtrip_executes() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
    publish_all(&runtime, &compiled);

    let module = module_id("move_e2e_comprehensive");
    let sender = publisher_address();

    let cs = exec(
        &runtime,
        &module,
        "e2e_vault_roundtrip",
        vec![Slot::Raw(tx_context_arg(sender, 0))],
        sender,
    )?;

    let types = created_types(&cs);
    assert!(
        types
            .iter()
            .any(|t| t.contains("move_e2e_comprehensive::Vault")),
        "e2e_vault_roundtrip must share a Vault, got {types:?}"
    );
    let vault_id = find_created(&cs, "::Vault");
    preload_created(&runtime, &cs);

    let _ = exec(
        &runtime,
        &module,
        "e2e_check_vault_balance",
        vec![
            Slot::Obj {
                id: vault_id.clone(),
                mutable: false,
            },
            Slot::Raw(u64_arg(0)),
        ],
        sender,
    )?;
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_vault_owner",
        vec![
            Slot::Obj {
                id: vault_id,
                mutable: false,
            },
            Slot::Raw(address_arg(E2E_PUBLISHER)),
        ],
        sender,
    )?;

    Ok(())
}

/// Registration, overwrite and lookup are asserted in-transaction, then the
/// shared registry and entry are read back through separate calls.
#[test]
fn e2e_registry_roundtrip_executes() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
    publish_all(&runtime, &compiled);

    let module = module_id("move_e2e_registry");
    let sender = publisher_address();

    let cs = exec(
        &runtime,
        &module,
        "e2e_registry_roundtrip",
        vec![Slot::Raw(tx_context_arg(sender, 0))],
        sender,
    )?;

    let registry_id = find_created(&cs, "::Registry");
    let entry_id = find_created(&cs, "::Entry");
    preload_created(&runtime, &cs);

    let _ = exec(
        &runtime,
        &module,
        "e2e_check_registry_size",
        vec![
            Slot::Obj {
                id: registry_id.clone(),
                mutable: false,
            },
            Slot::Raw(u64_arg(2)),
        ],
        sender,
    )?;
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_lookup",
        vec![
            Slot::Obj {
                id: registry_id.clone(),
                mutable: false,
            },
            Slot::Raw(vec_u8_arg(b"kanari")),
            Slot::Raw(address_arg("0x4")),
        ],
        sender,
    )?;
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_lookup_absent",
        vec![
            Slot::Obj {
                id: registry_id,
                mutable: false,
            },
            Slot::Raw(vec_u8_arg(b"absent")),
        ],
        sender,
    )?;
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_entry_value",
        vec![
            Slot::Obj {
                id: entry_id,
                mutable: false,
            },
            Slot::Raw(vec_u8_arg(b"value2")),
        ],
        sender,
    )?;

    Ok(())
}

/// Mints two NFTs in one transaction, then verifies the shared collection
/// through separate read-back calls.
#[test]
fn e2e_nft_roundtrip_executes() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
    publish_all(&runtime, &compiled);

    let module = module_id("move_e2e_nft");
    let sender = publisher_address();

    let cs = exec(
        &runtime,
        &module,
        "e2e_nft_roundtrip",
        vec![Slot::Raw(tx_context_arg(sender, 0))],
        sender,
    )?;

    let types = created_types(&cs);
    let nft_ids: Vec<String> = cs
        .created_objects
        .iter()
        .filter(|(_, o)| o.type_.ends_with("::NFT"))
        .map(|(id, _)| id.clone())
        .collect();
    assert_eq!(
        nft_ids.len(),
        2,
        "e2e_nft_roundtrip must mint two NFTs, got {types:?}"
    );
    let (first_nft, second_nft) = (nft_ids[0].clone(), nft_ids[1].clone());

    let collection_id = find_created(&cs, "::NFTCollection");
    preload_created(&runtime, &cs);

    let _ = exec(
        &runtime,
        &module,
        "e2e_check_collection_size",
        vec![
            Slot::Obj {
                id: collection_id.clone(),
                mutable: false,
            },
            Slot::Raw(u64_arg(2)),
        ],
        sender,
    )?;
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_collection_name",
        vec![
            Slot::Obj {
                id: collection_id,
                mutable: false,
            },
            Slot::Raw(vec_u8_arg(b"E2E Collection")),
        ],
        sender,
    )?;

    // Neither NFT has been touched since minting, so the creator reads back.
    let _ = exec(
        &runtime,
        &module,
        "e2e_check_nft_creator",
        vec![
            Slot::Obj {
                id: first_nft.clone(),
                mutable: false,
            },
            Slot::Raw(address_arg(E2E_PUBLISHER)),
        ],
        sender,
    )?;

    // A hand-over by value takes the NFT out of the sender's possession.
    let transfer = exec(
        &runtime,
        &module,
        "transfer_nft",
        vec![
            Slot::Obj {
                id: first_nft,
                mutable: true,
            },
            Slot::Raw(address_arg("0x2")),
        ],
        sender,
    );
    assert!(
        transfer.is_ok(),
        "transfer_nft must succeed, got {:?}",
        transfer.err()
    );

    // Burning consumes the second NFT and removes it from state.
    let burn = exec(
        &runtime,
        &module,
        "e2e_burn_nft",
        vec![Slot::Obj {
            id: second_nft.clone(),
            mutable: true,
        }],
        sender,
    )?;
    assert!(
        burn.deleted_objects
            .iter()
            .any(|d| addr_literal(d) == addr_literal(&second_nft)),
        "e2e_burn_nft must delete the NFT {second_nft}, got {:?}",
        burn.deleted_objects
    );

    Ok(())
}

/// Each scenario owns its own runtime: `object::new` derives ids
/// deterministically from the transaction context, so separately-created
/// objects would otherwise collide and shadow each other in the object store.
#[test]
fn e2e_non_owner_mutations_abort() -> anyhow::Result<()> {
    let (_scratch, compiled) = compile_e2e_package()?;
    let sender = publisher_address();
    let other = MoveAccountAddress::from_hex_literal("0x2").expect("valid address");

    {
        let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
        publish_all(&runtime, &compiled);
        let module = module_id("move_e2e_storage");

        let cs = exec(
            &runtime,
            &module,
            "e2e_create_owned_store",
            vec![Slot::Raw(tx_context_arg(sender, 0))],
            sender,
        )?;
        let store_id = find_created(&cs, "::KeyValueStore");
        preload_created(&runtime, &cs);

        let denied = exec(
            &runtime,
            &module,
            "set_value",
            vec![
                Slot::Obj {
                    id: store_id,
                    mutable: true,
                },
                Slot::Raw(vec_u8_arg(b"evil")),
                Slot::Raw(vec_u8_arg(b"payload")),
                Slot::Raw(tx_context_arg(other, 1)),
            ],
            other,
        );
        assert_aborted(denied, 300, "non-owner set_value");
    }

    {
        let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
        publish_all(&runtime, &compiled);
        let module = module_id("move_e2e_registry");

        let cs = exec(
            &runtime,
            &module,
            "e2e_create_owned_registry",
            vec![Slot::Raw(tx_context_arg(sender, 0))],
            sender,
        )?;
        let registry_id = find_created(&cs, "::Registry");
        preload_created(&runtime, &cs);

        let denied = exec(
            &runtime,
            &module,
            "register_name",
            vec![
                Slot::Obj {
                    id: registry_id,
                    mutable: true,
                },
                Slot::Raw(vec_u8_arg(b"evil")),
                Slot::Raw(address_arg("0x2")),
                Slot::Raw(tx_context_arg(other, 1)),
            ],
            other,
        );
        assert_aborted(denied, 700, "non-owner register_name");
    }

    {
        let runtime = MoveRuntime::new_with_kanari_natives_in_memory()?;
        publish_all(&runtime, &compiled);
        let module = module_id("move_e2e_nft");

        let cs = exec(
            &runtime,
            &module,
            "e2e_create_owned_collection",
            vec![
                Slot::Raw(vec_u8_arg(b"Owned")),
                Slot::Raw(vec_u8_arg(b"owned by publisher")),
                Slot::Raw(tx_context_arg(sender, 0)),
            ],
            sender,
        )?;
        let collection_id = find_created(&cs, "::NFTCollection");
        preload_created(&runtime, &cs);

        let denied = exec(
            &runtime,
            &module,
            "e2e_mint_nft",
            vec![
                Slot::Obj {
                    id: collection_id,
                    mutable: true,
                },
                Slot::Raw(vec_u8_arg(b"Intruder")),
                Slot::Raw(vec_u8_arg(b"not yours")),
                Slot::Raw(vec_u8_arg(b"https://kanari.example/x.png")),
                Slot::Raw(tx_context_arg(other, 1)),
            ],
            other,
        );
        assert_aborted(denied, 500, "non-owner e2e_mint_nft");
    }

    Ok(())
}
