//! End-to-end staked-voting governance flow on the genesis-loaded `0x3`
//! package.
//!
//! `kanari_governance::governance` is published (and `init`ed) during genesis,
//! which shares the `GovernanceState` singleton and transfers the admin
//! `GovernanceCap` to the init sender. This test boots the runtime, publishes
//! a thin wrapper module (`tester::governance_e2e` at `@0x43`), then drives a
//! full token-weighted vote:
//!
//! 1. claim `TEST_COIN` via `coin::create_currency`,
//! 2. escrow a deposit and create a proposal,
//! 3. two distinct voters stake `TEST_COIN` coins for/against,
//! 4. execution after the voting window records the quorum/majority result,
//! 5. voters withdraw their escrowed stakes and the creator reclaims the
//!    deposit,
//! 6. a second proposal with a large `total_supply` demonstrates the quorum
//!    gate rejecting a lopsided-but-underweight vote.
//!
//! Every mutation lands on the genesis `GovernanceState` and the shared
//! `Proposal` objects and is visible after a fresh runtime boot.

use anyhow::{Context, Result};
use kanari_move_runtime_v2::changeset::{ChangeSet, CreatedObject};
use kanari_move_runtime_v2::move_runtime::{EntryFunctionObjectContext, MoveRuntime};
use kanari_move_runtime_v2::state::StateManager;
use kanari_move_runtime_v2::storage::persistent_store::PersistentStore;
use kanari_types::transaction::{ObjectInput, ObjectOwnerKind, ObjectRef};
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::ModuleId;
use move_package::BuildConfig;
use serde_json::Value as JsonValue;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::tempdir;

#[path = "support/move_manifest.rs"]
mod move_manifest;

const TESTER_ADDR: &str = "0x43";
const MODULE_NAME: &str = "governance_e2e";
const MOVE_SOURCE: &str = include_str!("move_packages/governance_e2e/sources/governance_e2e.move");

/// Two voters. `VOTER_A` is also the module address / currency owner.
const VOTER_A: &str = "0x43";
const VOTER_B: &str = "0x44";

const PROPOSAL_TS: u64 = 1_000;
const VOTING_PERIOD: u64 = 1_000;
const AFTER_VOTE_TS: u64 = 3_000;

const DEPOSIT: u64 = 10_000;

#[test]
fn governance_staked_vote_flow_end_to_end() -> Result<()> {
    let package_dir = create_test_package()?;
    let (module_id, module_bytes) = compile_test_module(&package_dir)?;
    let store = Arc::new(PersistentStore::open_in_memory()?);

    let mut state = StateManager::new(store.clone());
    let runtime = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;

    // Genesis must have shared the GovernanceState singleton and transferred
    // the admin GovernanceCap to the init sender.
    let all_objects = state.query_objects(None, None, None, None, None)?;
    let (state_id, _) = single_object(&all_objects, "::governance::GovernanceState")
        .context("genesis must share exactly one GovernanceState")?;
    let (admin_cap_id, cap_object) = single_object(&all_objects, "::governance::GovernanceCap")
        .context("genesis must transfer exactly one GovernanceCap")?;
    let cap_owner = cap_object.owner.to_hex_literal();

    let _ = runtime
        .publish_module(module_bytes, *module_id.address(), None, None)
        .context("publish governance_e2e module")?;

    // --- claim TEST_COIN ---------------------------------------------------
    let changes =
        call_entry(&runtime, &module_id, "create_currency", vec![], vec![], PROPOSAL_TS, VOTER_A)
            .context("create_currency")?;
    let (cap_id, _) = find_created(&changes, "::coin::TreasuryCap<", "::governance_e2e::TEST_COIN")
        .context("TreasuryCap<TEST_COIN> should be created")?;
    state.apply_changeset(&changes)?;
    state.commit()?;

    let cap_input = |state: &StateManager, mutable: bool| -> Result<ObjectInput> {
        owned_object_input(state, &cap_id, mutable)
    };

    // --- proposal 1: majority wins (quorum trivially met, supply = 0) ------
    let changes = call_entry(
        &runtime,
        &module_id,
        "mint_and_propose",
        vec![
            object_arg(&cap_id)?,
            object_arg(&state_id)?,
            bcs::to_bytes(&DEPOSIT)?,
            bcs::to_bytes(&b"Treasury diversification".to_vec())?,
            bcs::to_bytes(&b"Move 5% of the treasury into stables".to_vec())?,
            bcs::to_bytes(&VOTING_PERIOD)?,
        ],
        vec![cap_input(&state, true)?, shared_object_input(&state_id, true)],
        PROPOSAL_TS,
        VOTER_A,
    )
    .context("mint_and_propose 1")?;
    let (proposal_1, _) =
        find_created(&changes, "::governance::Proposal<", "::governance_e2e::TEST_COIN")
            .context("shared Proposal 1 should be created")?;
    state.apply_changeset(&changes)?;
    state.commit()?;

    // Stake coins for the two voters.
    // Stake coins are minted to each voter inside `vote_with_coin`.

    // ALICE stakes 600 for, BOB stakes 300 against.
    vote_with_coin(
        &store,
        &module_id,
        &mut state,
        &cap_id,
        600,
        VOTER_A,
        &state_id,
        &proposal_1,
        true,
    )?;
    vote_with_coin(
        &store,
        &module_id,
        &mut state,
        &cap_id,
        300,
        VOTER_B,
        &state_id,
        &proposal_1,
        false,
    )?;

    // Tallies must read back through a fresh runtime boot.
    let rt_read = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    assert_view(&rt_read, "yes_votes", &proposal_1, JsonValue::from(600u64), "yes tally")?;
    assert_view(&rt_read, "no_votes", &proposal_1, JsonValue::from(300u64), "no tally")?;
    assert_view(&rt_read, "total_staked", &proposal_1, JsonValue::from(900u64), "stake sum")?;
    assert_view(&rt_read, "deposit_of", &proposal_1, JsonValue::from(DEPOSIT), "escrowed deposit")?;
    assert_view(&rt_read, "is_executed", &proposal_1, JsonValue::from(0u64), "not executed yet")?;

    // Execution before the window closes must abort (E_VOTING_NOT_ENDED).
    let early = call_entry(
        &rt_read,
        &module_id,
        "execute",
        vec![object_arg(&state_id)?, object_arg(&proposal_1)?],
        vec![shared_object_input(&state_id, false), shared_object_input(&proposal_1, true)],
        PROPOSAL_TS + 100,
        VOTER_A,
    );
    assert!(early.is_err(), "executing an open proposal must abort with E_VOTING_NOT_ENDED");

    // Execute after the window: 600 > 300 and quorum is trivially met.
    let changes = call_entry(
        &rt_read,
        &module_id,
        "execute",
        vec![object_arg(&state_id)?, object_arg(&proposal_1)?],
        vec![shared_object_input(&state_id, false), shared_object_input(&proposal_1, true)],
        AFTER_VOTE_TS,
        VOTER_A,
    )
    .context("execute proposal 1")?;
    state.apply_changeset(&changes)?;
    state.commit()?;

    let rt_done = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    assert_view(&rt_done, "is_executed", &proposal_1, JsonValue::from(1u64), "executed")?;
    assert_view(
        &rt_done,
        "execution_result",
        &proposal_1,
        JsonValue::from(1u64),
        "majority + quorum should pass",
    )?;

    // Voters withdraw their escrowed stakes; the creator reclaims the deposit.
    let changes = call_entry(
        &rt_done,
        &module_id,
        "withdraw_stake",
        vec![object_arg(&proposal_1)?],
        vec![shared_object_input(&proposal_1, true)],
        AFTER_VOTE_TS,
        VOTER_A,
    )
    .context("withdraw_stake A")?;
    state.apply_changeset(&changes)?;
    state.commit()?;

    let rt_withdrawn_a = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    let changes = call_entry(
        &rt_withdrawn_a,
        &module_id,
        "withdraw_stake",
        vec![object_arg(&proposal_1)?],
        vec![shared_object_input(&proposal_1, true)],
        AFTER_VOTE_TS,
        VOTER_B,
    )
    .context("withdraw_stake B")?;
    state.apply_changeset(&changes)?;
    state.commit()?;

    let rt_withdrawn = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    assert_view(
        &rt_withdrawn,
        "total_staked",
        &proposal_1,
        JsonValue::from(0u64),
        "all stakes withdrawn",
    )?;

    let changes = call_entry(
        &rt_withdrawn,
        &module_id,
        "withdraw_deposit",
        vec![object_arg(&proposal_1)?],
        vec![shared_object_input(&proposal_1, true)],
        AFTER_VOTE_TS,
        VOTER_A,
    )
    .context("withdraw_deposit")?;
    state.apply_changeset(&changes)?;
    state.commit()?;

    // --- proposal 2: quorum gate rejects an underweight win ----------------
    // A large `total_supply` (via the genesis GovernanceCap) demands 100_000
    // staked at the default 10% threshold; 100 staked "yes" votes must fail.
    let changes = call_entry(
        &rt_withdrawn,
        &module_id,
        "set_total_supply",
        vec![object_arg(&admin_cap_id)?, object_arg(&state_id)?, bcs::to_bytes(&1_000_000u64)?],
        vec![
            owned_object_input(&state, &admin_cap_id, false)?,
            shared_object_input(&state_id, true),
        ],
        AFTER_VOTE_TS,
        &cap_owner,
    )
    .context("set_total_supply")?;
    state.apply_changeset(&changes)?;
    state.commit()?;

    let rt_supply = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    assert_state_view(
        &rt_supply,
        "state_total_supply",
        &state_id,
        JsonValue::from(1_000_000u64),
        "total_supply maintained by admin",
    )?;

    let changes = call_entry(
        &rt_supply,
        &module_id,
        "mint_and_propose",
        vec![
            object_arg(&cap_id)?,
            object_arg(&state_id)?,
            bcs::to_bytes(&DEPOSIT)?,
            bcs::to_bytes(&b"Mint more supply".to_vec())?,
            bcs::to_bytes(&b"Increase emissions by 2%".to_vec())?,
            bcs::to_bytes(&VOTING_PERIOD)?,
        ],
        vec![owned_object_input(&state, &cap_id, true)?, shared_object_input(&state_id, true)],
        PROPOSAL_TS,
        VOTER_A,
    )
    .context("mint_and_propose 2")?;
    let (proposal_2, _) =
        find_created(&changes, "::governance::Proposal<", "::governance_e2e::TEST_COIN")
            .context("shared Proposal 2 should be created")?;
    state.apply_changeset(&changes)?;
    state.commit()?;

    vote_with_coin(
        &store,
        &module_id,
        &mut state,
        &cap_id,
        100,
        VOTER_A,
        &state_id,
        &proposal_2,
        true,
    )?;

    let rt_final = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    assert_state_view(
        &rt_final,
        "state_total_supply",
        &state_id,
        JsonValue::from(1_000_000u64),
        "total_supply still 1_000_000 before executing p2",
    )?;
    assert_view(&rt_final, "total_staked", &proposal_2, JsonValue::from(100u64), "p2 stake sum")?;
    let changes = call_entry(
        &rt_final,
        &module_id,
        "execute",
        vec![object_arg(&state_id)?, object_arg(&proposal_2)?],
        vec![shared_object_input(&state_id, false), shared_object_input(&proposal_2, true)],
        AFTER_VOTE_TS,
        VOTER_A,
    )
    .context("execute proposal 2")?;
    state.apply_changeset(&changes)?;
    state.commit()?;

    let rt_check = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    assert_view(&rt_check, "is_executed", &proposal_2, JsonValue::from(1u64), "executed")?;
    assert_view(
        &rt_check,
        "execution_result",
        &proposal_2,
        JsonValue::from(0u64),
        "yes-majority without quorum must fail",
    )?;

    Ok(())
}

/// Vote by escrowing the sender's stake coin (minted through the wrapper and
/// transferred to the voter first). The coin object is consumed by the vote.
#[allow(clippy::too_many_arguments)]
fn vote_with_coin(
    store: &Arc<PersistentStore>,
    module_id: &ModuleId,
    state: &mut StateManager,
    cap_id: &str,
    amount: u64,
    voter: &str,
    state_id: &str,
    proposal_id: &str,
    support: bool,
) -> Result<()> {
    // Mint and transfer the stake coin to the voter.
    let runtime = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    let changes = call_entry(
        &runtime,
        module_id,
        "mint_to",
        vec![
            object_arg(cap_id)?,
            bcs::to_bytes(&amount)?,
            bcs::to_bytes(&AccountAddress::from_hex_literal(voter)?)?,
        ],
        vec![owned_object_input(state, cap_id, true)?],
        PROPOSAL_TS,
        VOTER_A,
    )
    .context("mint stake coin")?;
    let (coin_id, _) = find_created(&changes, "::coin::Coin<", "::governance_e2e::TEST_COIN")
        .context("stake coin should be created")?;
    state.apply_changeset(&changes)?;
    state.commit()?;

    // The voter stakes the coin on the proposal.
    let runtime = MoveRuntime::new_with_kanari_natives_and_store(store.clone())?;
    let changes = call_entry(
        &runtime,
        module_id,
        "vote_with_coin",
        vec![
            object_arg(&coin_id)?,
            object_arg(state_id)?,
            object_arg(proposal_id)?,
            bcs::to_bytes(&support)?,
        ],
        vec![
            owned_object_input(state, &coin_id, true)?,
            shared_object_input(state_id, true),
            shared_object_input(proposal_id, true),
        ],
        PROPOSAL_TS,
        voter,
    )
    .context("vote_with_coin")?;
    state.apply_changeset(&changes)?;
    state.commit()?;
    Ok(())
}

fn call_entry(
    runtime: &MoveRuntime,
    module_id: &ModuleId,
    function: &str,
    args: Vec<Vec<u8>>,
    object_inputs: Vec<ObjectInput>,
    timestamp: u64,
    sender: &str,
) -> Result<ChangeSet> {
    runtime.execute_entry_function_with_object_context_and_persistence(
        module_id,
        function,
        vec![],
        args,
        EntryFunctionObjectContext {
            object_inputs,
            sender: Some(AccountAddress::from_hex_literal(sender)?),
            gas_info: None,
            timestamp: Some(timestamp),
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

fn view(
    runtime: &MoveRuntime,
    function: &str,
    args: Vec<Vec<u8>>,
    inputs: Vec<ObjectInput>,
) -> Result<JsonValue> {
    runtime.execute_view_function(TESTER_ADDR, MODULE_NAME, function, &[], &args, &inputs)
}

fn assert_view(
    runtime: &MoveRuntime,
    function: &str,
    proposal_id: &str,
    expected: JsonValue,
    label: &str,
) -> Result<()> {
    let actual = view(
        runtime,
        function,
        vec![object_arg(proposal_id)?],
        vec![shared_object_input(proposal_id, false)],
    )
    .with_context(|| format!("view {function} ({label})"))?;
    assert_eq!(actual, expected, "{label}");
    Ok(())
}

fn assert_state_view(
    runtime: &MoveRuntime,
    function: &str,
    state_id: &str,
    expected: JsonValue,
    label: &str,
) -> Result<()> {
    let actual = view(
        runtime,
        function,
        vec![object_arg(state_id)?],
        vec![shared_object_input(state_id, false)],
    )
    .with_context(|| format!("view {function} ({label})"))?;
    assert_eq!(actual, expected, "{label}");
    Ok(())
}

/// The single object whose type ends with `suffix`; errors when zero or
/// several match.
fn single_object<'a>(
    objects: &'a [(String, CreatedObject)],
    suffix: &str,
) -> Result<&'a (String, CreatedObject)> {
    let mut matches = objects.iter().filter(|(_, object)| object.type_.ends_with(suffix));
    let first = matches.next().context(format!("no object of type *{suffix}"))?;
    assert!(matches.next().is_none(), "expected exactly one object of type *{suffix}");
    Ok(first)
}

/// The single object created by `changes` whose type contains both markers.
fn find_created(
    changes: &ChangeSet,
    marker: &str,
    coin_type_marker: &str,
) -> Option<(String, CreatedObject)> {
    changes
        .created_objects
        .iter()
        .find(|(_, object)| {
            object.type_.contains(marker) && object.type_.contains(coin_type_marker)
        })
        .map(|(id, object)| (id.clone(), object.clone()))
}

/// Owned-object input carrying the current version/digest from `state`.
fn owned_object_input(state: &StateManager, object_id: &str, mutable: bool) -> Result<ObjectInput> {
    let object = state
        .get_object(object_id)?
        .with_context(|| format!("object {object_id} missing from state"))?;
    let owner_hex = object.owner.to_hex_literal();
    Ok(ObjectInput {
        object_ref: object.object_ref(object_id),
        owner: Some(ObjectOwnerKind::AddressOwner(owner_hex)),
        mutable,
    })
}

fn shared_object_input(object_id: &str, mutable: bool) -> ObjectInput {
    ObjectInput {
        object_ref: ObjectRef::new(object_id.to_string(), None, None),
        owner: Some(ObjectOwnerKind::Shared),
        mutable,
    }
}

fn object_arg(object_id: &str) -> Result<Vec<u8>> {
    Ok(AccountAddress::from_hex_literal(object_id)?.to_vec())
}

fn create_test_package() -> Result<PathBuf> {
    let dir = tempdir()?;
    let package_dir = dir.keep();
    fs::create_dir_all(package_dir.join("sources"))?;

    let system_path = move_manifest::kanari_system_package_path()?;
    let governance_path = move_manifest::kanari_governance_package_path()?;
    let manifest = format!(
        "[package]\nname = \"GovernanceE2E\"\n\n[dependencies]\nKanariSystem = {{ local = \"{}\" }}\nKanariGovernance = {{ local = \"{}\" }}\n\n[addresses]\ntester = \"{}\"\n",
        system_path, governance_path, TESTER_ADDR,
    );
    fs::write(package_dir.join("Move.toml"), manifest)?;

    fs::write(package_dir.join("sources").join(format!("{}.move", MODULE_NAME)), MOVE_SOURCE)?;

    Ok(package_dir)
}

fn compile_test_module(package_dir: &Path) -> Result<(ModuleId, Vec<u8>)> {
    let package = BuildConfig::default().compile_package(package_dir, &mut Vec::new())?;
    let unit =
        package.root_modules().next().context("compiled package should contain a root module")?;
    let module = &unit.unit;
    let module_id = ModuleId::new(
        AccountAddress::new(module.address.into_bytes()),
        Identifier::new(module.name.to_string())?,
    );
    Ok((module_id, module.serialize(None)))
}
