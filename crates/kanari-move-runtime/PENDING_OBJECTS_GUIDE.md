# Pending Objects Guide

คู่มือการใช้งาน `pending_objects` ใน MoveRuntime สำหรับติดตาม object operations ระหว่าง Move VM execution

## 📝 ภาพรวม

`pending_objects` เป็น field ใน `MoveRuntime` ที่ใช้เก็บรายการ object operations ทั้งหมดที่เกิดขึ้นระหว่างการ execute Move functions โดยเฉพาะอย่างยิ่ง operations ที่เกิดจาก native functions เช่น:

- **Transfer**: การโอน object ให้ผู้อื่น
- **Freeze**: การทำให้ object เป็น immutable
- **Share**: การแชร์ object ให้หลายคนใช้งาน

## 🎯 การทำงาน

### Architecture

```
Move Function Call
       ↓
Native Function (transfer::public_transfer)
       ↓
Add to GLOBAL_PENDING_OPS (thread-safe global)
       ↓
execute_entry_function completes
       ↓
Merge: runtime.pending_objects + GLOBAL_PENDING_OPS
       ↓
Return in ChangeSet.object_operations
```

### Type Definition

```rust
// Thread-safe reference wrapper
pub type PendingObjectOpsRef = Arc<Mutex<PendingObjectOps>>;

// Container for all pending operations
pub struct PendingObjectOps {
    pub transfers: Vec<ObjectTransfer>,
    pub freezes: Vec<ObjectFreeze>,
    pub shares: Vec<ObjectShare>,
}
```

## 🔧 Methods

### 1. `get_pending_objects()` - อ่านค่า

อ่านค่า pending operations ปัจจุบันโดยไม่ลบข้อมูล (read-only)

```rust
let runtime = MoveRuntime::new_with_kanari_natives()?;

// Execute some function that creates objects
runtime.execute_entry_function(...)?;

// Read pending operations
let pending = runtime.get_pending_objects();
println!("Total transfers: {}", pending.transfers.len());
println!("Total freezes: {}", pending.freezes.len());
println!("Total shares: {}", pending.shares.len());
```

### 2. `take_pending_objects()` - เอาค่าและลบ

เอาค่า pending operations และลบข้อมูลออกทันที (consume and clear)

```rust
let mut runtime = MoveRuntime::new_with_kanari_natives()?;

// Execute function
runtime.execute_entry_function(...)?;

// Take operations (clears after taking)
let operations = runtime.take_pending_objects();

// Process operations
for transfer in operations.transfers {
    println!("Transfer {} to {}", 
        hex::encode(&transfer.object_id),
        transfer.recipient
    );
}

// Now pending_objects is empty
assert!(runtime.get_pending_objects().is_empty());
```

### 3. `clear_pending_objects()` - ลบข้อมูล

ลบ pending operations ทั้งหมด

```rust
let mut runtime = MoveRuntime::new_with_kanari_natives()?;

runtime.execute_entry_function(...)?;

// Clear all pending operations
runtime.clear_pending_objects();

// Verify empty
let pending = runtime.get_pending_objects();
assert!(pending.is_empty());
```

### 4. `add_pending_transfer()` - เพิ่ม transfer operation

เพิ่ม transfer operation เองด้วยตนเอง (สำหรับกรณีพิเศษ)

```rust
let mut runtime = MoveRuntime::new_with_kanari_natives()?;

// Manually add a transfer
let object_id = vec![1, 2, 3, 4];
let object_type = "0x2::nft::NFT".to_string();
let object_data = vec![/* serialized object */];
let recipient = AccountAddress::from_hex_literal("0x999")?;

runtime.add_pending_transfer(
    object_id,
    object_type,
    object_data,
    recipient,
);

// Check that it was added
let pending = runtime.get_pending_objects();
assert_eq!(pending.transfers.len(), 1);
```

## 📊 ChangeSet Integration

เมื่อ execute function สำเร็จ, `ChangeSet` จะมี `object_operations` ที่รวม operations จากทั้ง:
1. Runtime's `pending_objects`
2. Global `GLOBAL_PENDING_OPS` (จาก native functions)

```rust
let mut runtime = MoveRuntime::new_with_kanari_natives()?;

let changeset = runtime.execute_entry_function(
    &module_id,
    "mint_nft",
    vec![],
    args,
    Some(sender),
    Some((1_000_000, 1)),
)?;

// Access object operations from changeset
println!("📦 Object Operations in ChangeSet:");
println!("  Transfers: {}", changeset.object_operations.transfers.len());
println!("  Freezes:   {}", changeset.object_operations.freezes.len());
println!("  Shares:    {}", changeset.object_operations.shares.len());

// Inspect details
for (i, transfer) in changeset.object_operations.transfers.iter().enumerate() {
    println!("\n🔄 Transfer #{}", i + 1);
    println!("  Object ID:   {}", hex::encode(&transfer.object_id));
    println!("  Type:        {}", transfer.object_type);
    println!("  Recipient:   {}", transfer.recipient);
    println!("  Data Size:   {} bytes", transfer.object_data.len());
}
```

## 💡 Use Cases

### 1. Tracking NFT Transfers

```rust
let changeset = runtime.execute_entry_function(
    &nft_module,
    "transfer_nft",
    vec![],
    args,
    Some(sender),
    gas_info,
)?;

// Track all NFT transfers
for transfer in &changeset.object_operations.transfers {
    if transfer.object_type.contains("NFT") {
        // Log NFT transfer for indexing
        log_nft_transfer(
            hex::encode(&transfer.object_id),
            transfer.recipient,
        );
    }
}
```

### 2. Batch Object Operations

```rust
let mut runtime = MoveRuntime::new_with_kanari_natives()?;

// Execute multiple functions
for nft in nfts {
    runtime.execute_entry_function(
        &module,
        "mint",
        vec![],
        serialize_args(&nft),
        Some(minter),
        None,
    )?;
}

// Get all operations at once
let all_operations = runtime.take_pending_objects();
println!("Total NFTs minted: {}", all_operations.transfers.len());

// Process in batch
state_manager.apply_object_operations(&all_operations)?;
```

### 3. Validating Object Safety

```rust
let changeset = runtime.execute_entry_function(...)?;

// Ensure no unauthorized transfers
for transfer in &changeset.object_operations.transfers {
    if !is_authorized_recipient(&transfer.recipient) {
        return Err(anyhow::anyhow!("Unauthorized transfer detected"));
    }
}

// Ensure frozen objects aren't modified
if !changeset.object_operations.freezes.is_empty() {
    validate_freeze_permissions(&changeset.object_operations.freezes)?;
}
```

## 🔐 Thread Safety

`pending_objects` ใช้ `Arc<Mutex<>>` เพื่อความปลอดภัยใน multi-threaded environments:

```rust
use std::sync::Arc;

let runtime = Arc::new(Mutex::new(
    MoveRuntime::new_with_kanari_natives()?
));

// Safe to use across threads
let runtime_clone = Arc::clone(&runtime);
std::thread::spawn(move || {
    let mut rt = runtime_clone.lock().unwrap();
    rt.execute_entry_function(...);
});
```

## ⚠️ Best Practices

### 1. Always Process Operations After Execution

```rust
// ✅ Good: Process operations immediately
let changeset = runtime.execute_entry_function(...)?;
process_object_operations(&changeset.object_operations)?;
```

```rust
// ❌ Bad: Ignoring operations
let _ = runtime.execute_entry_function(...)?;
// Operations lost!
```

### 2. Clear Operations Between Transactions

```rust
// Execute transaction 1
let cs1 = runtime.execute_entry_function(...)?;
process_operations(&cs1.object_operations)?;

// Clear before transaction 2
runtime.clear_pending_objects();

// Execute transaction 2
let cs2 = runtime.execute_entry_function(...)?;
// cs2 won't contain cs1's operations
```

### 3. Validate Operations Before Applying

```rust
let changeset = runtime.execute_entry_function(...)?;

// Validate before applying to state
for transfer in &changeset.object_operations.transfers {
    if !object_exists(&transfer.object_id) {
        return Err(anyhow::anyhow!("Object not found"));
    }
    if !can_transfer_to(&transfer.recipient) {
        return Err(anyhow::anyhow!("Invalid recipient"));
    }
}

// Safe to apply
state_manager.apply_changeset(changeset)?;
```

## 📚 See Also

- [examples/pending_objects_example.rs](examples/pending_objects_example.rs) - ตัวอย่างการใช้งานแบบเต็ม
- [src/objects/pending_objects.rs](src/objects/pending_objects.rs) - Implementation code
- [src/natives/object_natives.rs](src/natives/object_natives.rs) - Native functions ที่ใช้ pending_objects
- [SYSTEM_MODULES.md](SYSTEM_MODULES.md) - ข้อมูล kanari_system::transfer และ kanari_system::object

## 🎓 Summary

**pending_objects** คือกลไกสำคัญที่เชื่อมโยง Move VM กับ Kanari blockchain โดย:

1. ✅ ติดตาม object operations จาก native functions
2. ✅ รวม operations ไว้ใน ChangeSet
3. ✅ ให้ StateManager นำไป apply กับ blockchain state
4. ✅ รองรับ multi-threaded access
5. ✅ มี API ที่ชัดเจนและใช้งานง่าย

ใช้ methods เหล่านี้เพื่อ:
- `get_pending_objects()` - อ่านค่า
- `take_pending_objects()` - เอาค่าและลบ
- `clear_pending_objects()` - ลบทั้งหมด
- `add_pending_transfer()` - เพิ่มด้วยตนเอง
