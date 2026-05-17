// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::natives::function::{PartialVMError, PartialVMResult};

pub(crate) fn make_module_natives(
    natives: impl IntoIterator<Item = (impl Into<String>, NativeFunction)>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    natives
        .into_iter()
        .map(|(func_name, func)| (func_name.into(), func))
}

pub(crate) fn expect_native_args(actual: usize, expected: usize) -> PartialVMResult<()> {
    if actual == expected {
        return Ok(());
    }

    Err(
        PartialVMError::new(move_core_types::vm_status::StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH)
            .with_message(format!(
                "Native argument count mismatch: expected {expected}, got {actual}"
            )),
    )
}

pub(crate) fn expect_type_args(actual: usize, expected: usize) -> PartialVMResult<()> {
    if actual == expected {
        return Ok(());
    }

    Err(PartialVMError::new(
        move_core_types::vm_status::StatusCode::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH,
    )
    .with_message(format!(
        "Native type argument count mismatch: expected {expected}, got {actual}"
    )))
}

pub(crate) fn expect_native_signature(
    actual_args: usize,
    expected_args: usize,
    actual_type_args: usize,
    expected_type_args: usize,
) -> PartialVMResult<()> {
    expect_native_args(actual_args, expected_args)?;
    expect_type_args(actual_type_args, expected_type_args)
}
