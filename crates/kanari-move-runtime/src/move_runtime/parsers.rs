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
            // 1. จัดการ Modules (เหมือนเดิม)
            for (module_name, op) in account_changes.modules() {
                if matches!(op, MoveOp::New(_) | MoveOp::Modify(_)) {
                    kanari_cs.publish_module(*addr, module_name.to_string());
                }
            }

            // 2. จัดการ Resources (จุดที่มีปัญหา)
            for (struct_tag, op) in account_changes.resources() {
                // 🚨 สร้าง ID มาตรฐานที่ใช้ร่วมกันทั้ง New และ Delete (Owner + Type)
                let id_input = format!("0x{}::{}", hex::encode(addr.as_ref()), struct_tag);
                let deterministic_id = format!(
                    "0x{}",
                    hex::encode(kanari_crypto::hash_data_blake3(id_input.as_bytes()))
                );

                match op {
                    MoveOp::New(bytes) | MoveOp::Modify(bytes) => {
                        // (ส่วนอัปเดต balance_set เหมือนเดิม)
                        let uid_opt = if bytes.len() >= 32 {
                            let mut arr = [0u8; 32];
                            arr.copy_from_slice(&bytes[0..32]);
                            Some(UIDRecord::new(AccountAddress::new(arr)))
                        } else {
                            None
                        };

                        kanari_cs.add_created_object(
                            *addr,
                            format!("{}", struct_tag),
                            bytes.to_vec(),
                            0,
                            uid_opt,
                            Some(deterministic_id), // 🚨 ใช้ ID ที่คงที่นี้เท่านั้น
                        );
                    }
                    MoveOp::Delete => {
                        // 🚨 ใช้ ID เดียวกันเป๊ะกับตอนสร้าง เพื่อให้ลบเหรียญเก่าออกได้จริง
                        kanari_cs.add_deleted_object(deterministic_id);

                        if self.is_balance_resource(struct_tag)
                            && let Some(token_type) = self.token_type_from_struct_tag(struct_tag)
                        {
                            kanari_cs.add_token_balance_set(*addr, token_type, 0);
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
