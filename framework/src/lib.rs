//! Kanari Framework
//! Core framework implementation for the Kanari blockchain

// Core type imports
use mona_types::{
    address::Address,
    object::{ID, UID},
    coin::Coin,
};

// Re-export core dependencies
pub use mona_types;
pub use move_binary_format;
pub use move_core_types;

// Optional Move VM features
#[cfg(feature = "move-vm")]
pub use {
    move_bytecode_utils,
    move_command_line_common,
    move_vm_test_utils,
    move_vm_types,
    move_vm_profiler,
};



