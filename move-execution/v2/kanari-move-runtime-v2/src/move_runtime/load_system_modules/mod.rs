// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Load and sort system modules from a directory
//!
//! This module provides functionality to load Move modules from a specified directory, sort them based on their dependencies, and verify their integrity. It is used to load system modules such as the Move standard library and Kanari system modules into the Move runtime. The module ensures that the loaded modules are compatible with the existing state and handles any necessary upgrades or replacements of existing modules.
//!
//! The main functions provided in this module are:
//! - `load_move_stdlib`: Loads the Move standard library modules (0x1::*).
//! - `load_kanari_system`: Loads the Kanari system modules (0x2::*).
//! - `load_kanari_governance`: Loads the Kanari governance modules (0x3::*).
//!
//! The module also includes utilities for discovering modules in a directory, verifying their dependencies, and computing a manifest and hash for the loaded framework modules. It ensures that the loaded modules are in a valid state and compatible with the existing runtime environment.
//!
//! The module also handles scenarios where the modules on disk may be in test mode and rebuilds them in production mode if necessary. It provides warnings and error handling for incompatible upgrades and ensures that all validators in a network upgrade together to maintain consensus.
//!
//! The module is designed to be used in conjunction with the `MoveRuntime` struct, which manages the execution of Move code and maintains the state of the blockchain. It provides a structured approach to loading and managing system modules, ensuring that the runtime environment is consistent and secure.
//!
//! load_move_stdlib and load_kanari_system are the main entry points for loading the respective frameworks. They handle the discovery of modules, verification of dependencies, and saving of modules to the runtime state. The module also provides functionality to prune old or unused modules from the runtime state to maintain a clean and efficient environment.
//!
//! The module is part of the Kanari Move runtime and is intended to be used in a blockchain context where Move modules are executed and managed. It provides a robust and reliable way to load and manage system modules, ensuring that the runtime environment is secure, consistent, and compatible with the existing state of the blockchain.
//! The module is designed to be used in a blockchain context where Move modules are executed and managed. It provides a robust and reliable way to load and manage system modules, ensuring that the runtime environment is secure, consistent, and compatible with the existing state of the blockchain.

mod discovery;
mod framework;

pub(crate) use discovery::*;
pub(crate) use framework::*;

include!(concat!(env!("OUT_DIR"), "/embedded_framework_modules.rs"));
