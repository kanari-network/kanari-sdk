// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Genesis initialization for Kanari blockchain
//!
//! This module executes the framework bytecode embedded in the node binary to
//! initialize genesis state. It publishes modules in dependency order and calls
//! framework initializers through the Move VM.

use crate::ChangeSet;
use crate::move_runtime::{MoveRuntime, SupplyValidation, load_system_modules};
use crate::state::StateManager;
use anyhow::{Context, Result};
use kanari_types::address::Address as KanariAddress;
use kanari_types::hash::blake3;
use move_core_types::account_address::AccountAddress;

use std::format;

/// One-time framework initializer executed right after its module is published.
struct GenesisInit {
    /// Published module file that triggers this init (e.g. `"kanari.mv"`).
    file_name: String,
    /// Module name of the `init` function (e.g. `"kanari"`).
    module_name: &'static str,
    /// Pre-context entry arguments: empty witness encoding, or nothing.
    args: Vec<Vec<u8>>,
    /// Domain-separation seed for the init pseudo transaction hash.
    tx_hash_seed: &'static [u8],
}

/// `kanari::init(witness, ctx)` and `usd::init(witness, ctx)` run after their
/// modules are published; both take an empty witness encoding.
fn system_inits() -> Vec<GenesisInit> {
    vec![
        GenesisInit {
            file_name: format!("{}.mv", kanari_types::gas_coin::GasModule::GAS_MODULE),
            module_name: kanari_types::gas_coin::GasModule::GAS_MODULE,
            args: vec![Vec::new()],
            tx_hash_seed: b"KANARI::GENESIS::INIT::KANARI",
        },
        GenesisInit {
            file_name: format!("{}.mv", kanari_types::usd_coin::UsdModule::USD_MODULE),
            module_name: kanari_types::usd_coin::UsdModule::USD_MODULE,
            args: vec![Vec::new()],
            tx_hash_seed: b"KANARI::GENESIS::INIT::USD",
        },
    ]
}

/// `governance::init(ctx)` takes no witness, hence empty args.
fn governance_inits() -> Vec<GenesisInit> {
    vec![GenesisInit {
        file_name: format!("{}.mv", kanari_types::governance::GovernanceModule::GOVERNANCE_MODULE),
        module_name: kanari_types::governance::GovernanceModule::GOVERNANCE_MODULE,
        args: vec![],
        tx_hash_seed: b"KANARI::GENESIS::INIT::GOVERNANCE",
    }]
}

/// Initialize genesis state by executing framework modules from bytecode files
pub fn init_genesis(state: &mut StateManager) -> Result<()> {
    log::info!("=== Executing framework modules for genesis ===");

    log::info!("Loading the system framework bundle embedded in this node binary");

    // Build an in-memory runtime and preload framework/system natives.
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()
        .context("MoveRuntime initialization failed")?;
    log::info!("✓ MoveRuntime initialized with system modules");

    let system_addr = KanariAddress::kanari_system_account_address();

    // Discover framework modules and publish them in dependency order.
    let sorted_modules = load_system_modules::load_embedded_kanari_system_modules()
        .context("Failed to load embedded system modules")?;

    log::info!("Discovered {} framework modules", sorted_modules.len());
    log::info!("Publishing {} framework modules in dependency order", sorted_modules.len());

    let system_inits = system_inits();
    publish_framework_bundle(&runtime, state, system_addr, &sorted_modules, &system_inits)?;

    // Publish the governance framework (0x3) and run its `init(ctx)`, which
    // shares `GovernanceState` and transfers `GovernanceCap` to the sender.
    let governance_addr = KanariAddress::kanari_governance_account_address();
    let governance_modules = load_system_modules::load_embedded_kanari_governance_modules()
        .context("Failed to load embedded governance modules")?;
    log::info!("Publishing {} governance modules in dependency order", governance_modules.len());

    let governance_inits = governance_inits();
    publish_framework_bundle(
        &runtime,
        state,
        governance_addr,
        &governance_modules,
        &governance_inits,
    )?;

    log::info!("✓ All framework modules published");
    runtime
        .ensure_system_clock(state)
        .context("Failed to initialize system clock during genesis")?;
    log::info!("System clock initialized");
    runtime
        .ensure_deny_list(state)
        .context("Failed to initialize deny-list singleton during genesis")?;
    log::info!("Deny-list singleton initialized");
    log::info!("=== Genesis initialization complete ===");

    // Commit the accumulated genesis overlay to persistent storage.
    state.commit()?;

    Ok(())
}

/// Publish every module of a framework bundle, logging each change and running
/// the configured initializers after their module is published.
fn publish_framework_bundle(
    runtime: &MoveRuntime,
    state: &mut StateManager,
    addr: AccountAddress,
    modules: &[load_system_modules::DiscoveredModule],
    inits: &[GenesisInit],
) -> Result<()> {
    for (idx, module) in modules.iter().enumerate() {
        let module_name = &module.file_name;
        log::info!("[{}/{}] Publishing {}...", idx + 1, modules.len(), module_name);

        let changeset = match runtime.bootstrap_module_with_context_and_persistence(
            module.bytes.clone(),
            addr,
            None, // No gas info for genesis
            None, // No timestamp
            None, // No transaction hash
            true, // Persist genesis framework state
        ) {
            Ok(changeset) => changeset,
            Err(e) => {
                log::error!("   ❌ Failed to publish {}: {:?}", module_name, e);
                return Err(e).context(format!("Failed to publish module: {}", module_name));
            }
        };

        log_genesis_changeset(&changeset);

        // Persist state and object side effects for this module publish.
        runtime.commit_genesis_changeset(state, &changeset, SupplyValidation::Enforce)?;

        // Run the configured `init` once the triggering module is published.
        if let Some(init) = inits.iter().find(|i| i.file_name == *module_name) {
            execute_framework_init(
                runtime,
                state,
                addr,
                init.module_name,
                init.args.clone(),
                init.tx_hash_seed,
            )?;
        }
    }

    Ok(())
}

/// Log a genesis publish changeset for diagnostics.
fn log_genesis_changeset(changeset: &ChangeSet) {
    log::info!(
        "   ✓ Published successfully ({} objects created, {} events)",
        changeset.created_objects.len(),
        changeset.events.len()
    );

    // Log created objects for genesis diagnostics.
    if !changeset.created_objects.is_empty() {
        for (obj_id, obj) in &changeset.created_objects {
            log::info!(
                "      Created object: {} (type: {}, owner: {})",
                obj_id,
                obj.type_,
                obj.owner.to_hex_literal()
            );
        }
    }

    // Log emitted events for genesis diagnostics.
    if !changeset.events.is_empty() {
        for event in &changeset.events {
            log::info!(
                "      Event: type={}, seq={}, data_len={}",
                event.type_tag,
                event.sequence_number,
                event.event_data.len()
            );
        }
    }
}

/// Execute a framework module's `init(witness, ctx)` once at genesis.
///
/// `module_name` is both the published module and the witness type name
/// (e.g. `kanari`); `tx_hash_seed` domain-separates the init pseudo
/// transaction hash so replay tooling can attribute each init separately.
/// `args` holds the pre-context entry arguments: the empty witness encoding
/// for `init(witness, ctx)` modules, or nothing for plain `init(ctx)`.
fn execute_framework_init(
    runtime: &MoveRuntime,
    state: &mut StateManager,
    module_addr: move_core_types::account_address::AccountAddress,
    module_name: &str,
    args: Vec<Vec<u8>>,
    tx_hash_seed: &[u8],
) -> Result<()> {
    log::info!("   Executing {module_name}::init() via Move VM...");

    let move_module_addr = module_addr;

    let init_tx_hash = blake3(tx_hash_seed).to_vec();

    match runtime.execute_init_function_with_context(
        move_module_addr,
        module_name,
        args,
        Some(0),
        Some(init_tx_hash),
    ) {
        Ok(init_changeset) => {
            log::info!(
                "   ✓ {module_name}::init() executed ({} objects created, {} events)",
                init_changeset.created_objects.len(),
                init_changeset.events.len()
            );

            // Log created objects for genesis diagnostics.
            for (obj_id, obj) in &init_changeset.created_objects {
                log::info!(
                    "      Created object: {} (type: {}, owner: {})",
                    obj_id,
                    obj.type_,
                    obj.owner.to_hex_literal()
                );
            }

            // Log emitted events for genesis diagnostics.
            if !init_changeset.events.is_empty() {
                for event in &init_changeset.events {
                    log::info!(
                        "      Event: type={}, seq={}",
                        event.type_tag,
                        event.sequence_number
                    );
                }
            }

            runtime.commit_genesis_changeset(state, &init_changeset, SupplyValidation::Enforce)?;
            Ok(())
        }
        Err(e) => {
            log::error!("   ❌ Failed to execute {module_name}::init(): {:?}", e);
            Err(e).context(format!("{module_name}::init() execution failed"))
        }
    }
}
