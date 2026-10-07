// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;

impl MoveRuntime {
    /// Execute a module's init function with explicit context parameters.
    pub(crate) fn execute_init_function_with_context(
        &self,
        module_addr: AccountAddress,
        module_name: &str,
        args: Vec<Vec<u8>>,
        timestamp: Option<u64>,
        tx_hash: Option<Vec<u8>>,
    ) -> Result<ChangeSet> {
        self.execute_init_function_internal(module_addr, module_name, args, timestamp, tx_hash)
            .require("Failed to execute init()")
    }
    fn execute_init_function_internal(
        &self,
        module_addr: AccountAddress,
        module_name: &str,
        args: Vec<Vec<u8>>,
        timestamp: Option<u64>,
        tx_hash: Option<Vec<u8>>,
    ) -> Result<ChangeSet> {
        let module_id = ModuleId::new(module_addr, Identifier::new(module_name)?);
        self.execute_entry_function_internal(
            &module_id,
            "init",
            vec![],
            args,
            ExecutionOptions::new(Some(module_addr), None, timestamp, tx_hash).bypass_entry_check(),
        )
        .map(|(changeset, _)| changeset)
    }

    pub(super) fn preprocess_entry_args(args: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
        args.into_iter()
            .map(|arg| {
                // Only perform preprocessing for string-like inputs that are meant to be converted
                // Skip preprocessing if the arg is already a properly serialized BCS value

                // Skip if the arg has a common BCS-encoded primitive size:
                // - 1 byte: u8/bool
                // - 8 bytes: u64 (BCS little-endian)
                // - 16 bytes: u128
                // - 32 bytes: AccountAddress (handled below for hex string case)
                // This prevents misinterpreting binary BCS data as human-readable text.
                let len = arg.len();
                if len == 1 || len == 8 || len == 16 {
                    return arg;
                }

                // Check if this looks like a potential address string (hex string)
                if let Ok(s) = std::str::from_utf8(&arg) {
                    let s_trim = s.trim();
                    if s_trim.starts_with("0x") || s_trim.chars().all(|c| c.is_ascii_hexdigit()) {
                        // This looks like a hex string that should be converted to bytes
                        let clean_hex = s_trim.strip_prefix("0x").unwrap_or(s_trim);
                        if let Ok(bytes) = hex::decode(clean_hex)
                            && bytes.len() == AccountAddress::LENGTH
                        {
                            // This is a valid address hex string
                            return bytes;
                        }
                    } else if s_trim.parse::<u64>().is_ok() {
                        // This looks like a number string
                        if let Ok(n) = s_trim.parse::<u64>() {
                            return bcs::to_bytes(&n).unwrap_or(arg);
                        }
                    }
                }

                // For all other cases, return the original arg without modification
                // This prevents corrupting already-serialized BCS arguments
                arg
            })
            .collect()
    }

    pub(super) fn build_tx_context_bytes(
        &self,
        sender: Option<AccountAddress>,
        timestamp: Option<u64>,
        tx_hash: Option<&[u8]>,
    ) -> Result<Vec<u8>> {
        let sender_addr = sender.unwrap_or(AccountAddress::ZERO);
        let epoch_timestamp_ms = timestamp.unwrap_or(0);
        let tx_hash = if let Some(raw_tx_hash) = tx_hash {
            if raw_tx_hash.len() == 32 {
                raw_tx_hash.to_vec()
            } else {
                hash_data_blake3(raw_tx_hash).to_vec()
            }
        } else {
            let mut input = Vec::new();
            input.extend_from_slice(b"kanari-txctx-v1");
            input.extend_from_slice(sender_addr.as_ref());
            input.extend_from_slice(&epoch_timestamp_ms.to_le_bytes());
            hash_data_blake3(&input).to_vec()
        };
        let move_value = MoveValue::Struct(MoveStruct(vec![
            MoveValue::Address(sender_addr),
            MoveValue::vector_u8(tx_hash),
            MoveValue::U64(0),
            MoveValue::U64(epoch_timestamp_ms),
            MoveValue::U64(0),
        ]));
        move_value
            .simple_serialize()
            .require("Failed to serialize TxContext MoveValue")
    }

    pub(super) fn synthesize_otw_bytes_from_layout(layout: &MoveTypeLayout) -> Option<Vec<u8>> {
        let move_value = match layout {
            MoveTypeLayout::Struct(struct_layout) if struct_layout.0.is_empty() => {
                MoveValue::Struct(MoveStruct(vec![]))
            }
            MoveTypeLayout::Struct(struct_layout)
                if struct_layout.0.len() == 1
                    && matches!(struct_layout.0.first(), Some(MoveTypeLayout::Bool)) =>
            {
                MoveValue::Struct(MoveStruct(vec![MoveValue::Bool(true)]))
            }
            _ => return None,
        };

        move_value.simple_serialize()
    }
}
