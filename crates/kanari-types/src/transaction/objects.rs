// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectOwnerKind {
    AddressOwner(String),
    Shared,
    Immutable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectRef {
    pub object_id: String,
    pub version: Option<u64>,
    pub digest: Option<String>,
}

impl ObjectRef {
    pub fn new(object_id: impl Into<String>, version: Option<u64>, digest: Option<String>) -> Self {
        Self {
            object_id: object_id.into(),
            version,
            digest,
        }
    }

    pub fn has_full_metadata(&self) -> bool {
        self.version.is_some() && self.digest.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectInput {
    pub object_ref: ObjectRef,
    pub owner: Option<ObjectOwnerKind>,
    pub mutable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GasPayment {
    pub payment_objects: Vec<ObjectRef>,
    pub owner: String,
    pub budget: u64,
    pub price: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishedModule {
    pub module_name: String,
    pub module_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectChangeKind {
    Created,
    Mutated,
    Deleted,
    Transferred,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectGraphEdgeKind {
    InputCreate,
    InputMutate,
    InputDelete,
    InputTransfer,
    SharedInputCreate,
    SharedInputMutate,
    SharedInputDelete,
    SharedInputTransfer,
    ImmutableInputCreate,
    ImmutableInputMutate,
    ImmutableInputDelete,
    ImmutableInputTransfer,
    GasCreate,
    GasMutate,
    GasDelete,
    GasTransfer,
    CallContextCreate,
    VersionSuccessor,
    Delete,
    OwnershipTransfer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectGraphEdge {
    pub source_object_ref: ObjectRef,
    pub target_object_ref: ObjectRef,
    pub relation: ObjectGraphEdgeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectChange {
    pub change_type: ObjectChangeKind,
    pub object_ref: ObjectRef,
    pub previous_object_ref: Option<ObjectRef>,
    pub type_: Option<String>,
    pub owner: Option<ObjectOwnerKind>,
    pub previous_owner: Option<ObjectOwnerKind>,
    pub previous_version: Option<u64>,
    /// Coin balance carried by this object after the change, in base units.
    /// Set only for `Created` / `Transferred` coin objects (what the
    /// recipient actually received). Always `None` for `Mutated` (the
    /// balance there is a remainder, not a transferred amount) and for
    /// non-coin objects. `None` also covers history written before this
    /// field existed — clients must render those as amount unknown.
    ///
    /// NOTE: `default` without `skip_serializing_if` is load-bearing:
    /// `ObjectChange` is BCS-persisted in checkpoints, and BCS is not
    /// self-describing — skipping `None` on write would misalign every
    /// subsequent field on read.
    #[serde(default)]
    pub amount: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionEffects {
    pub status: String,
    pub gas_used: u64,
    pub gas_payment: Option<GasPayment>,
    #[serde(default)]
    pub input_objects: Vec<ObjectRef>,
    #[serde(default)]
    pub shared_inputs: Vec<ObjectRef>,
    #[serde(default)]
    pub immutable_inputs: Vec<ObjectRef>,
    #[serde(default)]
    pub gas_object_refs: Vec<ObjectRef>,
    pub object_changes: Vec<ObjectChange>,
    #[serde(default)]
    pub created: Vec<ObjectChange>,
    #[serde(default)]
    pub mutated: Vec<ObjectChange>,
    #[serde(default)]
    pub deleted: Vec<ObjectChange>,
    #[serde(default)]
    pub transferred: Vec<ObjectChange>,
    #[serde(default)]
    pub causal_edges: Vec<ObjectGraphEdge>,
    pub error_message: Option<String>,
}
