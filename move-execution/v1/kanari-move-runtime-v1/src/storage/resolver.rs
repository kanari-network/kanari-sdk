// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::storage::move_vm_state::MoveVMState;
use crate::storage::object_storage::ObjectStore;
use anyhow::Result;
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::{ModuleId, StructTag};
use move_core_types::resolver::{LinkageResolver, ModuleResolver, ResourceResolver};
use std::sync::Arc;

/// A resolver that fetches modules and resources directly from persistent storage.
/// This enables "Stateless" execution (Sui-like) without pre-loading everything into memory.
#[derive(Clone)]
pub struct KanariMoveResolver {
    pub(crate) state: MoveVMState,
    pub(crate) _object_storage: Arc<dyn ObjectStore>,
}

impl KanariMoveResolver {
    pub fn new(state: MoveVMState, object_storage: Arc<dyn ObjectStore>) -> Self {
        Self {
            state,
            _object_storage: object_storage,
        }
    }
}

impl LinkageResolver for KanariMoveResolver {
    type Error = anyhow::Error;
}

impl ModuleResolver for KanariMoveResolver {
    type Error = anyhow::Error;

    fn get_module(&self, id: &ModuleId) -> Result<Option<Vec<u8>>, Self::Error> {
        // Fetch module directly from RocksDB via MoveVMState
        Ok(self.state.get_module(id))
    }
}

impl ResourceResolver for KanariMoveResolver {
    type Error = anyhow::Error;

    fn get_resource(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        if let Some(data) = self.state.get_resource(address, tag) {
            return Ok(Some(data));
        }

        if let Some(data) = self.state.get_object(address) {
            log::debug!("[RESOLVER] Loaded Sui-style object at {}", address);
            return Ok(Some(data));
        }

        Ok(None)
    }
}
