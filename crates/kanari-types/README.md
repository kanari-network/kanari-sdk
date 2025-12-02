# Kanari Types - Rust Bindings for Move Modules

[![Tests](https://img.shields.io/badge/tests-passing-brightgreen)](./validate-sync.ps1)
[![Sync Status](https://img.shields.io/badge/sync-complete-blue)](./SYNC_STATUS.md)

Rust type definitions และ utilities สำหรับ Kanari Move modules พร้อม type-safe bindings

## ⚡ Quick Start

```powershell
# run tests
cargo test

# validate sync status
.\validate-sync.ps1

# Build
cargo build --release
```

## 📦 Modules

### Core System Modules (`kanari_system::*`)

| Move Module | Rust Module | Status |
|-------------|-------------|--------|
| `balance.move` | `balance.rs` | ✅ |
| `coin.move` | `coin.rs` | ✅ |
| `object.move` | `object.rs` | ✅ |
| `transfer.move` | `transfer.rs` | ✅ |
| `tx_context.move` | `tx_context.rs` | ✅ |
| `kanari.move` | `kanari.rs` | ✅ |

### Standard Library (`std::*`)

| Move Module | Rust Module | Status |
|-------------|-------------|--------|
| `signer.move` | `stdlib/signer.rs` | ✅ |
| `option.move` | `stdlib/option.rs` | ✅ |
| `string.move` | `stdlib/string.rs` | ✅ |
| `ascii.move` | `stdlib/ascii.rs` | ✅ |
| `vector.move` | `stdlib/vector.rs` | ✅ |

## 🔧 Usage Examples

### Creating Currency Metadata

```rust
use kanari_types::coin::CurrencyMetadata;

let metadata = CurrencyMetadata::new(
    b"KANARI".to_vec(),
    b"Kanari Coin".to_vec(),
    b"Native token".to_vec(),
    9u8,                    // decimals
    None,                   // icon_url
);
```

### Working with Transfers

```rust
use kanari_types::transfer::TransferRecord;

let transfer = TransferRecord::from_hex_literals(
    "0x1",      // from
    "0x2",      // to
    1000,       // amount
)?;
```

### Module Registry

```rust
use kanari_types::module_registry::{ModuleRegistry, ModuleCallBuilder};

// Get module ID
let module_id = ModuleRegistry::get_module_id("coin")?;

// Check function exists
let exists = ModuleRegistry::function_exists("coin", "mint");

// Build function call
let call = ModuleCallBuilder::new("kanari")
    .function("transfer")
    .validate()?;
```

## 🛡️ Type Safety

All Rust types match Move module structures:

```rust
// Move: struct Coin<phantom T> has store, drop
pub struct CoinRecord { pub value: u64 }

// Move: struct CoinMetadata<phantom T> has key, store, drop
pub struct CurrencyMetadata {
    pub symbol: Vec<u8>,
    pub name: Vec<u8>,
    pub description: Vec<u8>,
    pub decimals: u8,
    pub icon_url: Option<Vec<u8>>,
}
```

## 🔄 Sync Workflow

### When Updating Move Modules

1. **Update Move code**

   ```bash
   cd crates/kanari-frameworks/packages/kanari-system
   # Edit .move files
   move-cli test
   ```

2. **Update Rust types**

   ```bash
   cd crates/kanari-types
   # Edit corresponding .rs files
   cargo test
   ```

3. **Validate sync**

   ```powershell
   .\validate-sync.ps1
   ```

4. **Update documentation**
   - Update `SYNC_STATUS.md`
   - Update `module_registry.rs` if functions changed

### Critical Files to Update

| Move Change | Rust Files to Update |
|-------------|---------------------|
| New struct field | Matching struct in `src/` |
| New function | `module_registry.rs` + module's `function_names()` |
| New module | `module_registry.rs` + new `src/{module}.rs` |
| Return type change | Function signature documentation |

## 📋 Testing

```powershell
# Unit tests
cargo test --lib

# Integration tests  
cargo test --test integration_test

# Doc tests
cargo test --doc

# All tests with coverage
cargo test

# Move tests
cd ../kanari-frameworks/packages/kanari-system
move-cli test
```

## 🚨 Error Prevention

### Type Mismatches

❌ **Bad:** Rust type missing Move field

```rust
// Move has: decimals: u8
// Rust missing decimals field
pub struct CurrencyMetadata {
    pub symbol: Vec<u8>,
    // Missing: pub decimals: u8,
}
```

✅ **Good:** All fields present

```rust
pub struct CurrencyMetadata {
    pub symbol: Vec<u8>,
    pub decimals: u8,  // ✓ Matches Move
}
```

### Function Registry

❌ **Bad:** Missing function in registry

```move
// Move: public fun new_function()
```

```rust
// Rust: function NOT in module_registry.rs
```

✅ **Good:** All functions registered

```rust
Self::COIN => vec![
    "create_currency",
    "new_function",  // ✓ Added
    // ...
]
```

## 🔍 Validation Script

`validate-sync.ps1` checks:

- ✅ Rust tests pass
- ✅ Move tests pass  
- ✅ All modules registered
- ✅ Critical types present
- ✅ Required fields exist

```powershell
# Run validation
.\validate-sync.ps1

# Expected output:
# SUCCESS: All checks passed
```

## 📚 Documentation

- **[SYNC_STATUS.md](./SYNC_STATUS.md)** - Current sync status
- **Module docs** - Run `cargo doc --open`
- **Examples** - See `examples/` directory

## 🤝 Contributing

1. Update Move modules first
2. Run Move tests
3. Update Rust types
4. Run Rust tests
5. Run validation script
6. Update SYNC_STATUS.md

## 📄 License

Part of Kanari blockchain project.
