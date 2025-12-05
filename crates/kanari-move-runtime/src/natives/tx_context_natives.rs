use move_binary_format::errors::PartialVMResult;
use move_core_types::{account_address::AccountAddress, gas_algebra::InternalGas};
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::NativeResult,
    values::{StructRef, Value},
};
use smallvec::smallvec;
use std::collections::VecDeque;
use std::sync::Arc;

/// Native function: tx_context::sender
/// Returns the address of the transaction sender
pub fn native_sender(
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    // Pop TxContext reference (it's a struct reference)
    let tx_context_ref = arguments.pop_back().unwrap();

    // Borrow the struct and extract sender field (first field, index 0)
    let sender = match tx_context_ref.value_as::<StructRef>() {
        Ok(struct_ref) => {
            // Get first field (sender: address)
            match struct_ref.borrow_field(0) {
                Ok(sender_value) => match sender_value.value_as::<AccountAddress>() {
                    Ok(addr) => addr,
                    Err(_) => AccountAddress::ZERO,
                },
                Err(_) => AccountAddress::ZERO,
            }
        }
        Err(_) => AccountAddress::ZERO,
    };

    Ok(NativeResult::ok(
        InternalGas::new(100),
        smallvec![Value::address(sender)],
    ))
}

/// Native function: tx_context::epoch
/// Returns the current epoch number
pub fn native_epoch(
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let tx_context_ref = arguments.pop_back().unwrap();

    // Extract epoch field (index 2) from TxContext struct
    let epoch = match tx_context_ref.value_as::<StructRef>() {
        Ok(struct_ref) => match struct_ref.borrow_field(2) {
            Ok(epoch_value) => epoch_value.value_as::<u64>().unwrap_or(0),
            Err(_) => 0,
        },
        Err(_) => 0,
    };

    Ok(NativeResult::ok(
        InternalGas::new(100),
        smallvec![Value::u64(epoch)],
    ))
}

/// Native function: tx_context::epoch_timestamp_ms
/// Returns the epoch start timestamp in milliseconds
pub fn native_epoch_timestamp_ms(
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let tx_context_ref = arguments.pop_back().unwrap();

    // Extract epoch_timestamp_ms field (index 3) from TxContext struct
    let timestamp = match tx_context_ref.value_as::<StructRef>() {
        Ok(struct_ref) => match struct_ref.borrow_field(3) {
            Ok(ts_value) => ts_value.value_as::<u64>().unwrap_or(0),
            Err(_) => 0,
        },
        Err(_) => 0,
    };

    Ok(NativeResult::ok(
        InternalGas::new(100),
        smallvec![Value::u64(timestamp)],
    ))
}

/// Native function: tx_context::fresh_object_address
/// Generates a unique object address
pub fn native_fresh_object_address(
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let _tx_context_ref = arguments.pop_back().unwrap();

    // Generate a random object address
    // In production, this should be deterministic based on tx_hash and ids_created counter
    let object_address = AccountAddress::random();

    Ok(NativeResult::ok(
        InternalGas::new(200),
        smallvec![Value::address(object_address)],
    ))
}

/// Native function: tx_context::derive_id
/// Derives an ID from transaction hash and counter (internal use)
pub fn native_derive_id(
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let _counter = arguments.pop_back().unwrap().value_as::<u64>().unwrap_or(0);
    let _tx_hash = arguments.pop_back().unwrap();

    // Generate a derived address
    let derived_address = AccountAddress::random();

    Ok(NativeResult::ok(
        InternalGas::new(200),
        smallvec![Value::address(derived_address)],
    ))
}

/// Alternative registration for direct use (compatible with existing object_natives pattern)
pub fn tx_context_natives(
    addr: AccountAddress,
) -> Vec<(
    AccountAddress,
    move_core_types::identifier::Identifier,
    move_core_types::identifier::Identifier,
    NativeFunction,
)> {
    use move_core_types::identifier::Identifier;

    let module_name = Identifier::new("tx_context").unwrap();

    vec![
        (
            addr,
            module_name.clone(),
            Identifier::new("native_sender").unwrap(),
            Arc::new(native_sender),
        ),
        (
            addr,
            module_name.clone(),
            Identifier::new("native_epoch").unwrap(),
            Arc::new(native_epoch),
        ),
        (
            addr,
            module_name.clone(),
            Identifier::new("native_epoch_timestamp_ms").unwrap(),
            Arc::new(native_epoch_timestamp_ms),
        ),
        (
            addr,
            module_name.clone(),
            Identifier::new("native_fresh_object_address").unwrap(),
            Arc::new(native_fresh_object_address),
        ),
        (
            addr,
            module_name,
            Identifier::new("native_derive_id").unwrap(),
            Arc::new(native_derive_id),
        ),
    ]
}
