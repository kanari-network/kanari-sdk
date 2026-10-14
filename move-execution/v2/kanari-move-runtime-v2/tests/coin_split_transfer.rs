//! Regression: a partial coin split that sends the split part to a different
//! owner must leave the remainder on the original object and create a distinct
//! object for the recipient.
//!
//! Mirrors `kanari_system::pay::split_and_transfer`: the reported symptom is
//! that sending part of a coin "moves the whole coin" instead of separating the
//! two coin objects.

use anyhow::{Context, Result};
use kanari_move_runtime_v2::changeset::ChangeSet;
use kanari_move_runtime_v2::move_runtime::{EntryFunctionObjectContext, MoveRuntime};
use kanari_move_runtime_v2::storage::persistent_store::PersistentStore;
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

const TESTER_ADDR: &str = "0x4c";
const MODULE_NAME: &str = "coin_split_transfer";
const SYSTEM_PAY_ADDR: &str = "0x2";

const MINT_AMOUNT: u64 = 1_000;
const SEND_AMOUNT: u64 = 300;
const REMAINDER: u64 = MINT_AMOUNT - SEND_AMOUNT;

const MOVE_SOURCE: &str =
    include_str!("move_packages/coin_split_transfer/sources/coin_split_transfer.move");

#[test]
fn split_and_transfer_separates_objects() -> Result<()> {
    let package_dir = create_test_package()?;
    let (module_id, module_bytes) = compile_test_module(&package_dir)?;
    let sender = *module_id.address();
    assert_eq!(sender, AccountAddress::from_hex_literal(TESTER_ADDR)?);

    let recipient = AccountAddress::from_hex_literal("0xbeef")?;

    let store = Arc::new(PersistentStore::open_in_memory()?);
    let mut nonce = Nonce::default();
    let runtime = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;

    let _ = runtime
        .publish_module(module_bytes, sender, None, None)
        .context("publish coin_split_transfer module")?;

    // Create the test currency once (further creates abort), then faucet
    // a single coin of MINT_AMOUNT to the sender.
    let setup = entry(
        &runtime,
        &module_id,
        "setup_currency",
        vec![],
        vec![],
        vec![],
        &mut nonce,
    )?;
    let cap = find_created(&setup, |t| t.contains("TreasuryCap"))?;
    let mint = entry(
        &runtime,
        &module_id,
        "mint",
        vec![],
        vec![object_arg(&cap.id)?, bcs::to_bytes(&MINT_AMOUNT)?],
        vec![owned_input(&cap.id, sender, true)],
        &mut nonce,
    )?;
    let minted = find_created(&mint, |t| t.contains("coin::Coin") && t.contains("MOCK"))?;
    assert_eq!(coin_balance(&minted.data), MINT_AMOUNT);
    assert_eq!(minted.owner, sender);
    let original_id = minted.id.clone();

    // Send part of the coin to a different owner.
    let send = entry(
        &runtime,
        &module_id,
        "split_and_transfer",
        vec![],
        vec![
            object_arg(&original_id)?,
            bcs::to_bytes(&SEND_AMOUNT)?,
            bcs::to_bytes(&recipient)?,
        ],
        vec![owned_input(&original_id, sender, true)],
        &mut nonce,
    )?;

    let coins: Vec<_> = send
        .created_objects
        .iter()
        .filter(|(_, c)| c.type_.contains("coin::Coin") && c.type_.contains("MOCK"))
        .map(|(id, c)| (id.clone(), c.owner, coin_balance(&c.data)))
        .collect();

    let remainder = coins
        .iter()
        .find(|(id, _, _)| *id == original_id)
        .with_context(|| format!("original coin {original_id} must still exist"))?;
    assert_eq!(
        remainder.2, REMAINDER,
        "the original coin must hold the remainder after a partial split"
    );
    assert_eq!(remainder.1, sender, "the remainder stays with the sender");

    let sent = coins
        .iter()
        .find(|(id, _, _)| *id != original_id)
        .context("a distinct coin object must be created for the recipient")?;
    assert_eq!(
        sent.2, SEND_AMOUNT,
        "the recipient coin must carry the split amount"
    );
    assert_eq!(
        sent.1, recipient,
        "the split coin must be owned by the recipient"
    );
    assert_ne!(sent.0, original_id, "the two coin objects must be distinct");

    // A fresh runtime instance reads the persisted state: two distinct coins.
    let reader = MoveRuntime::new_with_kanari_natives_and_store(store)?;
    let stored_remainder = view_coin_value(
        &reader,
        TESTER_ADDR,
        MODULE_NAME,
        &owned_input(&original_id, sender, false),
    )?;
    assert_eq!(
        stored_remainder, REMAINDER,
        "persisted original coin must hold the remainder"
    );
    let stored_sent = view_coin_value(
        &reader,
        TESTER_ADDR,
        MODULE_NAME,
        &owned_input(&sent.0, recipient, false),
    )?;
    assert_eq!(
        stored_sent, SEND_AMOUNT,
        "persisted recipient coin must carry the split amount"
    );

    Ok(())
}

/// Same as above, but through the real production path:
/// `kanari_system::pay::split_and_transfer<T>` (generic, system address 0x2).
#[test]
fn system_pay_split_and_transfer_separates_objects() -> Result<()> {
    let package_dir = create_test_package()?;
    let (module_id, module_bytes) = compile_test_module(&package_dir)?;
    let sender = *module_id.address();
    let recipient = AccountAddress::from_hex_literal("0xbeef")?;

    let store = Arc::new(PersistentStore::open_in_memory()?);
    let mut nonce = Nonce::default();
    let runtime = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;

    let _ = runtime
        .publish_module(module_bytes, sender, None, None)
        .context("publish coin_split_transfer module")?;

    let setup = entry(
        &runtime,
        &module_id,
        "setup_currency",
        vec![],
        vec![],
        vec![],
        &mut nonce,
    )?;
    let cap = find_created(&setup, |t| t.contains("TreasuryCap"))?;
    let mint = entry(
        &runtime,
        &module_id,
        "mint",
        vec![],
        vec![object_arg(&cap.id)?, bcs::to_bytes(&MINT_AMOUNT)?],
        vec![owned_input(&cap.id, sender, true)],
        &mut nonce,
    )?;
    let minted = find_created(&mint, |t| t.contains("coin::Coin") && t.contains("MOCK"))?;
    let original_id = minted.id.clone();

    let pay_module = ModuleId::new(
        AccountAddress::from_hex_literal(SYSTEM_PAY_ADDR)?,
        Identifier::new("pay")?,
    );
    let send = entry(
        &runtime,
        &pay_module,
        "split_and_transfer",
        vec![mock_type()?],
        vec![
            object_arg(&original_id)?,
            bcs::to_bytes(&SEND_AMOUNT)?,
            bcs::to_bytes(&recipient)?,
        ],
        vec![owned_input(&original_id, sender, true)],
        &mut nonce,
    )?;

    assert_split_objects(&send, &original_id, sender, recipient)?;
    Ok(())
}

fn assert_split_objects(
    cs: &ChangeSet,
    original_id: &str,
    sender: AccountAddress,
    recipient: AccountAddress,
) -> Result<()> {
    let coins: Vec<_> = cs
        .created_objects
        .iter()
        .filter(|(_, c)| c.type_.contains("coin::Coin") && c.type_.contains("MOCK"))
        .map(|(id, c)| (id.clone(), c.owner, coin_balance(&c.data)))
        .collect();

    let remainder = coins
        .iter()
        .find(|(id, _, _)| id == original_id)
        .with_context(|| format!("original coin {original_id} must still exist"))?;
    assert_eq!(
        remainder.2, REMAINDER,
        "the original coin must hold the remainder after a partial split"
    );
    assert_eq!(remainder.1, sender, "the remainder stays with the sender");

    let sent = coins
        .iter()
        .find(|(id, _, _)| id != original_id)
        .context("a distinct coin object must be created for the recipient")?;
    assert_eq!(
        sent.2, SEND_AMOUNT,
        "the recipient coin must carry the split amount"
    );
    assert_eq!(
        sent.1, recipient,
        "the split coin must be owned by the recipient"
    );
    assert_ne!(sent.0, original_id, "the two coin objects must be distinct");
    Ok(())
}

fn mock_type() -> Result<TypeTag> {
    Ok(TypeTag::Struct(Box::new(StructTag::from_str(
        "0x4c::coin_split_transfer::MOCK",
    )?)))
}

fn view_coin_value(
    runtime: &MoveRuntime,
    package_addr: &str,
    module: &str,
    object_input: &ObjectInput,
) -> Result<u64> {
    let value = runtime.execute_view_function(
        package_addr,
        module,
        "value_of",
        &[],
        &[],
        std::slice::from_ref(object_input),
    )?;
    value.as_u64().context("value_of must return a number")
}

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

#[allow(clippy::too_many_arguments)]
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

fn create_test_package() -> Result<PathBuf> {
    let dir = tempdir()?;
    let package_dir = dir.keep();
    fs::create_dir_all(package_dir.join("sources"))?;

    let dependency_path = move_manifest::kanari_system_package_path()?;
    let manifest = format!(
        "[package]\nname = \"CoinSplitTransfer\"\n\n[dependencies]\nKanariSystem = {{ local = \"{}\" }}\n\n[addresses]\ntester = \"{}\"\n",
        dependency_path, TESTER_ADDR,
    );
    fs::write(package_dir.join("Move.toml"), manifest)?;
    fs::write(
        package_dir.join("sources").join("coin_split_transfer.move"),
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
