// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Parser functions for Move VM changesets and events
use crate::changeset::ChangeSet;
use move_core_types::effects::Op as MoveOp;

impl super::MoveRuntime {
    /// Parse Move VM ChangeSet and extract state changes into Kanari ChangeSet
    /// This converts Move VM's canonical state changes into our domain model
    pub(crate) fn parse_move_changeset(
        &self,
        move_cs: &move_core_types::effects::ChangeSet,
        kanari_cs: &mut ChangeSet,
    ) {
        for (addr, account_changes) in move_cs.accounts() {
            // Process module changes
            for (module_name, op) in account_changes.modules() {
                match op {
                    MoveOp::New(_bytes) | MoveOp::Modify(_bytes) => {
                        // Module published or updated
                        kanari_cs.publish_module(*addr, module_name.to_string());
                    }
                    MoveOp::Delete => {
                        // Module deletion (rare, but possible)
                        eprintln!(
                            "Warning: Module deletion detected for {}::{}",
                            addr, module_name
                        );
                    }
                }
            }

            // Process resource changes
            for (struct_tag, op) in account_changes.resources() {
                match op {
                    MoveOp::New(bytes) | MoveOp::Modify(bytes) => {
                        // Parse resource bytes and set per-account token balances when applicable
                        if self.is_balance_resource(struct_tag) {
                            if let Some(amount) = self.extract_balance_from_bytes(bytes, struct_tag)
                            {
                                if let Some(token_type) =
                                    self.token_type_from_struct_tag(struct_tag)
                                {
                                    // Record absolute token balance for this account
                                    kanari_cs.add_token_balance_set(
                                        *addr,
                                        token_type.clone(),
                                        amount,
                                    );
                                    eprintln!(
                                        "Set token balance for {}: {} = {}",
                                        addr, token_type, amount
                                    );
                                }
                            }
                        }

                        // Parse TreasuryCap resources: track total_supply and owner
                        if self.is_treasury_resource(struct_tag) {
                            if let Some(total) = self.extract_treasury_total_from_bytes(bytes) {
                                if let Some(token_type) =
                                    self.token_type_from_struct_tag(struct_tag)
                                {
                                    kanari_cs.add_treasury(*addr, token_type.clone(), total);
                                    eprintln!(
                                        "TreasuryCap for {} created/updated at {} with supply {}",
                                        token_type, addr, total
                                    );
                                }
                            }
                        }

                        // Detect created objects with UID in first bytes (object::UID.addr)
                        // For all objects that have a UID field (standard Move object pattern)
                        // the serialized layout starts with the UID address (32 bytes).
                        let obj_id = if bytes.len() >= 32 {
                            // Try to extract ID from first 32 bytes (UID pattern)
                            let id_bytes = &bytes[0..32];
                            hex::encode(id_bytes)
                        } else {
                            // Generate unique ID from address + struct_tag + sequence
                            self.generate_object_id(addr, struct_tag, bytes)
                        };

                        let obj_type = if let Some(tt) = self.token_type_from_struct_tag(struct_tag)
                        {
                            tt
                        } else {
                            format!(
                                "0x{}::{}::{}",
                                struct_tag.address.short_str_lossless(),
                                struct_tag.module.as_str(),
                                struct_tag.name.as_str()
                            )
                        };

                        // Record created object (version 0 for new objects)
                        kanari_cs.add_created_object(
                            obj_id.clone(),
                            *addr,
                            obj_type.clone(),
                            bytes.to_vec(),
                            0,
                        );
                        eprintln!(
                            "Created object detected: id={} owner={} type={}",
                            obj_id, addr, obj_type
                        );
                    }
                    MoveOp::Delete => {
                        // Resource deletion: if Coin/Balance deleted, set token balance to 0
                        if self.is_balance_resource(struct_tag) {
                            if let Some(token_type) = self.token_type_from_struct_tag(struct_tag) {
                                kanari_cs.add_token_balance_set(*addr, token_type.clone(), 0);
                                eprintln!(
                                    "Deleted token resource for {}: {} -> balance 0",
                                    addr, token_type
                                );
                            }
                        } else {
                            eprintln!("Resource deleted for {}: {}", addr, struct_tag);
                        }
                    }
                }
            }
        }
    }

    /// Parse Move VM events and add to Kanari ChangeSet
    /// Events provide an audit trail of all state changes
    pub(crate) fn parse_move_events(
        &self,
        events: &[move_core_types::effects::Event],
        kanari_cs: &mut ChangeSet,
    ) {
        use crate::changeset::Event;

        for event in events.iter() {
            let (key, sequence_number, type_tag, event_data) = event;
            let kanari_event = Event {
                key: key.clone(),
                sequence_number: *sequence_number,
                type_tag: format!("{}", type_tag),
                event_data: event_data.clone(),
            };
            kanari_cs.add_event(kanari_event);
        }
    }
}
