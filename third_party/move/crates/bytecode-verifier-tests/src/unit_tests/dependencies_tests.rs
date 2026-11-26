// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::file_format::*;
use move_bytecode_verifier::dependencies;
use move_core_types::{identifier::Identifier, vm_status::StatusCode};

/// A function handle pointing at a module handle with no matching dependency must fail
/// verification instead of panicking while the dependency context is built (in any build mode).
#[test]
fn missing_dependency_in_function_handle_returns_error() {
    let mut module = empty_module();
    module.identifiers.push(Identifier::new("Missing").unwrap());
    module
        .module_handles
        .push(ModuleHandle { address: AddressIdentifierIndex(0), name: IdentifierIndex(1) });
    module.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(1),
        name: IdentifierIndex(1),
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters: vec![],
    });

    let result = dependencies::verify_module(&module, std::iter::empty());
    assert_eq!(result.unwrap_err().major_status(), StatusCode::MISSING_DEPENDENCY,);
}
