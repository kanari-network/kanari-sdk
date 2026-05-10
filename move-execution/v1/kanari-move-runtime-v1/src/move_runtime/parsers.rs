// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Parser functions for Move VM changesets and events
use crate::changeset::ChangeSet;
use kanari_types::event::Event;
use kanari_types::object::UIDRecord;
use log::debug;
use move_core_types::account_address::AccountAddress;
use move_core_types::effects::Op as MoveOp;

impl super::MoveRuntime {
    /// Parse Move VM ChangeSet and extract state changes into Kanari ChangeSet
    /// This converts Move VM's canonical state changes into our domain model
    pub(crate) fn parse_move_changeset(
        &self,
        move_cs: &move_core_types::effects::ChangeSet,
        kanari_cs: &mut ChangeSet,
    ) {
        debug!(
            "[PARSER] parse_move_changeset: accounts={}, total_resources={}",
            move_cs.accounts().len(),
            move_cs
                .accounts()
                .values()
                .map(|a| a.resources().len())
                .sum::<usize>()
        );

        for (addr, account_changes) in move_cs.accounts() {
            for (module_name, op) in account_changes.modules() {
                if matches!(op, MoveOp::New(_) | MoveOp::Modify(_)) {
                    kanari_cs.publish_module(*addr, module_name.to_string());
                }
            }

            for (struct_tag, op) in account_changes.resources() {
                let id_input = format!("0x{}::{}", hex::encode(addr.as_ref()), struct_tag);
                let deterministic_id = format!(
                    "0x{}",
                    hex::encode(kanari_crypto::hash_data_blake3(id_input.as_bytes()))
                );

                match op {
                    MoveOp::New(bytes) | MoveOp::Modify(bytes) => {
                        let uid_opt = if bytes.len() >= 32 {
                            let mut arr = [0u8; 32];
                            arr.copy_from_slice(&bytes[0..32]);
                            Some(UIDRecord::new(AccountAddress::new(arr)))
                        } else {
                            None
                        };

                        let final_object_id = if let Some(uid) = &uid_opt {
                            uid.address().to_hex_literal()
                        } else {
                            deterministic_id.clone()
                        };

                        kanari_cs.add_created_object(
                            *addr,
                            format!("{}", struct_tag),
                            bytes.to_vec(),
                            0,
                            uid_opt,
                            Some(final_object_id),
                        );
                    }
                    MoveOp::Delete => {
                        // 🚨 FIX: Delete only the fake ID, never force Token balance to 0!
                        // StateManager will calculate the correct balance at the end
                        kanari_cs.add_deleted_object(deterministic_id.clone());
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
