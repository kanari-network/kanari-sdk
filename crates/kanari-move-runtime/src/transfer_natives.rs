/// Native functions for object transfer with proper tracking
/// Uses thread-local storage to track transferred objects for parse_move_changeset
use move_core_types::account_address::AccountAddress;
use move_core_types::gas_algebra::GasQuantity;
use move_vm_runtime::native_functions::{NativeFunction, make_table_from_iter};
use move_vm_types::natives::function::NativeResult;
use move_vm_types::natives::function::PartialVMResult;
use move_vm_types::pop_arg;
use smallvec::smallvec;
use std::cell::RefCell;
use std::{collections::VecDeque, sync::Arc};

// Thread-local storage for tracking transferred objects
thread_local! {
    static TRANSFERRED_OBJECTS: RefCell<Vec<TransferredObject>> = RefCell::new(Vec::new());
}

/// Information about a transferred object with full data
#[derive(Clone, Debug)]
pub struct TransferredObject {
    pub object_id: String,
    pub object_type: String,
    pub recipient: AccountAddress,
    pub data: Vec<u8>,
    pub should_persist: bool, // Flag to indicate if object should be stored persistently
}

/// Record a transferred object for later tracking
pub fn record_transfer(obj: TransferredObject) {
    TRANSFERRED_OBJECTS.with(|objects| {
        objects.borrow_mut().push(obj);
    });
}

/// Get and clear all transferred objects
pub fn take_transferred_objects() -> Vec<TransferredObject> {
    TRANSFERRED_OBJECTS.with(|objects| objects.borrow_mut().drain(..).collect())
}

/// Build a NativeFunction from closure
fn make_native<F>(f: F) -> NativeFunction
where
    F: Fn(
            &mut move_vm_runtime::native_functions::NativeContext,
            Vec<move_vm_types::loaded_data::runtime_types::Type>,
            VecDeque<move_vm_types::values::Value>,
        ) -> PartialVMResult<NativeResult>
        + Send
        + Sync
        + 'static,
{
    Arc::new(f)
}

/// Get all transfer native functions
pub fn all_natives(
    move_addr: AccountAddress,
) -> move_vm_runtime::native_functions::NativeFunctionTable {
    let natives = vec![(
        "transfer",
        "transfer_with_uid",
        make_native(native_transfer_with_uid),
    )];

    make_table_from_iter(move_addr, natives)
}

/// transfer::transfer_with_uid<T: key + store>(obj: T, recipient: address)
/// Tracks transferred objects in thread-local storage for later retrieval
fn native_transfer_with_uid(
    context: &mut move_vm_runtime::native_functions::NativeContext,
    ty_args: Vec<move_vm_types::loaded_data::runtime_types::Type>,
    mut arguments: VecDeque<move_vm_types::values::Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    // Pop arguments: recipient (address), obj (generic T with key+store)
    let recipient = pop_arg!(arguments, AccountAddress);
    let obj_val = arguments.pop_back().expect("Missing object argument");

    // Get type argument (the object type T)
    if ty_args.is_empty() {
        return Ok(NR::err(context.gas_used(), 1)); // Missing type argument
    }

    // Extract type information
    let type_str = format!("{:?}", ty_args[0]);

    // Clean up the debug format to get a readable type string
    let type_str = if type_str.contains("Struct") {
        type_str
            .replace("Struct(", "")
            .replace(")", "")
            .trim()
            .to_string()
    } else {
        type_str
    };

    // Serialize object to bytes using Move's internal format
    // We'll use a simple approach: create a placeholder with just the type info
    // The actual data will be tracked by Move VM's changeset
    let obj_data = {
        // For now, store minimal data - just mark that object exists
        // In full implementation, we'd extract struct fields properly
        let mut data = Vec::new();
        data.extend_from_slice(recipient.as_ref()); // 32 bytes owner
        data.extend_from_slice(type_str.as_bytes()); // type name
        data
    };

    // Generate unique object ID
    let obj_id = {
        use kanari_crypto::hash_data_blake3;
        let mut input = Vec::new();
        input.extend_from_slice(recipient.as_ref());
        input.extend_from_slice(type_str.as_bytes());
        // Add timestamp for uniqueness
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        input.extend_from_slice(&timestamp.to_le_bytes());
        let hash = hash_data_blake3(&input);
        hex::encode(&hash[0..32])
    };

    println!(
        "[NATIVE] transfer_with_uid: object_id={}, type={}, recipient={}, data_len={}",
        obj_id,
        type_str,
        recipient,
        obj_data.len()
    );

    // Record the transfer in thread-local storage
    record_transfer(TransferredObject {
        object_id: obj_id,
        object_type: type_str,
        recipient,
        data: obj_data,
        should_persist: true, // Mark for persistent storage
    });

    // Consume the object (it's been transferred)
    drop(obj_val);

    // Gas cost: 2000 gas units for transfer tracking
    let gas_cost = GasQuantity::new(2000);

    Ok(NR::ok(context.gas_used() + gas_cost, smallvec![]))
}
