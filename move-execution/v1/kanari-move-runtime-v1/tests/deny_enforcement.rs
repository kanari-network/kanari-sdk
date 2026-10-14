//! End-to-end regulated-coin enforcement at the transfer layer.
//!
//! A regulated `MOCK` coin is created, an address is denied on the shared
//! deny list, and transfers through the *unchecked* `pay::split_and_transfer`
//! path must then fail at the runtime — proving enforcement lives in the
//! transfer funnel itself, not in opt-in Move helpers. Also covers undeny,
//! global pause, same-transaction deny+send (pending-write visibility), and
//! abort atomicity (a denied tx persists nothing, including its own deny).

use anyhow::{Context, Result};
use kanari_move_runtime_v1::changeset::ChangeSet;
use kanari_move_runtime_v1::move_runtime::{EntryFunctionObjectContext, MoveRuntime};
use kanari_move_runtime_v1::state::StateManager;
use kanari_move_runtime_v1::storage::persistent_store::PersistentStore;
use kanari_types::transaction::{ObjectInput, ObjectOwnerKind, ObjectRef};
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::{ModuleId, StructTag, TypeTag};
use move_package::BuildConfig;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use tempfile::tempdir;

#[path = "support/move_manifest.rs"]
mod move_manifest;

const TESTER_ADDR: &str = "0x42";
const MOCK_TYPE: &str = "0x42::deny_enforcement::MOCK";
const SYSTEM_PAY_ADDR: &str = "0x2";
/// Fixed deny-list singleton id (`deny_list::deny_list_object_id`).
const DENY_ID: &str = "0x0000000000000000000000000000000000000000000000000000000000000403";

const BAD: &str = "0xbad";
const CAFE: &str = "0xcafe";
const DAVID: &str = "0xda51d";

const MOVE_SOURCE: &str =
    include_str!("move_packages/deny_enforcement/sources/deny_enforcement.move");

struct Fixture {
    runtime: MoveRuntime,
    state: StateManager,
    module_id: ModuleId,
    nonce: Nonce,
    sender: AccountAddress,
}

fn setup() -> Result<(Fixture, String, String, String)> {
    let package_dir = create_test_package()?;
    let (module_id, module_bytes) = compile_test_module(&package_dir)?;
    let sender = *module_id.address();
    assert_eq!(sender, AccountAddress::from_hex_literal(TESTER_ADDR)?);

    let store = Arc::new(PersistentStore::open_in_memory()?);
    let nonce = Nonce::default();
    let runtime = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    let state = StateManager::new(store);

    let _ = runtime
        .publish_module(module_bytes, sender, None, None)
        .context("publish deny_enforcement module")?;

    let mut fx = Fixture {
        runtime,
        state,
        module_id,
        nonce,
        sender,
    };
    let module_id = fx.module_id.clone();
    let setup_cs = fx_entry(&mut fx, &module_id, "setup", vec![], vec![], vec![])?;
    let coin = find_created(&setup_cs, |t| {
        t.contains("coin::Coin") && t.contains("MOCK") && !t.contains("Metadata")
    })?;
    let cap = find_created(&setup_cs, |t| t.contains("TreasuryCap"))?;
    let denycap = find_created(&setup_cs, |t| t.contains("DenyCap"))?;
    assert_eq!(coin_balance(&coin.data), 1000);
    Ok((fx, coin.id, cap.id, denycap.id))
}

#[test]
fn denied_recipient_blocked_on_unchecked_path() -> Result<()> {
    let (mut fx, coin_id, _cap, denycap_id) = setup()?;
    let bad = AccountAddress::from_hex_literal(BAD)?;
    let cafe = AccountAddress::from_hex_literal(CAFE)?;
    let sender = fx.sender;

    // Deny BAD for MOCK.
    call_deny(&mut fx, &denycap_id, bad)?;

    // The UNCHECKED pay path to a denied address must fail at the runtime.
    let denied = entry(
        &mut fx,
        "pay",
        "split_and_transfer",
        vec![mock_type()?],
        vec![
            object_arg(&coin_id)?,
            bcs::to_bytes(&300u64)?,
            bcs::to_bytes(&bad)?,
        ],
        vec![owned_input(&coin_id, sender, true)],
        SYSTEM_PAY_ADDR,
    );
    assert_blocked(denied, "denied recipient");

    // Allowed recipient still goes through; the failed tx moved nothing.
    let send = entry(
        &mut fx,
        "pay",
        "split_and_transfer",
        vec![mock_type()?],
        vec![
            object_arg(&coin_id)?,
            bcs::to_bytes(&300u64)?,
            bcs::to_bytes(&cafe)?,
        ],
        vec![owned_input(&coin_id, sender, true)],
        SYSTEM_PAY_ADDR,
    )
    .with_context(|| "allowed transfer")?;
    let remainder = find_created(&send, |t| {
        t.contains("coin::Coin") && t.contains("MOCK") && !t.contains("Metadata")
    })?;
    assert_eq!(remainder.id, coin_id);
    assert_eq!(coin_balance(&remainder.data), 700);

    // Undeny: BAD works again.
    call_undeny(&mut fx, &denycap_id, bad)?;
    let _ = entry(
        &mut fx,
        "pay",
        "split_and_transfer",
        vec![mock_type()?],
        vec![
            object_arg(&coin_id)?,
            bcs::to_bytes(&100u64)?,
            bcs::to_bytes(&bad)?,
        ],
        vec![owned_input(&coin_id, sender, true)],
        SYSTEM_PAY_ADDR,
    )
    .with_context(|| "undenied transfer")?;
    Ok(())
}

#[test]
fn denied_sender_cannot_move_coins_out() -> Result<()> {
    let (mut fx, coin_id, _cap, denycap_id) = setup()?;
    let cafe = AccountAddress::from_hex_literal(CAFE)?;
    let sender = fx.sender;

    // Deny the sender itself: even a transfer to an allowed recipient must
    // abort — denial blocks sending as well as receiving.
    call_deny(&mut fx, &denycap_id, sender)?;
    let blocked = entry(
        &mut fx,
        "pay",
        "split_and_transfer",
        vec![mock_type()?],
        vec![
            object_arg(&coin_id)?,
            bcs::to_bytes(&100u64)?,
            bcs::to_bytes(&cafe)?,
        ],
        vec![owned_input(&coin_id, sender, true)],
        SYSTEM_PAY_ADDR,
    );
    assert_blocked(blocked, "denied sender");

    // Undeny restores the flow and nothing moved in between.
    call_undeny(&mut fx, &denycap_id, sender)?;
    let send = entry(
        &mut fx,
        "pay",
        "split_and_transfer",
        vec![mock_type()?],
        vec![
            object_arg(&coin_id)?,
            bcs::to_bytes(&100u64)?,
            bcs::to_bytes(&cafe)?,
        ],
        vec![owned_input(&coin_id, sender, true)],
        SYSTEM_PAY_ADDR,
    )
    .with_context(|| "undenied sender")?;
    let remainder = find_created(&send, |t| {
        t.contains("coin::Coin") && t.contains("MOCK") && !t.contains("Metadata")
    })?;
    assert_eq!(remainder.id, coin_id);
    assert_eq!(coin_balance(&remainder.data), 900);
    Ok(())
}

#[test]
fn denied_multisig_executor_cannot_move_funds() -> Result<()> {
    // Embedded-coin coverage: the wallet custodies the coins (no direct
    // coin input at execution), but the denied executor's own transaction
    // must still abort via the sender check.
    let (mut fx, coin_id, _cap, denycap_id) = setup()?;
    let cafe = AccountAddress::from_hex_literal(CAFE)?;
    let sender = fx.sender;
    let multisig = ModuleId::new(
        AccountAddress::from_hex_literal(SYSTEM_PAY_ADDR)?,
        Identifier::new("multisig")?,
    );

    // Fund a 1-of-[sender] wallet with the whole coin.
    let mut owners = vec![1u8];
    owners.extend_from_slice(&sender.to_vec());
    let create = fx_entry(
        &mut fx,
        &multisig,
        "create_wallet_entry",
        vec![mock_type()?],
        vec![
            owners,
            bcs::to_bytes(&1u64)?,
            object_arg(&coin_id)?,
            bcs::to_bytes(&1000u64)?,
        ],
        vec![owned_input(&coin_id, sender, true)],
    )?;
    let wallet = find_created(&create, |t| t.contains("multisig::MultisigWallet"))?;

    // Propose a transfer to an allowed recipient, then deny the executor.
    let propose = fx_entry(
        &mut fx,
        &multisig,
        "propose_transfer_entry",
        vec![mock_type()?],
        vec![
            object_arg(&wallet.id)?,
            bcs::to_bytes(&cafe)?,
            bcs::to_bytes(&100u64)?,
            bcs::to_bytes(&b"ok".to_vec())?,
            bcs::to_bytes(&60_000u64)?,
        ],
        vec![],
    )?;
    let proposal = find_created(&propose, |t| t.contains("TransactionProposal"))?;
    call_deny(&mut fx, &denycap_id, sender)?;

    // Execution moves wallet funds to an allowed address, but the denied
    // executor aborts the transaction.
    let executed = fx_entry(
        &mut fx,
        &multisig,
        "execute_entry",
        vec![mock_type()?],
        vec![object_arg(&wallet.id)?, object_arg(&proposal.id)?],
        vec![],
    );
    assert_blocked(executed, "denied multisig executor");

    // Undeny restores execution.
    call_undeny(&mut fx, &denycap_id, sender)?;
    let _ = fx_entry(
        &mut fx,
        &multisig,
        "execute_entry",
        vec![mock_type()?],
        vec![object_arg(&wallet.id)?, object_arg(&proposal.id)?],
        vec![],
    )
    .with_context(|| "undenied executor")?;
    Ok(())
}

#[test]
fn global_pause_blocks_all_checked_and_unchecked() -> Result<()> {
    let (mut fx, coin_id, _cap, denycap_id) = setup()?;
    let cafe = AccountAddress::from_hex_literal(CAFE)?;
    let sender = fx.sender;

    call_pause(&mut fx, &denycap_id)?;
    let paused = entry(
        &mut fx,
        "pay",
        "split_and_transfer",
        vec![mock_type()?],
        vec![
            object_arg(&coin_id)?,
            bcs::to_bytes(&100u64)?,
            bcs::to_bytes(&cafe)?,
        ],
        vec![owned_input(&coin_id, sender, true)],
        SYSTEM_PAY_ADDR,
    );
    assert_blocked(paused, "paused coin type");

    call_unpause(&mut fx, &denycap_id)?;
    let _ = entry(
        &mut fx,
        "pay",
        "split_and_transfer",
        vec![mock_type()?],
        vec![
            object_arg(&coin_id)?,
            bcs::to_bytes(&100u64)?,
            bcs::to_bytes(&cafe)?,
        ],
        vec![owned_input(&coin_id, sender, true)],
        SYSTEM_PAY_ADDR,
    )
    .with_context(|| "unpaused transfer")?;
    Ok(())
}

#[test]
fn same_tx_deny_and_send_aborts_atomically() -> Result<()> {
    let (mut fx, coin_id, _cap, denycap_id) = setup()?;
    let david = AccountAddress::from_hex_literal(DAVID)?;
    let sender = fx.sender;

    // Deny + send in ONE transaction: the pending deny write is visible to
    // enforcement, so this aborts...
    let module_id = fx.module_id.clone();
    let same_tx = fx_entry(
        &mut fx,
        &module_id,
        "deny_and_send",
        vec![],
        vec![
            object_arg(DENY_ID)?,
            object_arg(&denycap_id)?,
            object_arg(&coin_id)?,
            bcs::to_bytes(&100u64)?,
            bcs::to_bytes(&david)?,
        ],
        vec![
            owned_input(&denycap_id, sender, false),
            owned_input(&coin_id, sender, true),
        ],
    );
    assert_blocked(same_tx, "same-tx deny+send");

    // ...and the abort persisted nothing, including the deny itself: a plain
    // send to DAVID still succeeds.
    let _ = entry(
        &mut fx,
        "pay",
        "split_and_transfer",
        vec![mock_type()?],
        vec![
            object_arg(&coin_id)?,
            bcs::to_bytes(&100u64)?,
            bcs::to_bytes(&david)?,
        ],
        vec![owned_input(&coin_id, sender, true)],
        SYSTEM_PAY_ADDR,
    )
    .with_context(|| "post-abort transfer proves atomicity")?;

    // A real deny afterwards blocks again.
    call_deny(&mut fx, &denycap_id, david)?;
    let blocked = entry(
        &mut fx,
        "pay",
        "split_and_transfer",
        vec![mock_type()?],
        vec![
            object_arg(&coin_id)?,
            bcs::to_bytes(&100u64)?,
            bcs::to_bytes(&david)?,
        ],
        vec![owned_input(&coin_id, sender, true)],
        SYSTEM_PAY_ADDR,
    );
    assert_blocked(blocked, "denied after real deny");
    Ok(())
}

fn assert_blocked(result: Result<ChangeSet>, what: &str) {
    let err = result.unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("blocked"),
        "{what} must abort at enforcement, got: {msg}"
    );
}

fn call_deny(fx: &mut Fixture, denycap_id: &str, addr: AccountAddress) -> Result<()> {
    call_list_fn(fx, "deny", denycap_id, Some(addr))
}

fn call_undeny(fx: &mut Fixture, denycap_id: &str, addr: AccountAddress) -> Result<()> {
    call_list_fn(fx, "undeny", denycap_id, Some(addr))
}

fn call_pause(fx: &mut Fixture, denycap_id: &str) -> Result<()> {
    call_list_fn(fx, "pause", denycap_id, None)
}

fn call_unpause(fx: &mut Fixture, denycap_id: &str) -> Result<()> {
    call_list_fn(fx, "unpause", denycap_id, None)
}

fn call_list_fn(
    fx: &mut Fixture,
    function: &str,
    denycap_id: &str,
    addr: Option<AccountAddress>,
) -> Result<()> {
    let sender = fx.sender;
    let mut args = vec![object_arg(DENY_ID)?, object_arg(denycap_id)?];
    if let Some(addr) = addr {
        args.push(bcs::to_bytes(&addr)?);
    }
    let module_id = fx.module_id.clone();
    let _ = fx_entry(
        fx,
        &module_id,
        function,
        vec![],
        args,
        vec![owned_input(denycap_id, sender, false)],
    )?;
    Ok(())
}

/// Call an entry in the test module and persist via the state manager (so
/// dynamic-field writes like deny entries survive across transactions).
fn fx_entry(
    fx: &mut Fixture,
    module_id: &ModuleId,
    function: &str,
    type_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
    object_inputs: Vec<ObjectInput>,
) -> Result<ChangeSet> {
    let cs = fx
        .runtime
        .execute_entry_function_with_object_context_and_persistence(
            module_id,
            function,
            type_args,
            args,
            EntryFunctionObjectContext {
                object_inputs,
                sender: Some(fx.sender),
                gas_info: None,
                timestamp: None,
                tx_hash: Some(fx.nonce.next_bytes()),
                persist_runtime_state: true,
                state_overlay: None,
            },
        )?;
    fx.state.apply_changeset(&cs)?;
    fx.state.commit()?;
    Ok(cs)
}

/// Call an entry in a system (0x2) module.
#[allow(clippy::too_many_arguments)]
fn entry(
    fx: &mut Fixture,
    module: &str,
    function: &str,
    type_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
    object_inputs: Vec<ObjectInput>,
    address: &str,
) -> Result<ChangeSet> {
    let module_id = ModuleId::new(
        AccountAddress::from_hex_literal(address)?,
        Identifier::new(module)?,
    );
    fx_entry(fx, &module_id, function, type_args, args, object_inputs)
}

fn mock_type() -> Result<TypeTag> {
    Ok(TypeTag::Struct(Box::new(StructTag::from_str(MOCK_TYPE)?)))
}

struct Created {
    id: String,
    data: Vec<u8>,
}

fn find_created(cs: &ChangeSet, pred: impl Fn(&str) -> bool) -> Result<Created> {
    cs.created_objects
        .iter()
        .find(|(_, created)| pred(&created.type_))
        .map(|(id, created)| Created {
            id: id.clone(),
            data: created.data.clone(),
        })
        .context("expected created object not found")
}

fn coin_balance(data: &[u8]) -> u64 {
    assert!(data.len() >= 40, "coin data too short: {}", data.len());
    u64::from_le_bytes(data[32..40].try_into().expect("u64 coin balance in range"))
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

#[derive(Default)]
struct Nonce(u64);

impl Nonce {
    fn next_bytes(&mut self) -> Vec<u8> {
        self.0 += 1;
        self.0.to_le_bytes().to_vec()
    }
}

fn create_test_package() -> Result<PathBuf> {
    let dir = tempdir()?;
    let package_dir = dir.keep();
    fs::create_dir_all(package_dir.join("sources"))?;

    let dependency_path = move_manifest::kanari_system_package_path()?;
    let manifest = format!(
        "[package]\nname = \"DenyEnforcement\"\n\n[dependencies]\nKanariSystem = {{ local = \"{}\" }}\n\n[addresses]\ntester = \"{}\"\n",
        dependency_path, TESTER_ADDR,
    );
    fs::write(package_dir.join("Move.toml"), manifest)?;
    fs::write(
        package_dir.join("sources").join("deny_enforcement.move"),
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

#[test]
fn second_currency_for_same_type_aborts() -> Result<()> {
    // One canonical currency per coin type: setup() claims MOCK, so a
    // second setup on the same chain must abort instead of fragmenting
    // mint authority across TreasuryCaps.
    let (mut fx, _coin, _cap, _denycap) = setup()?;
    let module_id = fx.module_id.clone();
    let again = fx_entry(&mut fx, &module_id, "setup", vec![], vec![], vec![]);
    let err = again.unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("already claimed"),
        "duplicate currency must abort on claimed witness, got: {msg}"
    );
    Ok(())
}
