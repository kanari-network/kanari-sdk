// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Genesis initialization for Kanari blockchain
//!
//! This module executes the framework bytecode embedded in the node binary to
//! initialize genesis state. It publishes modules in dependency order and calls
//! framework initializers through the Move VM.

use crate::move_runtime::{MoveRuntime, load_system_modules};
use crate::state::StateManager;
use anyhow::{Context, Result};
use kanari_crypto::hash_data_blake3;
use kanari_types::address::Address as KanariAddress;

use std::format;

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
    log::info!(
        "Publishing {} framework modules in dependency order",
        sorted_modules.len()
    );

    // Publish each module and apply the resulting changeset immediately.
    for (idx, module) in sorted_modules.iter().enumerate() {
        let module_name = &module.file_name;
        log::info!(
            "[{}/{}] Publishing {}...",
            idx + 1,
            sorted_modules.len(),
            module_name
        );

        match runtime.bootstrap_module_with_context_and_persistence(
            module.bytes.clone(),
            system_addr,
            None, // No gas info for genesis
            None, // No timestamp
            None, // No transaction hash
            true, // Persist genesis framework state
        ) {
            Ok(changeset) => {
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

                // Persist state and object side effects for this module publish.
                state.apply_changeset(&changeset)?;
                runtime.persist_created_objects(&changeset)?;
                runtime.persist_deleted_objects(&changeset)?;

                // Run `kanari::init()` after publishing the main kanari module,
                // then `usd::init()` to mint the USD stablecoin genesis supply.
                // Both must run before the system clock prologue commits.
                if *module_name == "kanari.mv" {
                    execute_framework_init(
                        &runtime,
                        state,
                        system_addr,
                        "kanari",
                        b"KANARI::GENESIS::INIT::KANARI",
                    )?;
                }
                if *module_name == "usd.mv" {
                    execute_framework_init(
                        &runtime,
                        state,
                        system_addr,
                        "usd",
                        b"KANARI::GENESIS::INIT::USD",
                    )?;
                }
            }
            Err(e) => {
                log::error!("   ❌ Failed to publish {}: {:?}", module_name, e);
                return Err(e).context(format!("Failed to publish module: {}", module_name));
            }
        }
    }

    log::info!("✓ All framework modules published");
    runtime
        .ensure_system_clock(state)
        .context("Failed to initialize system clock during genesis")?;
    log::info!("System clock initialized");
    log::info!("=== Genesis initialization complete ===");

    // Commit the accumulated genesis overlay to persistent storage.
    state.commit()?;

    Ok(())
}

/// Execute a framework module's `init(witness, ctx)` once at genesis.
///
/// `module_name` is both the published module and the witness type name
/// (e.g. `kanari`); `tx_hash_seed` domain-separates the init pseudo
/// transaction hash so replay tooling can attribute each init separately.
fn execute_framework_init(
    runtime: &MoveRuntime,
    state: &mut StateManager,
    system_addr: move_core_types::account_address::AccountAddress,
    module_name: &str,
    tx_hash_seed: &[u8],
) -> Result<()> {
    log::info!("   Executing {module_name}::init() via Move VM...");

    let move_system_addr = system_addr;

    let witness_bytes = Vec::new();
    let init_tx_hash = hash_data_blake3(tx_hash_seed).to_vec();

    match runtime.execute_init_function_with_context(
        move_system_addr,
        module_name,
        vec![witness_bytes],
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

            state.apply_changeset(&init_changeset)?;
            runtime.persist_created_objects(&init_changeset)?;
            runtime.persist_deleted_objects(&init_changeset)?;
            Ok(())
        }
        Err(e) => {
            log::error!("   ❌ Failed to execute {module_name}::init(): {:?}", e);
            Err(e).context(format!("{module_name}::init() execution failed"))
        }
    }
}
