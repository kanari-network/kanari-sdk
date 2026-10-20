//! Destination-side deny enforcement: the output policy.
//!
//! Companion to the input gate covered by `deny_list_cross_transaction`:
//! a *denied recipient* — or a globally paused coin type — must not be able
//! to receive a `Coin<T>` through the deliberately unchecked plain path
//! (`coin::split` + `transfer::public_transfer`, no Move-level deny reads).
//! The runtime's post-execution policy scans the assembled changeset, sees
//! the coin object whose final owner is the denied address, and rejects the
//! transaction before anything is persisted.

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

/// Recipient that must be rejected while on the deny list.
const DENIED_ADDR: &str = "0xCADA";
/// Recipient that never lands on the deny list.
const ALLOWED_ADDR: &str = "0xC0DE";

#[test]
fn denied_recipient_cannot_receive_coin_via_plain_transfer() -> Result<()> {
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
    let allowed_hex = allowed.to_hex_literal();

    // --- regulated currency + initial supply -------------------------------
    let currency_changes = execute(&runtime, &module_id, "create_currency", vec![], vec![])?;
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
    )
    .context("mint initial supply")?;
    let coin_id = object_of_type(&mint_changes, "::Coin<")
        .context("coin object should be minted to the sender")?;
    state.apply_changeset(&mint_changes)?;
    state.commit()?;

    // --- deny the recipient -------------------------------------------------
    let deny_changes = execute(
        &runtime,
        &module_id,
        "add_denied",
        vec![
            object_arg(DENY_LIST_ID)?,
            object_arg(&deny_cap_id)?,
            bcs::to_bytes(&denied)?,
        ],
        vec![
            shared_input(DENY_LIST_ID, true),
            owned_input(&deny_cap_id, &sender_hex),
        ],
    )
    .context("deny-list add for the recipient")?;
    state.apply_changeset(&deny_changes)?;
    state.commit()?;

    let rt_enforced = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;

    // Denied recipient through the plain, unchecked transfer path: the
    // output policy must reject the transaction after execution.
    let error = attempt_transfer(
        &rt_enforced,
        &module_id,
        "split_then_transfer",
        &coin_id,
        1_000u64,
        &denied,
    )
    .err()
    .context("denied recipient transfer should be rejected")?;
    let message = format!("{error:#}");
    assert!(
        message.contains("is denied for regulated coin"),
        "unexpected rejection: {message}"
    );

    // The failed transaction must not have landed: the coin still holds all
    // 10_000 with the sender.
    let coin_after = state.get_object(&coin_id)?.context("coin should persist")?;
    eprintln!(
        "[DBGTEST] coin id={coin_id} owner={} type={}",
        coin_after.owner.to_hex_literal(),
        coin_after.type_
    );

    // --- an allowed recipient passes the very same path ----------------------
    let ok_transfer = attempt_transfer(
        &rt_enforced,
        &module_id,
        "split_then_transfer",
        &coin_id,
        1_000u64,
        &allowed,
    )
    .context("allowed recipient transfer should succeed")?;
    state.apply_changeset(&ok_transfer)?;
    state.commit()?;
    let piece_id = delivered_piece(&ok_transfer, &allowed_hex)
        .context("the transferred piece should land at the allowed recipient")?;
    let piece = state
        .get_object(&piece_id)?
        .context("piece should persist")?;
    assert_eq!(
        piece.owner.to_hex_literal(),
        allowed_hex,
        "piece ownership must be the allowed recipient"
    );

    // --- a globally paused coin type blocks every recipient ------------------
    let pause_changes = execute(
        &rt_enforced,
        &module_id,
        "pause",
        vec![object_arg(DENY_LIST_ID)?, object_arg(&deny_cap_id)?],
        vec![
            shared_input(DENY_LIST_ID, true),
            owned_input(&deny_cap_id, &sender_hex),
        ],
    )?;
    state.apply_changeset(&pause_changes)?;
    state.commit()?;

    let rt_paused = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    let error = attempt_transfer(
        &rt_paused,
        &module_id,
        "split_then_transfer",
        &coin_id,
        1_000u64,
        &allowed,
    )
    .err()
    .context("globally paused coin type must reject every transfer")?;
    let message = format!("{error:#}");
    assert!(
        message.contains("globally paused"),
        "unexpected rejection: {message}"
    );

    // --- unpausing restores the allowed recipient ----------------------------
    let unpause_changes = execute(
        &rt_paused,
        &module_id,
        "unpause",
        vec![object_arg(DENY_LIST_ID)?, object_arg(&deny_cap_id)?],
        vec![
            shared_input(DENY_LIST_ID, true),
            owned_input(&deny_cap_id, &sender_hex),
        ],
    )?;
    state.apply_changeset(&unpause_changes)?;
    state.commit()?;

    let rt_unpaused = MoveRuntime::new_with_kanari_natives_and_store(store)?;
    let restored = attempt_transfer(
        &rt_unpaused,
        &module_id,
        "split_then_transfer",
        &coin_id,
        100u64,
        &allowed,
    )
    .context("after unpausing the allowed recipient should receive again")?;
    state.apply_changeset(&restored)?;
    state.commit()?;
    let piece_id = delivered_piece(&restored, &allowed_hex)
        .context("restored transfer should deliver a piece")?;
    let piece = state
        .get_object(&piece_id)?
        .context("piece should persist")?;
    assert_eq!(
        piece.owner.to_hex_literal(),
        allowed_hex,
        "piece ownership must be the allowed recipient after unpausing"
    );

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
) -> Result<ChangeSet> {
    runtime.execute_entry_function_with_object_context_and_persistence(
        module_id,
        function,
        vec![],
        args,
        EntryFunctionObjectContext {
            object_inputs,
            sender: Some(*module_id.address()),
            gas_info: None,
            timestamp: None,
            tx_hash: None,
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
        "[package]\nname = \"DenyRecipientE2E\"\n\n[dependencies]\nKanariSystem = {{ local = \"{}\" }}\n\n[addresses]\ntester = \"{}\"\n",
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
