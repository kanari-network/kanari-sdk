# Kanari Move Runtime - Object Management Improvements

## 📋 Overview

This document describes the improvements made to `kanari-move-runtime` for better object management through native function integration.

## ✅ Changes Made

### 1. Enhanced Session Extensions (`mod.rs`)

**File**: `src/move_runtime/mod.rs`

Added two critical extensions to support proper object lifecycle management:

```rust
// Added to create_session_with_storage_ext()
extensions.add(kanari_system_natives::object::LoadedObjectsExt::default());
extensions.add(kanari_system_natives::object::BorrowedObjectsExt::default());
```

**Purpose**:

- `LoadedObjectsExt`: Stores preloaded objects that can be accessed by `native_borrow_global_mut`
- `BorrowedObjectsExt`: Tracks mutable borrows for proper writeback after execution

### 2. Object Preloading Mechanism (`helpers.rs`)

**File**: `src/move_runtime/helpers.rs`

Added new helper function `preload_objects_for_execution()`:

```rust
pub(crate) fn preload_objects_for_execution(
    &self,
    session: &mut Session<KanariMoveResolver>,
    args: &[Vec<u8>],
) -> anyhow::Result<()>
```

**Functionality**:

- Scans transaction arguments for potential object IDs (32-byte addresses)
- Loads objects from `ObjectStorage` before VM execution
- Populates `LoadedObjectsExt` so `native_borrow_global_mut` can find them
- Enables proper object resolution during Move VM execution

**Integration Point**:
Called in `execute_entry_function_internal()` right after session creation:

```rust
let mut session = self.create_session_with_storage_ext(&vm_guard);
self.preload_objects_for_execution(&mut session, &args)?;
```

## 🎯 Benefits

### 1. Proper Native Function Integration

- `native_borrow_global_mut` now has access to preloaded objects
- Objects are resolved through proper extension mechanism instead of manual loading
- Follows Move VM architecture patterns

### 2. Cleaner Architecture

- Separates object loading from execution logic
- Uses extension pattern consistently across the runtime
- Makes code more maintainable and testable

### 3. Better Mutable Reference Tracking

- `BorrowedObjectsExt` tracks all mutable borrows
- Enables proper writeback handling after execution
- Reduces risk of state inconsistencies

### 4. Improved Debugging

- Added debug logging for object preloading
- Easier to trace object lifecycle through extensions
- Better visibility into what objects are loaded

## 🔧 Technical Details

### Object Loading Flow (Before)

```
Transaction Args → Manual ObjectStorage Lookup → Direct Data Injection
```

### Object Loading Flow (After)

```
Transaction Args → Preload to LoadedObjectsExt → native_borrow_global_mut Resolution
```

### Extension Lifecycle

1. **Session Creation**: All extensions initialized
2. **Preloading**: Objects loaded into `LoadedObjectsExt`
3. **Execution**: VM calls `native_borrow_global_mut` which accesses `LoadedObjectsExt`
4. **Post-Execution**: `BorrowedObjectsExt` contains tracked borrows for writeback
5. **Changeset Application**: Tracked objects written back to storage

## 📊 Testing

All existing tests pass without modification:

- ✅ 8 unit tests
- ✅ 1 publish/upgrade test  
- ✅ 5 inflation reproduction tests
- ✅ 7 mint consolidation tests

**Total**: 21/21 tests passing

## 🚀 Future Improvements

### Phase 2: Simplify Mutable Reference Handling

- Leverage `BorrowedObjectsExt` for automatic writeback
- Reduce manual tracking in `loaded_mutable_objects` vector
- Extract coin auto-merge logic to separate module

### Phase 3: Performance Optimization

- Batch object preloading for multiple transactions
- Cache frequently accessed objects
- Add metrics for object loading performance

### Phase 4: Enhanced Error Handling

- Better error messages when object loading fails
- Validation of object ownership before execution
- Graceful degradation when objects not found

## 📝 Migration Notes

### For Developers

- No API changes - all modifications are internal
- Existing Move modules work without changes
- CLI and SDK interfaces remain unchanged

### For Testing

- All existing tests pass without modification
- New tests can leverage `LoadedObjectsExt` for object setup
- Consider adding tests for edge cases in object preloading

## 🔍 Debugging Tips

### Enable Debug Logging

```bash
export RUST_LOG=debug
cargo run -p kanari-node start
```

### Key Log Messages

- `[RUNTIME] Preloaded object {id} into LoadedObjectsExt` - Object successfully preloaded
- Check for missing preloads if `native_borrow_global_mut` fails

### Common Issues

1. **Object Not Found**: Ensure object exists in ObjectStorage before transaction
2. **Type Mismatch**: Verify object type matches expected type in Move function
3. **Ownership Violation**: Check that sender owns or has access to object

## 📚 Related Documentation

- [`native_borrow_global_mut` Implementation](../../crates/kanari-system-natives/src/object.rs)
- [Object Storage Architecture](./ER_DIAGRAM.md)
- [Move Runtime Extensions](./src/move_runtime/move_runtime_extensions.rs)

## ✨ Summary

These improvements lay the foundation for proper object management in Kanari Move Runtime by:

1. Integrating native functions with extension system
2. Preloading objects before execution
3. Tracking mutable borrows for proper writeback
4. Maintaining backward compatibility

The changes make the runtime more robust, maintainable, and aligned with Move VM best practices.
