// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Core blockchain engine for the Kanari network.
//!
//! `kanari-core` provides the central [`BlockchainEngine`] that ties together
//! the Move VM runtime, persistent state management, transaction mempool,
//! parallel execution, and DAG-based consensus. It also exports the
//! [`blockchain::Blockchain`] state machine, [`Checkpoint`] consensus types,
//! and atomic file-I/O utilities.

pub mod blockchain;
pub mod consensus;
pub mod engine;
pub mod file_io;

pub use consensus::Checkpoint;
pub use engine::{
    BlockchainEngine, CheckpointInfo, CheckpointProductionInfo, CheckpointSyncData,
    DagProductionPolicy, DagVertex, decode_hex_exact, normalize_consensus_authority_id,
};
pub use file_io::{read_json_file, write_file_atomically, write_json_pretty_atomically};
pub use kanari_rpc_api::{BlockData, BlockchainStats, FullBlockData};

pub use kanari_move_runtime_v1;
