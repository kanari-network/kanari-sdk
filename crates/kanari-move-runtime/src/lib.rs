// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod changeset;
pub mod contract;

pub mod gas;
pub mod move_runtime;

pub mod state;
pub mod storage;

// #[cfg(test)]
// mod tests;

pub use changeset::{AccountChange, ChangeSet};
pub use contract::{
    ContractABI, ContractCall, ContractDeployment, ContractInfo, ContractMetadata,
    ContractRegistry, FieldInfo, FunctionSignature, ParameterInfo, StructSignature,
};

pub use gas::{GasConfig, GasError, GasEstimate, GasMeter, GasOperation, TransactionGas};
pub use kanari_crypto::keys::CurveType;
#[deprecated(note = "Use `runtime::MoveRuntime` instead")]
pub use move_runtime::MoveRuntime;
#[deprecated(note = "Use `runtime::RuntimeStats` instead")]
pub use move_runtime::move_runtime_extensions::RuntimeStats;

/// Grouped runtime exports. Prefer these paths for new code.
pub mod runtime {
    pub use crate::move_runtime::MoveRuntime;
    pub use crate::move_runtime::move_runtime_extensions::RuntimeStats;
}
pub use state::{Account, StateManager};
pub use storage::move_vm_state::MoveVMState;
