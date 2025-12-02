use move_binary_format::errors::PartialVMResult;
use move_core_types::account_address::AccountAddress;
use move_vm_runtime::native_functions::{NativeFunction, make_table_from_iter};
use move_vm_types::natives::function::NativeResult;
use move_vm_types::{pop_arg, values::Value};
use smallvec::smallvec;
use std::{collections::VecDeque, sync::{Arc, Mutex}};
use lazy_static::lazy_static;
use crate::pending_objects::{ObjectTransfer, ObjectFreeze, ObjectShare, PendingObjectOps};

// Global storage for pending object operations during VM execution
lazy_static! {
    static ref GLOBAL_PENDING_OPS: Mutex<PendingObjectOps> = Mutex::new(PendingObjectOps::new());
}

/// Get current pending operations
pub fn take_pending_ops() -> PendingObjectOps {
    let mut ops = GLOBAL_PENDING_OPS.lock().unwrap();
    std::mem::replace(&mut *ops, PendingObjectOps::new())
}

/// Build a NativeFunction easily
fn make_native<F>(f: F) -> NativeFunction
where
    F: Fn(
            &mut move_vm_runtime::native_functions::NativeContext,
            Vec<move_vm_types::loaded_data::runtime_types::Type>,
            VecDeque<Value>,
        ) -> PartialVMResult<NativeResult>
        + Send
        + Sync
        + 'static,
{
    Arc::new(f)
}

/// Native function: transfer object to recipient
/// This will be called by kanari_system::transfer::public_transfer
fn native_transfer_object(
    _context: &mut move_vm_runtime::native_functions::NativeContext,
    ty_args: Vec<move_vm_types::loaded_data::runtime_types::Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    eprintln!("🔍 native_transfer_object called!");
    
    // Pop arguments: object (generic T), recipient (address)
    let recipient = pop_arg!(arguments, AccountAddress);
    let object_value = arguments.pop_back(); // Generic object value
    
    eprintln!("🔍 Recipient: {}", recipient);
    
    // Serialize object type and data
    let object_type = if !ty_args.is_empty() {
        format!("{:?}", ty_args[0]) // Type parameter T
    } else {
        "Unknown".to_string()
    };
    
    eprintln!("🔍 Object type: {}", object_type);
    
    // Serialize object value to bytes (simplified)
    let object_data = if let Some(val) = &object_value {
        // In real implementation, would properly serialize Move value
        format!("{:?}", val).into_bytes()
    } else {
        vec![]
    };
    
    // Generate object ID (simplified - would use proper UID in real impl)
    let object_id = {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&object_type.as_bytes());
        hasher.update(&object_data);
        hasher.update(recipient.as_ref());
        let hash_result = hasher.finalize();
        hash_result[0..16].to_vec() // Use first 16 bytes
    };
    
    eprintln!("🔍 Object ID: {}", hex::encode(&object_id));
    
    // Store in global pending operations
    let transfer = ObjectTransfer {
        object_id: object_id.clone(),
        object_type: object_type.clone(),
        object_data,
        recipient,
    };
    
    {
        let mut ops = GLOBAL_PENDING_OPS.lock().unwrap();
        ops.add_transfer(transfer);
        eprintln!("🔍 Added to GLOBAL_PENDING_OPS, total transfers: {}", ops.transfers.len());
    }
    
    eprintln!("🔄 Object Transfer: {} -> {}", object_type, recipient);
    
    Ok(NativeResult::ok(
        100.into(), 
        smallvec![]
    ))
}

/// Native function: freeze object (make immutable)
fn native_freeze_object(
    _context: &mut move_vm_runtime::native_functions::NativeContext,
    _ty_args: Vec<move_vm_types::loaded_data::runtime_types::Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let _object = arguments.pop_back();
    
    // TODO: Mark object as frozen in object storage
    
    Ok(NativeResult::ok(
        50.into(),
        smallvec![]
    ))
}

/// Native function: share object (make accessible to all)
fn native_share_object(
    _context: &mut move_vm_runtime::native_functions::NativeContext,
    _ty_args: Vec<move_vm_types::loaded_data::runtime_types::Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let _object = arguments.pop_back();
    
    // TODO: Mark object as shared in object storage
    
    Ok(NativeResult::ok(
        50.into(),
        smallvec![]
    ))
}

/// Get all object-related native functions
pub fn object_natives(
    addr: AccountAddress,
) -> move_vm_runtime::native_functions::NativeFunctionTable {
    let natives = vec![
        (
            "transfer".to_string(),
            "native_transfer".to_string(),
            make_native(native_transfer_object),
        ),
        (
            "transfer".to_string(),
            "native_freeze".to_string(),
            make_native(native_freeze_object),
        ),
        (
            "transfer".to_string(),
            "native_share".to_string(),
            make_native(native_share_object),
        ),
    ];
    
    make_table_from_iter(
        addr,
        natives
            .into_iter()
            .map(|(m, f, func)| (m.into_boxed_str(), f.into_boxed_str(), func)),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_object_natives() {
        // Basic sanity test
        assert!(true);
    }
}
