//! Capacity pin for the deny-policy inner gas budget.
//!
//! Both runtime gates (`enforce_coin_input_policy` /
//! `enforce_coin_output_policy`) run their policy read in a fresh session
//! backed by a fixed budget, [`DENY_POLICY_GAS_BUDGET`]. The budget decouples
//! the read from the transaction's own gas, but it is still consensus-critical:
//! if the framework's `deny_list_contains` / `deny_list_paused` read ever
//! outgrows the budget at realistic deny-set sizes, fail-closed semantics
//! would reject *every* movement of a regulated coin.
//!
//! This test pins the current cost curve: it grows the deny list to
//! `BATCH_SIZE` entries in a single transaction, then proves that, with the
//! list fully populated,
//!
//! * an allowed recipient's transfer still passes (read succeeds within the
//!   fixed inner budget), and
//! * a denied recipient's transfer is still rejected (`deny_list_contains`
//!   answered, even after a full-list scan).
//!
//! It also passes a deliberately starved `gas_info` for the outer
//! transaction to show that the inner read budget is independent of the
//! transaction's own gas allowance.
//!
//! Increase `BATCH_SIZE` deliberately as the framework read grows cheaper;
//! if these assertions start failing, the inner read has outgrown
//! `DENY_POLICY_GAS_BUDGET` and the constant (and commentary) must move
//! together.

use anyhow::{Context, Result};
use kanari_move_runtime_v2::changeset::ChangeSet;
use kanari_move_runtime_v2::move_runtime::EntryFunctionObjectContext;
use kanari_move_runtime_v2::move_runtime::MoveRuntime;
use kanari_move_runtime_v2::state::StateManager;
use kanari_move_runtime_v2::storage::persistent_store::PersistentStore;
use kanari_types::transaction::{ObjectInput, ObjectOwnerKind, ObjectRef};
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::ModuleId;
use move_package::BuildConfig;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::tempdir;

#[path = "support/move_manifest.rs"]
mod move_manifest;

const TESTER_ADDR: &str = "0x43";
const MODULE_NAME: &str = "deny_recipient_e2e";
const DENY_LIST_ID: &str = kanari_types::deny_list::DenyListModule::DENY_LIST_OBJECT_ID;
const MOVE_SOURCE: &str =
    include_str!("move_packages/deny_recipient_e2e/sources/deny_recipient_e2e.move");

/// Deny-list size the budget must accommodate with headroom. Not a target —
/// proof of safety margin at a plausible production-field scale. Chosen with
/// the write side in mind too: `deny_list::add` backs the set with a
/// `VecSet`, whose insert duplicate-check is itself O(n), so a single
/// transaction caps out around ~230 inserts under the default outer budget.
/// 64 entries stays comfortably inside that on writes while still making the
/// fixed inner read budget scan a non-trivially populated list.
const BATCH_SIZE: u64 = 64;

/// Denied recipient (inside the batch).
const DENIED_ADDR: &str = "0xCADA";
/// Recipient that never lands on the deny list.
const ALLOWED_ADDR: &str = "0xC0DE";

#[test]
fn deny_policy_reads_stay_within_the_fixed_budget_at_scale() -> Result<()> {
    let package_dir = create_test_package()?;
    let (module_id, module_bytes) = compile_test_module(&package_dir)?;
    let store = Arc::new(PersistentStore::open_in_memory()?);

    let mut state = StateManager::new(store.clone());
    let runtime = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;

    let _ = runtime
        .publish_module(module_bytes, *module_id.address(), None, None)
        .context("publish deny_recipient_e2e module")?;

    let sender_hex = module_id.address().to_hex_literal();
    let denied = AccountAddress::from_hex_literal(DENIED_ADDR)?;
    let allowed = AccountAddress::from_hex_literal(ALLOWED_ADDR)?;

    // --- regulated currency + initial supply -------------------------------
    let currency_changes = execute(
        &runtime,
        &module_id,
        "create_currency",
        vec![],
        vec![],
        None,
    )?;
    let treasury_id = object_of_type(&currency_changes, "TreasuryCap<")
        .context("treasury cap should be created")?;
    let deny_cap_id =
        object_of_type(&currency_changes, "DenyCap<").context("deny cap should be created")?;
    state.apply_changeset(&currency_changes)?;
    state.commit()?;

    let mint_changes = execute(
        &runtime,
        &module_id,
        "mint",
        vec![object_arg(&treasury_id)?, bcs::to_bytes(&10_000u64)?],
        vec![owned_input(&treasury_id, &sender_hex)],
        None,
    )
    .context("mint initial supply")?;
    let coin_id = object_of_type(&mint_changes, "::Coin<")
        .context("coin object should be minted to the sender")?;
    state.apply_changeset(&mint_changes)?;
    state.commit()?;

    // --- grow the deny list to BATCH_SIZE in one transaction ----------------
    // The denied recipient is planted inside the batch; the remaining slots
    // are distinct addresses so every `contains` miss must scan the whole list.
    let mut addrs: Vec<AccountAddress> = Vec::new();
    for i in 0..BATCH_SIZE {
        if i == 0 {
            addrs.push(denied);
        } else {
            // Distinct fill addresses: `0xB0`-prefixed, low byte set to the
            // slot index so every `contains` miss scans the whole list.
            let mut bytes = [0u8; 32];
            bytes[0] = 0xB0;
            bytes[31] = (i + 1) as u8;
            addrs.push(AccountAddress::new(bytes));
        }
    }
    // The outer transaction budget constrains writes too (append cost grows
    // with list size); grow the list in fixed-size chunks.
    let chunk = 16usize;
    for (start, chunk_addrs) in addrs.chunks(chunk).enumerate() {
        let batch_changes = execute(
            &runtime,
            &module_id,
            "add_denied_many",
            vec![
                object_arg(DENY_LIST_ID)?,
                object_arg(&deny_cap_id)?,
                bcs::to_bytes(&chunk_addrs)?,
            ],
            vec![
                shared_input(DENY_LIST_ID, true),
                owned_input(&deny_cap_id, &sender_hex),
            ],
            None,
        )
        .with_context(|| format!("batch add chunk {start} failed"))?;
        state.apply_changeset(&batch_changes)?;
        state.commit()?;
    }

    // --- fresh runtime: enforcement must work against the populated list ----
    // `gas_info` is deliberately starved: the inner policy read runs on its
    // own fixed budget, not on the transaction's allowance.
    let rt_enforced = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    let starved_gas = Some((10_000u64, 1_000));

    // Denied recipient: the full-list scan must still complete and answer.
    let error = attempt_transfer(
        &rt_enforced,
        &module_id,
        "split_then_transfer",
        &coin_id,
        1_000u64,
        &denied,
        starved_gas,
    )
    .err()
    .context("denied recipient transfer should be rejected against a populated list")?;
    let message = format!("{error:#}");
    assert!(
        message.contains("is denied for regulated coin"),
        "unexpected rejection: {message}"
    );

    // Starving the transaction's gas did not silently bypass enforcement:
    // the rejected transfer must still not have landed.
    let coin_after = state.get_object(&coin_id)?.context("coin should persist")?;
    assert_eq!(
        coin_after.owner.to_hex_literal(),
        sender_hex,
        "owner must be unchanged after the rejected transfer"
    );

    // Allowed recipient: the very same scans must complete within the fixed
    // inner budget, or fail-closed semantics would reject a legitimate
    // transfer — the regression this test exists to catch.
    let ok_transfer = attempt_transfer(
        &rt_enforced,
        &module_id,
        "split_then_transfer",
        &coin_id,
        1_000u64,
        &allowed,
        starved_gas,
    )
    .context(
        "allowed recipient transfer must succeed with a populated deny list; \
         if this fails, the inner policy read no longer fits the gas budget",
    )?;
    state.apply_changeset(&ok_transfer)?;
    state.commit()?;
    let piece_id = delivered_piece(&ok_transfer, &allowed.to_hex_literal())
        .context("the transferred piece should land at the allowed recipient")?;
    let piece = state
        .get_object(&piece_id)?
        .context("piece should persist")?;
    assert_eq!(
        piece.owner.to_hex_literal(),
        allowed.to_hex_literal(),
        "piece ownership must be the allowed recipient"
    );

    // Starving the transaction's gas did not silently bypass enforcement:
    Ok(())
}

// --- helpers ------------------------------------------------------------

/// Push a coin piece from the sender-owned coin to `recipient` through the
/// plain, unchecked path (`coin::split` + `public_transfer`).
fn attempt_transfer(
    runtime: &MoveRuntime,
    module_id: &ModuleId,
    function: &str,
    coin_id: &str,
    amount: u64,
    recipient: &AccountAddress,
    gas_info: Option<(u64, u64)>,
) -> Result<ChangeSet> {
    execute(
        runtime,
        module_id,
        function,
        vec![
            object_arg(coin_id)?,
            bcs::to_bytes(&amount)?,
            bcs::to_bytes(recipient)?,
        ],
        vec![owned_input(coin_id, &module_id.address().to_hex_literal())],
        gas_info,
    )
}

/// Find the id of the created object whose type contains `type_marker`.
fn object_of_type(changes: &ChangeSet, type_marker: &str) -> Option<String> {
    changes
        .created_objects
        .iter()
        .find(|(_, object)| object.type_.contains(type_marker))
        .map(|(id, _)| id.clone())
}

/// Find the id of the coin created in `changes` whose owner is `owner_hex`.
fn delivered_piece(changes: &ChangeSet, owner_hex: &str) -> Option<String> {
    changes
        .created_objects
        .iter()
        .find(|(_, object)| {
            object.type_.contains("::Coin<") && object.owner.to_hex_literal() == owner_hex
        })
        .map(|(id, _)| id.clone())
}

fn execute(
    runtime: &MoveRuntime,
    module_id: &ModuleId,
    function: &str,
    args: Vec<Vec<u8>>,
    object_inputs: Vec<ObjectInput>,
    gas_info: Option<(u64, u64)>,
) -> Result<ChangeSet> {
    runtime.execute_entry_function_with_object_context_and_persistence(
        module_id,
        function,
        vec![],
        args,
        EntryFunctionObjectContext {
            object_inputs,
            sender: Some(*module_id.address()),
            gas_info,
            timestamp: None,
            tx_hash: Some({
                use std::sync::atomic::{AtomicU64, Ordering};
                static C: AtomicU64 = AtomicU64::new(1);
                C.fetch_add(1, Ordering::Relaxed).to_le_bytes().to_vec()
            }),
            persist_runtime_state: true,
            state_overlay: None,
        },
    )
}

fn owned_input(object_id: &str, owner_hex: &str) -> ObjectInput {
    ObjectInput {
        object_ref: ObjectRef::new(object_id.to_string(), None, None),
        owner: Some(ObjectOwnerKind::AddressOwner(owner_hex.to_string())),
        mutable: true,
    }
}

fn shared_input(object_id: &str, mutable: bool) -> ObjectInput {
    ObjectInput {
        object_ref: ObjectRef::new(object_id.to_string(), None, None),
        owner: Some(ObjectOwnerKind::Shared),
        mutable,
    }
}

fn create_test_package() -> Result<PathBuf> {
    let dir = tempdir()?;
    let package_dir = dir.keep();
    fs::create_dir_all(package_dir.join("sources"))?;

    let dependency_path = move_manifest::kanari_system_package_path()?;
    let manifest = format!(
        "[package]\nname = \"DenyRecipientCapacity\"\n\n[dependencies]\nKanariSystem = {{ local = \"{}\" }}\n\n[addresses]\ntester = \"{}\"\n",
        dependency_path, TESTER_ADDR,
    );
    fs::write(package_dir.join("Move.toml"), manifest)?;

    fs::write(
        package_dir
            .join("sources")
            .join(format!("{}.move", MODULE_NAME)),
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

fn object_arg(object_id: &str) -> Result<Vec<u8>> {
    // Canonicalize to the full 32-byte address; raw hex-decode of short
    // literals like `0x403` would fail (odd digit count).
    Ok(AccountAddress::from_hex_literal(object_id)?.to_vec())
}
