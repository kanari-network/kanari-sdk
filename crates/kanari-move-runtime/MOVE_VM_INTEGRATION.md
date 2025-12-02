# Full Move VM Integration Guide

## Overview

The Kanari SDK now provides comprehensive Move VM integration with support for:

- ✅ Custom native functions (crypto operations, system calls)
- ✅ Proper gas metering and accounting
- ✅ Module verification and dependency resolution
- ✅ Script execution support
- ✅ Session management with atomicity guarantees
- ✅ Read-only queries and simulation
- ✅ Gas estimation

## Architecture

### Core Components

1. **MoveRuntime**: Enhanced VM wrapper with native function support
2. **MoveVMState**: Persistent storage for published modules
3. **MoveRuntimeExtensions**: Advanced features (scripts, verification, simulation)
4. **BlockchainEngine**: Integrated blockchain with Move VM execution

## Features

### 1. Native Function Support

The runtime now supports custom native functions for extended functionality:

```rust
use kanari_move_runtime::MoveRuntime;
use kanari_crypto::move_natives;
use move_core_types::account_address::AccountAddress;

// Create runtime with Kanari system natives (crypto + stdlib)
// This also pre-loads all 0x2::* system modules
let runtime = MoveRuntime::new_with_kanari_natives()?;

// Or create with custom natives
let system_addr = AccountAddress::from_hex_literal("0x2")?;
let natives = move_natives::all_natives(system_addr);
let runtime = MoveRuntime::new_with_natives(vec![natives], true)?;
```

**System Modules Auto-Loaded:**

When using `new_with_kanari_natives()`, these modules are automatically published to the VM:

- `0x2::tx_context` - Transaction context
- `0x2::object` - Object system (UID, ID)
- `0x2::balance` - Balance management
- `0x2::coin` - Coin operations
- `0x2::transfer` - Transfer functions
- `0x2::kanari` - KANARI token
- `0x2::ecdsa_k1`, `0x2::ecdsa_r1`, `0x2::ed25519` - Crypto wrappers

This prevents `LINKER_ERROR: Cannot find ModuleId` errors. See [SYSTEM_MODULES.md](./SYSTEM_MODULES.md) for details.

**Available Native Functions:**

#### ECDSA (secp256k1)

- `ecdsa_k1::ecrecover(signature, msg, hash_type) -> public_key`
- `ecdsa_k1::verify(signature, public_key, msg, hash_type) -> bool`
- `ecdsa_k1::decompress_pubkey(compressed_pubkey) -> uncompressed_pubkey`

#### ECDSA (P-256)

- `ecdsa_r1::native_verify(signature, public_key, msg, hash_type) -> bool`

#### Ed25519

- `ed25519::verify(signature, public_key, msg) -> bool`

### 2. Gas Metering

Full gas accounting with configurable metering:

```rust
// Enable gas metering
let runtime = MoveRuntime::new_with_natives(vec![], true)?;

// Publish module with gas accounting
let gas_limit = 1_000_000;
let gas_price = 100;
let changeset = runtime.publish_module(
    module_bytes,
    sender,
    Some((gas_limit, gas_price))
)?;

// Gas costs are automatically deducted from sender and credited to DAO
println!("Gas used: {}", changeset.gas_used);
```

**Gas Operations:**

- `PublishModule`: Base cost + per-byte cost
- `ExecuteFunction`: Base cost + complexity factor
- `CallContract`: Base cost + depth penalty
- `CreateAccount`: Fixed cost

### 3. Module Verification

Automatic verification before publishing:

```rust
let module_bytes = compile_module("my_module.move")?;
let compiled = CompiledModule::deserialize(&module_bytes)?;

// Verify module dependencies and correctness
runtime.verify_module(&compiled)?;

// Publish if verification passes
runtime.publish_module(module_bytes, sender, None)?;
```

**Verification Checks:**

- Module has valid self-identifier
- All dependencies are available or are stdlib/system modules
- Module size within limits
- No circular dependencies (future)

### 4. Script Execution

Execute Move scripts for complex operations:

```rust
// Compile Move script
let script_bytes = compile_script("my_script.move")?;

// Execute with type arguments and parameters
let changeset = runtime.execute_script(
    script_bytes,
    vec![],  // type_args
    vec![],  // args (BCS-encoded)
    sender,
    Some((1_000_000, 100))  // gas
)?;
```

**Use Cases for Scripts:**

- Multi-step operations
- Batch transactions
- Complex state transitions
- Admin operations

### 5. Session Management

Safe execution with rollback on errors:

```rust
// Sessions are automatically managed
// Changes only committed on success
let result = runtime.execute_entry_function(
    &module_id,
    "transfer",
    vec![],
    args,
    Some(sender),
    Some((gas_limit, gas_price))
);

match result {
    Ok(changeset) => {
        // Changes committed to storage
        state_manager.apply_changeset(changeset)?;
    }
    Err(e) => {
        // Automatic rollback - storage unchanged
        eprintln!("Execution failed: {}", e);
    }
}
```

### 6. Read-Only Queries

Query state without modifications:

```rust
// Create read-only session for queries
let session = runtime.create_readonly_session();

// Simulate execution without committing
let changeset = runtime.simulate_entry_function(
    &module_id,
    "get_balance",
    vec![],
    args
)?;

// Storage remains unchanged
```

### 7. Gas Estimation

Estimate gas costs before execution:

```rust
let estimated_gas = runtime.estimate_gas(
    &module_id,
    "transfer",
    vec![],
    args
)?;

println!("Estimated gas: {} units", estimated_gas);

// Use estimate for transaction
let gas_limit = estimated_gas * 2; // Add buffer
```

## Integration with BlockchainEngine

The `BlockchainEngine` now uses the enhanced `MoveRuntime`:

```rust
use kanari_move_runtime::BlockchainEngine;

// Create engine with full Move VM support
let engine = BlockchainEngine::new()?;

// Submit transactions with automatic VM execution
let signed_tx = create_signed_transaction(...)?;
let tx_hash = engine.submit_transaction(signed_tx)?;

// Mine block with Move VM execution
engine.mine_block()?;
```

## Example: Complete Workflow

```rust
use kanari_move_runtime::{BlockchainEngine, MoveRuntime};
use move_core_types::account_address::AccountAddress;

// 1. Create runtime with natives
let mut runtime = MoveRuntime::new_with_kanari_natives()?;

// 2. Compile and publish module
let module_bytes = std::fs::read("my_token.mv")?;
let sender = AccountAddress::from_hex_literal("0xCAFE")?;

let changeset = runtime.publish_module(
    module_bytes,
    sender,
    Some((10_000_000, 100))
)?;

println!("Module published, gas used: {}", changeset.gas_used);

// 3. Execute entry function
let module_id = ModuleId::new(sender, "MyToken".parse()?);
let args = vec![
    bcs::to_bytes(&recipient)?,
    bcs::to_bytes(&amount)?,
];

let changeset = runtime.execute_entry_function(
    &module_id,
    "transfer",
    vec![],
    args,
    Some(sender),
    Some((1_000_000, 100))
)?;

println!("Transfer executed, gas used: {}", changeset.gas_used);
```

## Advanced Features

### Module Bundle Publishing

Publish multiple modules atomically:

```rust
let modules = vec![module1_bytes, module2_bytes, module3_bytes];
runtime.publish_module_bundle(modules, sender)?;
```

### Ordered Module Publishing

Automatically resolve dependencies:

```rust
// Modules will be published in dependency order
runtime.publish_modules_ordered(modules)?;
```

### Module Inspection

Query published modules:

```rust
// Check if module exists
if runtime.has_module(&module_id) {
    // Get module bytecode
    let bytes = runtime.get_module_bytes(&module_id);
}

// List all modules
let all_modules = runtime.list_modules();
```

### Runtime Statistics

Get runtime configuration:

```rust
let stats = runtime.get_stats();
println!("Gas metering: {}", stats.gas_metering_enabled);
```

## Configuration

### Environment Variables

- `KANARI_MOVE_VM_DB`: Custom database path for Move VM state

### Gas Configuration

Customize gas costs in `gas.rs`:

```rust
pub struct GasConfig {
    pub base_cost: u64,
    pub per_byte_cost: u64,
    pub max_gas_limit: u64,
}
```

## Error Handling

All operations return `Result<T, anyhow::Error>`:

```rust
use anyhow::Context;

let result = runtime.execute_entry_function(...)
    .context("Failed to execute transfer function")?;
```

Common errors:

- `VM init error`: Failed to initialize Move VM
- `publish error`: Module verification or dependency issues
- `exec error`: Runtime execution failure
- `apply error`: Storage update failure
- `Insufficient balance for gas`: Sender lacks funds

## Best Practices

1. **Always verify modules** before publishing in production
2. **Use gas limits** to prevent runaway execution
3. **Simulate transactions** before executing for gas estimation
4. **Handle errors gracefully** with proper rollback
5. **Use read-only sessions** for queries to avoid side effects
6. **Enable gas metering** in production environments
7. **Monitor gas costs** and adjust pricing as needed

## Testing

```rust
#[test]
fn test_module_publishing() {
    let mut runtime = MoveRuntime::new().unwrap();
    let module = compile_test_module();
    
    let result = runtime.publish_module(
        module,
        test_address(),
        None
    );
    
    assert!(result.is_ok());
}
```

## Migration from Previous Version

If you were using the old `MoveRuntime::new()`:

```rust
// Old
let runtime = MoveRuntime::new()?;

// New (same for basic usage)
let runtime = MoveRuntime::new()?;

// Or with full features
let runtime = MoveRuntime::new_with_kanari_natives()?;
```

The API is backward compatible - existing code will continue to work.

## Performance Considerations

- **Module caching**: Published modules are cached in RocksDB
- **Session overhead**: Each transaction creates a new session (~1-5ms)
- **Gas metering**: Adds ~10-20% overhead when enabled
- **Storage I/O**: Batch operations when possible

## Roadmap

- [ ] Move bytecode verifier integration
- [ ] Module upgrade policies
- [ ] Multi-version concurrency control
- [ ] Event indexing and querying
- [ ] Storage rent and cleanup
- [ ] Parallel transaction execution
- [ ] JIT compilation for hot paths

## Resources

- [Move Language Documentation](https://move-language.github.io/move/)
- [Move VM Documentation](https://github.com/move-language/move)
- [Kanari SDK Documentation](../README.md)
- [Gas Metering Guide](./GAS_METERING.md)

## Support

For issues or questions:

- GitHub Issues: <https://github.com/jamesatomc/kanari-sdk_V2>
- Documentation: See MOVE_CLI_GUIDE.md
