// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Genesis initialization for Kanari blockchain
//!
//! This module executes the actual framework bytecode to initialize genesis state.
//! It loads compiled .mv files from disk, publishes all modules in dependency order,
//! and calls kanari::init() to mint initial coin supply through Move VM.

use crate::move_runtime::{MoveRuntime, load_system_modules};
use crate::state::StateManager;
use anyhow::{Context, Result};
use kanari_types::address::Address as KanariAddress;

use std::format;
use std::path::PathBuf;

/// Path to compiled framework bytecode directory (relative to workspace root)
const FRAMEWORK_BYTECODE_DIR: &str =
    "crates/kanari-frameworks/packages/kanari-system/build/KanariSystem/bytecode_modules";

/// Initialize genesis state by executing framework modules from bytecode files
pub fn init_genesis(state: &mut StateManager) -> Result<()> {
    log::info!("=== Executing framework modules for genesis ===");

    // Find the framework bytecode directory
    let workspace_root = find_workspace_root()?;
    let bytecode_dir = workspace_root.join(FRAMEWORK_BYTECODE_DIR);

    if !bytecode_dir.exists() {
        anyhow::bail!(
            "Framework bytecode directory not found at {}. Please run 'kanari move build' first.",
            bytecode_dir.display()
        );
    }

    log::info!("Loading framework modules from: {}", bytecode_dir.display());

    // Create MoveRuntime with Kanari natives and system modules pre-loaded
    let runtime = MoveRuntime::new_with_kanari_natives_in_memory()
        .context("MoveRuntime initialization failed")?;
    log::info!("✓ MoveRuntime initialized with system modules");

    let system_addr = KanariAddress::kanari_system_account_address();

    // Use load_system_modules to discover and sort modules
    let sorted_modules = load_system_modules::load_system_modules_from_dir(&bytecode_dir)
        .context("Failed to load system modules")?;

    log::info!("Discovered {} framework modules", sorted_modules.len());
    log::info!(
        "Publishing {} framework modules in dependency order",
        sorted_modules.len()
    );

    // Publish each module
    for (idx, module) in sorted_modules.iter().enumerate() {
        let module_name = &module.file_name;
        log::info!(
            "[{}/{}] Publishing {}...",
            idx + 1,
            sorted_modules.len(),
            module_name
        );

        match runtime.publish_module(
            module.bytes.clone(),
            system_addr,
            None, // No gas info for genesis
            None, // No timestamp
        ) {
            Ok(changeset) => {
                log::info!(
                    "   ✓ Published successfully ({} objects created, {} events)",
                    changeset.created_objects.len(),
                    changeset.events.len()
                );

                // Log details of created objects for debugging
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

                // Log events if any
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

                // Apply changeset to state
                state.apply_changeset(&changeset)?;
                runtime.persist_created_objects(&changeset);
                runtime.persist_deleted_objects(&changeset);

                // If this is the kanari module, execute its init function via Move VM
                if *module_name == "kanari.mv" {
                    log::info!("   Executing kanari::init() via Move VM...");

                    use move_core_types::account_address::AccountAddress as MoveAccountAddress;

                    let move_system_addr = MoveAccountAddress::from_hex_literal(
                        system_addr.to_hex_literal().as_str(),
                    )?;

                    let witness_bytes = Vec::new();

                    match runtime.execute_init_function(
                        move_system_addr,
                        "kanari",
                        vec![witness_bytes],
                    ) {
                        Ok(init_changeset) => {
                            log::info!(
                                "   ✓ kanari::init() executed ({} objects created, {} events)",
                                init_changeset.created_objects.len(),
                                init_changeset.events.len()
                            );

                            // Log details of created objects for debugging
                            for (obj_id, obj) in &init_changeset.created_objects {
                                log::info!(
                                    "      Created object: {} (type: {}, owner: {})",
                                    obj_id,
                                    obj.type_,
                                    obj.owner.to_hex_literal()
                                );
                            }

                            // Log events if any
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
                            runtime.persist_created_objects(&init_changeset);
                            runtime.persist_deleted_objects(&init_changeset);
                        }
                        Err(e) => {
                            log::error!("   ❌ Failed to execute kanari::init(): {:?}", e);
                            return Err(e).context("kanari::init() execution failed");
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("   ❌ Failed to publish {}: {:?}", module_name, e);
                return Err(e).context(format!("Failed to publish module: {}", module_name));
            }
        }
    }

    log::info!("✓ All framework modules published");
    log::info!("=== Genesis initialization complete ===");

    // Commit to persist all genesis state
    state.commit()?;

    Ok(())
}

/// Find the workspace root by looking for Cargo.toml or similar markers
fn find_workspace_root() -> Result<PathBuf> {
    // Start from current directory and walk up
    let mut path = std::env::current_dir()?;
    loop {
        if path.join("Cargo.toml").exists() {
            return Ok(path);
        }
        if !path.pop() {
            break;
        }
    }
    // Fallback to current directory if not found
    Ok(std::env::current_dir()?)
}
