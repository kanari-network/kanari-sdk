// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Object changes, effects, and recorded writes on a changeset.

use crate::changeset::{ChangeSet, CreatedObject};
use kanari_crypto::hash_data_blake3;
use kanari_types::balance::BalanceRecord;
use kanari_types::coin::{CoinModule, TreasuryCap};
use kanari_types::event::Event;
use kanari_types::object::{IDRecord, UIDRecord};
use kanari_types::transaction::{
    GasPayment, ObjectChange, ObjectChangeKind, ObjectGraphEdge, ObjectGraphEdgeKind, ObjectInput,
    ObjectOwnerKind, ObjectRef, TransactionEffects,
};
use move_core_types::account_address::AccountAddress;

impl ChangeSet {
    fn input_edge_relation(change_type: &ObjectChangeKind) -> ObjectGraphEdgeKind {
        match change_type {
            ObjectChangeKind::Created => ObjectGraphEdgeKind::InputCreate,
            ObjectChangeKind::Mutated => ObjectGraphEdgeKind::InputMutate,
            ObjectChangeKind::Deleted => ObjectGraphEdgeKind::InputDelete,
            ObjectChangeKind::Transferred => ObjectGraphEdgeKind::InputTransfer,
        }
    }

    fn shared_input_edge_relation(change_type: &ObjectChangeKind) -> ObjectGraphEdgeKind {
        match change_type {
            ObjectChangeKind::Created => ObjectGraphEdgeKind::SharedInputCreate,
            ObjectChangeKind::Mutated => ObjectGraphEdgeKind::SharedInputMutate,
            ObjectChangeKind::Deleted => ObjectGraphEdgeKind::SharedInputDelete,
            ObjectChangeKind::Transferred => ObjectGraphEdgeKind::SharedInputTransfer,
        }
    }

    fn immutable_input_edge_relation(change_type: &ObjectChangeKind) -> ObjectGraphEdgeKind {
        match change_type {
            ObjectChangeKind::Created => ObjectGraphEdgeKind::ImmutableInputCreate,
            ObjectChangeKind::Mutated => ObjectGraphEdgeKind::ImmutableInputMutate,
            ObjectChangeKind::Deleted => ObjectGraphEdgeKind::ImmutableInputDelete,
            ObjectChangeKind::Transferred => ObjectGraphEdgeKind::ImmutableInputTransfer,
        }
    }

    fn gas_edge_relation(change_type: &ObjectChangeKind) -> ObjectGraphEdgeKind {
        match change_type {
            ObjectChangeKind::Created => ObjectGraphEdgeKind::GasCreate,
            ObjectChangeKind::Mutated => ObjectGraphEdgeKind::GasMutate,
            ObjectChangeKind::Deleted => ObjectGraphEdgeKind::GasDelete,
            ObjectChangeKind::Transferred => ObjectGraphEdgeKind::GasTransfer,
        }
    }

    /// Appends an event to this ChangeSet.
    pub fn add_event(&mut self, event: Event) {
        self.events.push(event);
    }

    /// Records a Move module/resource write or deletion.
    pub fn record_move_write(&mut self, key: Vec<u8>, value: Option<Vec<u8>>) {
        self.move_writes.insert(key, value);
    }

    /// Records an object deletion by its canonical ID.
    pub fn add_deleted_object(&mut self, object_id: String) {
        self.deleted_objects
            .push(Self::canonicalize_object_id(&object_id));
    }

    /// Records a treasury cap creation or update for a token type.
    pub fn add_treasury(&mut self, owner: AccountAddress, token_type: String, total_supply: u64) {
        self.treasuries
            .push((owner, token_type, TreasuryCap { total_supply }));
    }

    /// Record an absolute token balance for a given account/token pair. Multiple coin
    /// fragments observed in one VM session are summed to one absolute owner total.
    pub fn add_token_balance_set(
        &mut self,
        owner: AccountAddress,
        token_type: String,
        amount: u64,
    ) {
        if let Some((_, _, existing_balance)) = self
            .token_balance_sets
            .iter_mut()
            .find(|(o, t, _)| o == &owner && t == &token_type)
        {
            *existing_balance = BalanceRecord::new(existing_balance.value().saturating_add(amount));
        } else {
            self.token_balance_sets
                .push((owner, token_type, BalanceRecord::new(amount)));
        }
    }

    /// Records a newly created object with owner, type, data, and optional UID/ID records.
    pub fn add_created_object(
        &mut self,
        owner: AccountAddress,
        type_: String,
        data: Vec<u8>,
        version: u64,
        uid: Option<UIDRecord>,
        id: Option<IDRecord>,
        object_id: Option<String>,
    ) {
        let canonical_id = if let Some(id) = &object_id {
            Self::canonicalize_object_id(id)
        } else if let Some(ref u) = uid {
            u.address().to_hex_literal()
        } else if let Some(ref i) = id {
            i.address().to_hex_literal()
        } else {
            let mut input = Vec::new();
            input.extend_from_slice(owner.as_ref());
            input.extend_from_slice(type_.as_bytes());
            let hash = hash_data_blake3(&input);
            format!("0x{}", hex::encode(&hash[0..32]))
        };

        if let Some((_, existing_obj)) = self
            .created_objects
            .iter_mut()
            .find(|(id, _)| id == &canonical_id)
        {
            if owner.to_hex_literal() != canonical_id {
                existing_obj.owner = owner;
            }
            existing_obj.data = data;
            existing_obj.version = version;
            existing_obj.type_ = type_;
            if let Some(u) = uid {
                existing_obj.uid = Some(u);
            }
            if let Some(i) = id {
                existing_obj.id = Some(i);
            }
        } else {
            self.created_objects.push((
                canonical_id,
                CreatedObject {
                    owner,
                    uid,
                    id,
                    owner_kind: ObjectOwnerKind::AddressOwner(owner.to_hex_literal()),
                    type_,
                    data,
                    version,
                },
            ));
        }
    }

    /// Computes the list of object changes from created and deleted objects.
    pub fn object_changes(&self) -> Vec<ObjectChange> {
        if !self.explicit_object_changes.is_empty() {
            return self.explicit_object_changes.clone();
        }
        let mut changes = Vec::new();

        for (object_id, created) in &self.created_objects {
            // NOTE: version <= 1 is a heuristic to distinguish Created vs Mutated.
            // Objects created by parse_move_changeset start at version 0.
            // Objects created by upsert_created_object use existing.version + 1.
            // A re-created object after deletion would start at version 1 again,
            // but this is extremely rare in practice. A proper fix would add an
            // explicit `is_new` flag to CreatedObject.
            let change_type = if created.version <= 1 {
                ObjectChangeKind::Created
            } else {
                ObjectChangeKind::Mutated
            };

            // New coins carry what the recipient received; mutated
            // remainders carry no transferred amount.
            let amount = match change_type {
                ObjectChangeKind::Created => {
                    CoinModule::coin_balance_from_object(&created.type_, &created.data)
                }
                _ => None,
            };
            changes.push(ObjectChange {
                change_type: change_type.clone(),
                object_ref: created.object_ref(object_id),
                previous_object_ref: match change_type {
                    ObjectChangeKind::Created => None,
                    _ => Some(ObjectRef::new(
                        object_id.clone(),
                        created.version.checked_sub(1),
                        None,
                    )),
                },
                type_: Some(created.type_.clone()),
                owner: Some(created.owner_kind()),
                previous_owner: None,
                previous_version: match change_type {
                    ObjectChangeKind::Created => None,
                    _ => created.version.checked_sub(1),
                },
                amount,
            });
        }

        for object_id in &self.deleted_objects {
            changes.push(ObjectChange {
                change_type: ObjectChangeKind::Deleted,
                object_ref: ObjectRef::new(object_id.clone(), None, None),
                previous_object_ref: None,
                type_: None,
                owner: None,
                previous_owner: None,
                previous_version: None,
                amount: None,
            });
        }

        changes
    }

    /// Builds the full TransactionEffects from this ChangeSet.
    pub fn effects(&self, gas_payment: Option<GasPayment>) -> TransactionEffects {
        let object_changes = self.object_changes();
        let mut created = Vec::new();
        let mut mutated = Vec::new();
        let mut deleted = Vec::new();
        let mut transferred = Vec::new();
        let mut causal_edges = Vec::new();
        let input_refs = self
            .input_objects
            .iter()
            .map(|input| input.object_ref.clone())
            .collect::<Vec<_>>();

        for change in &object_changes {
            match change.change_type {
                ObjectChangeKind::Created => created.push(change.clone()),
                ObjectChangeKind::Mutated => mutated.push(change.clone()),
                ObjectChangeKind::Deleted => deleted.push(change.clone()),
                ObjectChangeKind::Transferred => transferred.push(change.clone()),
            }

            if let Some(previous_object_ref) = &change.previous_object_ref {
                causal_edges.push(ObjectGraphEdge {
                    source_object_ref: previous_object_ref.clone(),
                    target_object_ref: change.object_ref.clone(),
                    relation: if matches!(change.change_type, ObjectChangeKind::Deleted) {
                        ObjectGraphEdgeKind::Delete
                    } else {
                        ObjectGraphEdgeKind::VersionSuccessor
                    },
                });

                if matches!(change.change_type, ObjectChangeKind::Transferred)
                    && change.previous_owner != change.owner
                {
                    causal_edges.push(ObjectGraphEdge {
                        source_object_ref: previous_object_ref.clone(),
                        target_object_ref: change.object_ref.clone(),
                        relation: ObjectGraphEdgeKind::OwnershipTransfer,
                    });
                }
            }

            for input in &self.input_objects {
                causal_edges.push(ObjectGraphEdge {
                    source_object_ref: input.object_ref.clone(),
                    target_object_ref: change.object_ref.clone(),
                    relation: Self::input_edge_relation(&change.change_type),
                });
            }
            for input in &self.shared_inputs {
                causal_edges.push(ObjectGraphEdge {
                    source_object_ref: input.clone(),
                    target_object_ref: change.object_ref.clone(),
                    relation: Self::shared_input_edge_relation(&change.change_type),
                });
            }
            for input in &self.immutable_inputs {
                causal_edges.push(ObjectGraphEdge {
                    source_object_ref: input.clone(),
                    target_object_ref: change.object_ref.clone(),
                    relation: Self::immutable_input_edge_relation(&change.change_type),
                });
            }
            for input in &self.gas_object_refs {
                causal_edges.push(ObjectGraphEdge {
                    source_object_ref: input.clone(),
                    target_object_ref: change.object_ref.clone(),
                    relation: Self::gas_edge_relation(&change.change_type),
                });
                if matches!(change.change_type, ObjectChangeKind::Created) {
                    causal_edges.push(ObjectGraphEdge {
                        source_object_ref: input.clone(),
                        target_object_ref: change.object_ref.clone(),
                        relation: ObjectGraphEdgeKind::CallContextCreate,
                    });
                }
            }
        }

        for gas_ref in &self.gas_object_refs {
            for input_ref in &input_refs {
                causal_edges.push(ObjectGraphEdge {
                    source_object_ref: gas_ref.clone(),
                    target_object_ref: input_ref.clone(),
                    relation: ObjectGraphEdgeKind::GasMutate,
                });
            }
            for input_ref in &self.shared_inputs {
                causal_edges.push(ObjectGraphEdge {
                    source_object_ref: gas_ref.clone(),
                    target_object_ref: input_ref.clone(),
                    relation: ObjectGraphEdgeKind::GasMutate,
                });
            }
            for input_ref in &self.immutable_inputs {
                causal_edges.push(ObjectGraphEdge {
                    source_object_ref: gas_ref.clone(),
                    target_object_ref: input_ref.clone(),
                    relation: ObjectGraphEdgeKind::GasMutate,
                });
            }
        }
        causal_edges.sort_by(|a, b| {
            (
                a.source_object_ref.object_id.as_str(),
                a.target_object_ref.object_id.as_str(),
                format!("{:?}", a.relation),
            )
                .cmp(&(
                    b.source_object_ref.object_id.as_str(),
                    b.target_object_ref.object_id.as_str(),
                    format!("{:?}", b.relation),
                ))
        });
        causal_edges.dedup();

        TransactionEffects {
            status: if self.success {
                "success".to_string()
            } else {
                "failed".to_string()
            },
            gas_used: self.gas_used,
            gas_payment: gas_payment.or_else(|| self.gas_payment.clone()),
            input_objects: self
                .input_objects
                .iter()
                .map(|input| input.object_ref.clone())
                .collect(),
            shared_inputs: self.shared_inputs.clone(),
            immutable_inputs: self.immutable_inputs.clone(),
            gas_object_refs: self.gas_object_refs.clone(),
            object_changes,
            created,
            mutated,
            deleted,
            transferred,
            causal_edges,
            error_message: self.error_message.clone(),
        }
    }

    /// Sets input object references, gas payment, and partitions shared/immutable inputs.
    pub fn set_transaction_context(
        &mut self,
        object_inputs: Vec<ObjectInput>,
        gas_payment: Option<GasPayment>,
    ) {
        self.input_objects = object_inputs.clone();
        self.shared_inputs = object_inputs
            .iter()
            .filter(|input| matches!(input.owner, Some(ObjectOwnerKind::Shared)))
            .map(|input| input.object_ref.clone())
            .collect();
        self.immutable_inputs = object_inputs
            .iter()
            .filter(|input| matches!(input.owner, Some(ObjectOwnerKind::Immutable)))
            .map(|input| input.object_ref.clone())
            .collect();
        self.gas_object_refs = gas_payment
            .as_ref()
            .map(|payment| payment.payment_objects.clone())
            .unwrap_or_default();
        self.gas_payment = gas_payment;
    }

    /// Replaces the explicit object changes with the provided list.
    pub fn set_explicit_object_changes(&mut self, object_changes: Vec<ObjectChange>) {
        self.explicit_object_changes = object_changes;
    }
}
