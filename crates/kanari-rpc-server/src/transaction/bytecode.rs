// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Compiled-module introspection: function names and object-parameter detection.

use super::*;

// Extract function names from module bytecode (returns None on error)
pub(crate) fn extract_functions_from_bytes(bytes: &[u8]) -> Option<Vec<String>> {
    CompiledModule::deserialize_with_defaults(bytes)
        .ok()
        .map(|module| {
            module
                .function_defs()
                .iter()
                .map(|func_def| {
                    let fh = module.function_handle_at(func_def.function);
                    module.identifier_at(fh.name).as_str().to_string()
                })
                .collect()
        })
}

// Best-effort lookup of module functions using the engine registry
pub(crate) fn lookup_module_functions(
    state: &RpcServerState,
    module_str: &str,
) -> Option<Vec<String>> {
    // If given as "address::Name" try direct lookup
    if let Some(idx) = module_str.find("::") {
        let addr = &module_str[..idx];
        let name = &module_str[idx + 2..];
        if let Some(bytes) = state.engine.get_module_bytecode(addr, name) {
            return extract_functions_from_bytes(&bytes);
        }
    }

    // Search published modules by name or address
    let modules = state.engine.list_all_modules();
    let results: Vec<String> = modules
        .into_iter()
        .filter(|(addr, name)| name == module_str || addr == module_str)
        .filter_map(|(addr, name)| {
            state
                .engine
                .get_module_bytecode(&addr, &name)
                .and_then(|bytes| extract_functions_from_bytes(&bytes))
                .map(|fns| {
                    fns.into_iter()
                        .map(move |f| format!("{}::{}::{}", addr, name, f))
                })
        })
        .flatten()
        .collect();

    if results.is_empty() {
        None
    } else {
        Some(results)
    }
}

fn is_tx_context_token(module: &CompiledModule, token: &SignatureToken) -> bool {
    let struct_idx = match token {
        SignatureToken::Struct(idx) => *idx,
        SignatureToken::StructInstantiation(instantiation) => instantiation.0,
        _ => return false,
    };
    let struct_handle = module.struct_handle_at(struct_idx);
    let module_handle = module.module_handle_at(struct_handle.module);
    let address = module.address_identifier_at(module_handle.address);
    let module_name = module.identifier_at(module_handle.name);
    let struct_name = module.identifier_at(struct_handle.name);
    *address == Address::kanari_system_account_address()
        && module_name.as_str() == "tx_context"
        && struct_name.as_str() == "TxContext"
}

fn struct_token_has_key_ability(module: &CompiledModule, struct_idx: StructHandleIndex) -> bool {
    module.struct_handle_at(struct_idx).abilities.has_key()
}

fn token_is_object_param(module: &CompiledModule, token: &SignatureToken) -> bool {
    fn visit(module: &CompiledModule, token: &SignatureToken, by_ref: bool) -> bool {
        match token {
            SignatureToken::Reference(inner) | SignatureToken::MutableReference(inner) => {
                visit(module, inner, true)
            }
            SignatureToken::Struct(struct_idx) => {
                !is_tx_context_token(module, token)
                    && struct_token_has_key_ability(module, *struct_idx)
            }
            SignatureToken::StructInstantiation(instantiation) => {
                // Generic structs passed by reference are object bindings even
                // when dependency loading does not expose key ability metadata.
                !is_tx_context_token(module, token)
                    && (by_ref || struct_token_has_key_ability(module, instantiation.0))
            }
            _ => false,
        }
    }

    visit(module, token, false)
}

pub(crate) fn function_object_param_indices(
    state: &RpcServerState,
    package: &str,
    module_name: &str,
    function_name: &str,
) -> Option<HashSet<usize>> {
    let bytes = state.engine.get_module_bytecode(package, module_name)?;
    let module = CompiledModule::deserialize_with_defaults(&bytes).ok()?;
    let func_def = module.function_defs().iter().find(|func_def| {
        let handle = module.function_handle_at(func_def.function);
        module.identifier_at(handle.name).as_str() == function_name
    })?;
    let handle = module.function_handle_at(func_def.function);
    let signature = module.signature_at(handle.parameters);
    Some(
        signature
            .0
            .iter()
            .enumerate()
            .filter_map(|(index, token)| token_is_object_param(&module, token).then_some(index))
            .collect(),
    )
}
