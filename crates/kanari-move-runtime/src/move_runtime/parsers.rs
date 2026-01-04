// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Parser functions for Move VM changesets and events
use crate::changeset::ChangeSet;
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
        // Diagnostic: print top-level summary of incoming Move changeset
        let acct_len = move_cs.accounts().len();
        let mut total_resources = 0usize;
        for acct in move_cs.accounts().values() {
            total_resources += acct.resources().len();
        }
        debug!(
            "[PARSER] parse_move_changeset: accounts={}, total_resources={}",
            acct_len, total_resources
        );
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
                        debug!(
                            "[PARSER] Module deletion detected for {}::{}",
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
                        if self.is_balance_resource(struct_tag)
                            && let Some(amount) = self.extract_balance_from_bytes(bytes, struct_tag)
                            && let Some(token_type) = self.token_type_from_struct_tag(struct_tag)
                        {
                            // Record absolute token balance for this account
                            kanari_cs.add_token_balance_set(*addr, token_type.clone(), amount);
                            debug!(
                                "[PARSER] Set token balance for {}: {} = {}",
                                addr, token_type, amount
                            );
                        }

                        // Parse TreasuryCap resources: track total_supply and owner
                        if self.is_treasury_resource(struct_tag)
                            && let Some(total) = self.extract_treasury_total_from_bytes(bytes)
                            && let Some(token_type) = self.token_type_from_struct_tag(struct_tag)
                        {
                            kanari_cs.add_treasury(*addr, token_type.clone(), total);
                            debug!(
                                "[PARSER] TreasuryCap for {} owner={} supply={}",
                                token_type, addr, total
                            );

                            // Persist treasury record so supply/owner survive restarts
                            if let Err(e) = self.state.save_treasury(&token_type, addr, total) {
                                log::warn!(
                                    "Failed to persist treasury {} -> {}: {}",
                                    token_type,
                                    addr,
                                    e
                                );
                            }
                        }

                        // Parse NftCap resources: track remaining/issued and collection id
                        if self.is_nftcap_resource(struct_tag)
                            && let Some((remaining, issued, collection_id)) = self.extract_nftcap_from_bytes(bytes)
                            && let Some(token_type) = self.token_type_from_struct_tag(struct_tag)
                        {
                            use kanari_types::collection::NftCapRecord;

                            let mut cap = NftCapRecord::new(
                                // UID for cap is present in bytes; use placeholder UIDRecord via UIDRecord::new
                                UIDRecord::new(collection_id),
                                remaining,
                                issued,
                                collection_id,
                            );

                            // Record nft cap for this owner and token type
                            kanari_cs.add_nftcap(*addr, token_type.clone(), cap);
                            debug!(
                                "[PARSER] NftCap for {} owner={} remaining={} issued={}",
                                token_type, addr, remaining, issued
                            );
                        }

                        // Detect created objects with UID in first bytes (object::UID.addr)
                        // For all objects that have a UID field (standard Move object pattern)
                        // the serialized layout starts with the UID address (32 bytes).
                        // Only treat as a created object if it's NOT a balance/treasury resource
                        if !(self.is_balance_resource(struct_tag)
                            || self.is_treasury_resource(struct_tag))
                        {
                            // Try to extract a UID address from the first 32 bytes
                            // of the serialized object value. If present, convert
                            // to `UIDRecord` and let ChangeSet compute canonical id.
                            let uid_opt = if bytes.len() >= 32 {
                                let mut arr = [0u8; 32];
                                arr.copy_from_slice(&bytes[0..32]);
                                Some(UIDRecord::new(AccountAddress::new(arr)))
                            } else {
                                None
                            };

                            let obj_type =
                                if let Some(tt) = self.token_type_from_struct_tag(struct_tag) {
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
                                *addr,
                                obj_type.clone(),
                                bytes.to_vec(),
                                0,
                                uid_opt,
                            );
                            debug!(
                                "[PARSER] Created object detected: owner={} type={}",
                                addr, obj_type
                            );
                        }
                    }
                    MoveOp::Delete => {
                        // Resource deletion: if Coin/Balance deleted, set token balance to 0
                        if self.is_balance_resource(struct_tag)
                            && let Some(token_type) = self.token_type_from_struct_tag(struct_tag)
                        {
                            kanari_cs.add_token_balance_set(*addr, token_type.clone(), 0);
                            debug!(
                                "[PARSER] Deleted token resource for {}: {} -> balance 0",
                                addr, token_type
                            );
                        } else {
                            debug!("[PARSER] Resource deleted for {}: {}", addr, struct_tag);
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
