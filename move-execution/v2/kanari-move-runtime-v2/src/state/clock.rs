// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! System clock object id pointer.

use super::*;

impl StateManager {
    /// Load the configured system clock object ID.
    pub(crate) fn get_system_clock_object_id(&self) -> Result<Option<AccountAddress>> {
        let bytes_opt: Option<Vec<u8>> = self.load_internal(SYSTEM_CLOCK_OBJECT_ID_KEY)?;
        match bytes_opt {
            None => Ok(None),
            Some(bytes) => {
                let addr = AccountAddress::from_bytes(bytes)?;
                Ok(Some(addr))
            }
        }
    }

    /// Set the system clock object ID.
    pub(crate) fn set_system_clock_object_id(&mut self, id: AccountAddress) -> Result<()> {
        self.save_internal(SYSTEM_CLOCK_OBJECT_ID_KEY, &id.as_ref().to_vec())
    }
}
