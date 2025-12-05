use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Pending object operations collected during Move VM execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PendingObjectOps {
    pub transfers: Vec<ObjectTransfer>,
    pub freezes: Vec<ObjectFreeze>,
    pub shares: Vec<ObjectShare>,
}

/// Object transfer operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectTransfer {
    pub object_id: Vec<u8>,
    pub object_type: String,
    pub object_data: Vec<u8>,
    pub recipient: AccountAddress,
}

/// Object freeze operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectFreeze {
    pub object_id: Vec<u8>,
    pub object_type: String,
    pub object_data: Vec<u8>,
}

/// Object share operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectShare {
    pub object_id: Vec<u8>,
    pub object_type: String,
    pub object_data: Vec<u8>,
}

impl PendingObjectOps {
    pub fn new() -> Self {
        Self {
            transfers: Vec::new(),
            freezes: Vec::new(),
            shares: Vec::new(),
        }
    }

    pub fn add_transfer(&mut self, transfer: ObjectTransfer) {
        self.transfers.push(transfer);
    }

    pub fn add_freeze(&mut self, freeze: ObjectFreeze) {
        self.freezes.push(freeze);
    }

    pub fn add_share(&mut self, share: ObjectShare) {
        self.shares.push(share);
    }

    pub fn is_empty(&self) -> bool {
        self.transfers.is_empty() && self.freezes.is_empty() && self.shares.is_empty()
    }
}

/// Thread-safe wrapper for pending object operations
pub type PendingObjectOpsRef = Arc<Mutex<PendingObjectOps>>;

/// Create new pending ops reference
pub fn new_pending_ops() -> PendingObjectOpsRef {
    Arc::new(Mutex::new(PendingObjectOps::new()))
}
