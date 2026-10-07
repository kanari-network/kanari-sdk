// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;

impl MoveRuntime {
    /// Execute a read-only function without persisting any state changes.
    pub fn execute_view_function(
        &self,
        package_addr: &str,
        module_name: &str,
        function_name: &str,
        type_args: &[String],
        args: &[Vec<u8>],
        object_inputs: &[ObjectInput],
    ) -> Result<serde_json::Value> {
        use move_core_types::account_address::AccountAddress;

        // Normalize the package address before constructing the module id.
        let addr_hex = if package_addr.starts_with("0x") || package_addr.starts_with("0X") {
            &package_addr[2..]
        } else {
            package_addr
        };
        let addr = AccountAddress::from_hex_literal(&format!("0x{}", addr_hex))
            .require("Invalid package address")?;

        let module_id = ModuleId::new(addr, Identifier::new(module_name)?);

        // Reuse the same session setup as entry-function execution.
        let vm_guard = self.read_vm();
        let mut session = self.create_session_with_storage_ext(&vm_guard);

        // Preload only object refs explicitly declared by the request.
        self.preload_objects_for_execution(&mut session, object_inputs, None, None)
            .require("Failed to preload objects")?;

        // Parse and load type arguments before invocation.
        let mut ty_args_loaded = Vec::with_capacity(type_args.len());
        for type_arg in type_args {
            // Parse type tag from string (e.g., "0x1::aptos_coin::AptosCoin") - uses cache
            let type_tag = self.parse_type_tag_fast(type_arg)?;
            ty_args_loaded.push(
                session
                    .load_type(&type_tag)
                    .require("Failed to load type argument")?,
            );
        }

        let ident = IdentStr::new(function_name).require("Invalid function name")?;
        let mut final_args = args.to_vec();

        if let Ok(func) = session.load_function(&module_id, ident, &ty_args_loaded) {
            let probe = ParamProbe::new(&session);
            let binding_requirements = probe.binding_requirements(&func.parameters);
            Self::validate_object_input_bindings(object_inputs, &binding_requirements, true)?;

            let mut explicit_object_bindings = object_inputs.iter();
            for (i, param_type) in func.parameters.iter().enumerate() {
                if MoveRuntime::object_param_mutability(param_type, |t| {
                    probe.is_key_struct_param(t)
                })
                .is_some()
                    && !probe.is_tx_context_param(param_type)
                    && let Some(explicit_input) = explicit_object_bindings.next()
                {
                    if final_args.len() <= i {
                        final_args.resize_with(i + 1, Vec::new);
                    }
                    let stored_obj = self.load_object_by_ref_checked(&explicit_input.object_ref)?;
                    final_args[i] = stored_obj.data;
                }
            }
        }

        const MAX_VIEW_GAS_UNITS: u64 = 1_000_000;
        let mut metered_gas = crate::kanari_gas_meter::KanariGasMeter::new(MAX_VIEW_GAS_UNITS);
        let execution_result = session.execute_function_bypass_visibility(
            &module_id,
            ident,
            ty_args_loaded,
            final_args,
            &mut metered_gas,
        );

        // Finish the session without persisting any writes.
        let (res, _) = session.finish();
        let (_, _) = res.require("Session error")?;

        match execution_result {
            Ok(return_values) => {
                // Convert return values into JSON without layout-heavy decoding.
                let results: Vec<serde_json::Value> = return_values
                    .return_values
                    .into_iter()
                    .map(|(bytes, _)| Self::bytes_to_json_fast(&bytes))
                    .collect();

                if results.len() == 1 {
                    results
                        .into_iter()
                        .next()
                        .require("View function returned no values")
                } else {
                    Ok(serde_json::Value::Array(results))
                }
            }
            Err(e) => Err(anyhow::anyhow!(
                "View function execution failed: {} ({:?})",
                Self::explain_view_vm_error(&e),
                e
            )),
        }
    }

    fn explain_view_vm_error(e: &move_binary_format::errors::VMError) -> &'static str {
        match e.sub_status() {
            Some(kanari_system_natives::object::E_OBJECT_NOT_FOUND) => {
                "object not found; the object id may be wrong, deleted, or not indexed yet"
            }
            Some(kanari_system_natives::object::E_OBJECT_LAYOUT_UNAVAILABLE) => {
                "object type layout is unavailable for this view call"
            }
            Some(kanari_system_natives::object::E_OBJECT_TYPE_MISMATCH) => {
                "object type mismatch; this commonly happens after calling a newer package version with an object created by an older package address/type"
            }
            Some(kanari_system_natives::object::E_OBJECT_DESERIALIZE_FAILED) => {
                "object data could not be deserialized with the current struct layout; this commonly happens after an incompatible contract upgrade"
            }
            Some(1100) => {
                "object data could not be deserialized or loaded; check object id, type args, and contract version"
            }
            _ => "VM aborted during view function execution",
        }
    }

    /// Parse type tags with a small in-memory cache for repeated lookups.
    fn parse_type_tag_fast(&self, type_str: &str) -> Result<TypeTag> {
        use move_core_types::language_storage::TypeTag;

        // Check the cache before parsing common type strings again.
        if let Some(cached) = self.get_cached_type_tag(type_str) {
            return Ok(cached);
        }

        // Handle primitive types directly before falling back to struct parsing.
        let result = match type_str {
            "u8" => TypeTag::U8,
            "u16" => TypeTag::U16,
            "u32" => TypeTag::U32,
            "u64" => TypeTag::U64,
            "u128" => TypeTag::U128,
            "u256" => TypeTag::U256,
            "bool" => TypeTag::Bool,
            "address" => TypeTag::Address,
            "signer" => TypeTag::Signer,
            _ => {
                // Fall back to simple `address::module::name` struct parsing.
                if type_str.contains("::") {
                    let parts: Vec<&str> = type_str.split("::").collect();
                    if parts.len() >= 3 {
                        TypeTag::Struct(Box::new(StructTag {
                            address: AccountAddress::from_hex_literal(parts[0])
                                .require("Invalid address in type")?,
                            module: Identifier::new(parts[1]).require("Invalid module in type")?,
                            name: Identifier::new(parts[2]).require("Invalid name in type")?,
                            type_params: vec![],
                        }))
                    } else {
                        return Err(anyhow::anyhow!("Unsupported type: {}", type_str));
                    }
                } else {
                    return Err(anyhow::anyhow!("Unsupported type: {}", type_str));
                }
            }
        };

        // Cache the parsed result for later calls.
        self.cache_type_tag(type_str.to_string(), result.clone());
        Ok(result)
    }

    /// Read a cached type tag if present.
    fn get_cached_type_tag(&self, type_str: &str) -> Option<TypeTag> {
        if let Ok(cache) = self.type_tag_cache.read() {
            return cache.get(type_str).cloned();
        }
        None
    }

    /// Store a parsed type tag in the cache.
    fn cache_type_tag(&self, type_str: String, type_tag: TypeTag) {
        if let Ok(mut cache) = self.type_tag_cache.write() {
            cache.insert(type_str, type_tag);
        }
    }

    /// Convert return bytes into JSON with a lightweight best-effort strategy.
    fn bytes_to_json_fast(bytes: &[u8]) -> serde_json::Value {
        // Try a few simple decodings before falling back to hex.
        if bytes.len() <= 8 {
            // Likely a primitive type (u8, u16, u32, u64, bool)
            if bytes.len() == 1 {
                return serde_json::Value::Number(serde_json::Number::from(bytes[0]));
            }
            if bytes.len() == 8
                && let Ok(num) = bcs::from_bytes::<u64>(bytes)
            {
                return serde_json::Value::Number(serde_json::Number::from(num));
            }
        }

        // For addresses (32 bytes)
        if bytes.len() == 32
            && let Ok(addr) = AccountAddress::from_bytes(bytes)
        {
            return serde_json::Value::String(addr.to_hex_literal());
        }

        serde_json::Value::Array(bytes.iter().copied().map(serde_json::Value::from).collect())
    }
}
