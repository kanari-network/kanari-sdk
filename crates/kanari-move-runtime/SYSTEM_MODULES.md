# Loading Kanari System Modules

## Problem

When executing Move transactions, you may encounter:

```
LINKER_ERROR: Cannot find ModuleId { address: 0x2, name: "transfer" } in data cache
```

This happens because the Kanari system modules (0x2::*) need to be loaded into the Move VM storage.

## Solution

The enhanced `MoveRuntime::new_with_kanari_natives()` now automatically loads all system modules.

### Automatic Loading (Recommended)

```rust
use kanari_move_runtime::MoveRuntime;

// Creates runtime with:
// 1. Standard library natives (0x1)
// 2. Kanari crypto natives (0x2)  
// 3. Pre-compiled system modules (0x2::transfer, 0x2::coin, etc.)
let mut runtime = MoveRuntime::new_with_kanari_natives()?;

// Now you can use system modules immediately
runtime.execute_entry_function(
    &ModuleId::new(
        AccountAddress::from_hex_literal("0x2")?,
        "transfer".parse()?
    ),
    "transfer_object",
    vec![],
    args,
    Some(sender),
    Some((gas_limit, gas_price))
)?;
```

### Custom Module Path

If your Kanari framework is in a different location:

```bash
# Set environment variable
export KANARI_FRAMEWORK_PATH=/path/to/kanari-frameworks/packages/kanari-system/build/KanariSystem/bytecode_modules
```

Or in PowerShell:

```powershell
$env:KANARI_FRAMEWORK_PATH = "C:\path\to\kanari-frameworks\packages\kanari-system\build\KanariSystem\bytecode_modules"
```

## System Modules Loaded

When using `new_with_kanari_natives()`, these modules are automatically published:

### Core Modules (0x2::*)

- ✅ `tx_context` - Transaction context and sender info
- ✅ `object` - Object system with UID/ID
- ✅ `balance` - Balance management  
- ✅ `coin` - Coin type and operations
- ✅ `transfer` - Object transfer functions
- ✅ `url` - URL type
- ✅ `kanari` - KANARI coin type

### Crypto Wrapper Modules (0x2::*)

- ✅ `ecdsa_k1` - secp256k1 signature verification
- ✅ `ecdsa_r1` - P-256 signature verification
- ✅ `ed25519` - Ed25519 signature verification

## Example: Using System Modules

```rust
use kanari_move_runtime::{BlockchainEngine, MoveRuntime};
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::ModuleId;

fn main() -> anyhow::Result<()> {
    // Create engine with system modules loaded
    let engine = BlockchainEngine::new()?;
    
    // Or create standalone runtime
    let mut runtime = MoveRuntime::new_with_kanari_natives()?;
    
    println!("System modules loaded successfully!");
    
    // Now you can execute functions from 0x2::transfer, 0x2::coin, etc.
    let transfer_module = ModuleId::new(
        AccountAddress::from_hex_literal("0x2")?,
        "transfer".parse()?
    );
    
    // Execute transfer function
    let result = runtime.execute_entry_function(
        &transfer_module,
        "public_transfer",
        vec![/* type args */],
        vec![/* args */],
        Some(sender),
        Some((1_000_000, 100))
    )?;
    
    println!("Transfer executed: {} gas used", result.gas_used);
    
    Ok(())
}
```

## Troubleshooting

### Modules Not Found

If you see warnings like:

```
Warning: Kanari system modules not found at "..."
```

**Solutions:**

1. **Build the framework first:**

   ```bash
   cd crates/kanari-frameworks
   cargo run --bin kanari-framework-builder
   ```

2. **Check the path:**

   ```bash
   ls crates/kanari-frameworks/packages/kanari-system/build/KanariSystem/bytecode_modules
   ```

3. **Set KANARI_FRAMEWORK_PATH:**
   Point to the correct bytecode_modules directory

### Module Load Failures

If specific modules fail to load:

```
Warning: Failed to load transfer.mv: ...
```

This might indicate:

- Dependency order issues (modules should load in order)
- Incompatible bytecode versions
- Missing dependencies

**Solution:** Rebuild the framework with matching Move version.

## Manual Loading (Advanced)

If you need to manually load modules:

```rust
let mut runtime = MoveRuntime::new_with_natives(vec![natives], true)?;

// Read module bytecode
let module_bytes = std::fs::read("path/to/module.mv")?;

// Publish to 0x2
let system_addr = AccountAddress::from_hex_literal("0x2")?;
runtime.publish_module(module_bytes, system_addr, None)?;
```

## Module Dependencies

Load order matters! Dependencies must be loaded first:

1. **tx_context** - No dependencies
2. **object** - Depends on tx_context
3. **balance** - Depends on object
4. **coin** - Depends on balance
5. **transfer** - Depends on object
6. **kanari** - Depends on coin
7. **Crypto modules** - No dependencies (use natives)

The `load_system_modules()` function handles this automatically.

## Verification

To verify modules are loaded:

```rust
let runtime = MoveRuntime::new_with_kanari_natives()?;

// Check if transfer module exists
let transfer_id = ModuleId::new(
    AccountAddress::from_hex_literal("0x2")?,
    "transfer".parse()?
);

if runtime.has_module(&transfer_id) {
    println!("✓ Transfer module loaded");
} else {
    println!("✗ Transfer module not found");
}
```

## Best Practices

1. **Always use `new_with_kanari_natives()`** for production code
2. **Build framework before running** to ensure modules exist
3. **Check warnings** during runtime initialization
4. **Set KANARI_FRAMEWORK_PATH** if using non-standard locations
5. **Verify module availability** before executing transactions

## Related Documentation

- [MOVE_VM_INTEGRATION.md](./MOVE_VM_INTEGRATION.md) - Full integration guide
- [Kanari System Modules](../kanari-frameworks/packages/kanari-system/docs/) - Module documentation
- [Move CLI Guide](./MOVE_CLI_GUIDE.md) - Building and testing modules
