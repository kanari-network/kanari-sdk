# Quick Start Guide - Kanari Packages

Get started with the Kanari Move package management tool in 5 minutes.

## Prerequisites

- Rust toolchain installed (`rustc`, `cargo`)
- Move language knowledge (basic)

## Installation

```bash
cd c:\Users\Pukpuy\Desktop\kanari-cp\crates\packages
cargo build --release
```

## Basic Usage

### 1. Compile All Packages

```bash
cargo run --release -- build --version 1
```

**What it does:**

- Compiles Move source code to bytecode
- Creates `released/1/{address}/package.rpd` files
- Output is JSON with hex-encoded bytecode

**Expected output:**

```
🚀 Kanari Package Compiler
Compiling MoveStdlib (0x1)...
  ✓ Compiled 11 modules
✅ Successfully compiled MoveStdlib

Compiling KanariSystem (0x2)...
  ✓ Compiled 1 modules
✅ Successfully compiled KanariSystem

✨ Compilation Summary:
   ✅ Successful: 2
```

### 2. Generate Documentation

Generate docs for all packages:

```bash
cargo run --release -- docs
```

Generate docs for one package:

```bash
cargo run --release -- docs --package KanariSystem
```

**What it does:**

- Creates Markdown documentation in `{package}/docs/`
- Includes module docs, function signatures, constants
- Auto-generated from Move doc comments

### 3. View Results

Check compiled output:

```bash
# View package.rpd (JSON format)
Get-Content released/1/0x1/package.rpd | Select-Object -First 30

# List documentation files
Get-ChildItem move-stdlib/docs/
```

## Adding a New Package

### Step 1: Create Package Structure

```bash
# Create directory
mkdir kanari-system

# Create Move.toml
@"
[package]
name = "KanariSystem"
version = "1.0.0"

[addresses]
KanariSystem = "0x2"

[dependencies]
MoveStdlib = { local = "../move-stdlib" }
"@ | Out-File -FilePath kanari-system/Move.toml

# Create sources directory
mkdir kanari-system/sources
```

### Step 2: Add Configuration

Edit `src/packages_config.rs`:

```rust
const PACKAGES: &[PackageConfig] = &[
    PackageConfig { name: "MoveStdlib", directory: "move-stdlib", address: "0x1" },
    PackageConfig { name: "KanariSystem", directory: "kanari-system", address: "0x2" },
    // Add your package here
    PackageConfig { name: "YourPackage", directory: "your-package", address: "0x3" },
];
```

Edit `src/main.rs` in `get_doc_configs()`:

```rust
vec![
    PackageDocConfig::new("move-stdlib")
        .with_address("std", "0x1"),
    PackageDocConfig::new("kanari-system")
        .with_address("KanariSystem", "0x2")
        .with_dependency("MoveStdlib", "0x1"),
    // Add your package here
    PackageDocConfig::new("your-package")
        .with_address("YourPackage", "0x3")
        .with_dependency("MoveStdlib", "0x1"),
]
```

### Step 3: Build and Test

```bash
# Compile
cargo run --release -- build --version 1

# Generate docs
cargo run --release -- docs --package YourPackage

# Verify output
Get-Content released/1/0x3/package.rpd
```

## Common Tasks

### Rebuild Everything

```bash
cargo clean
cargo build --release
cargo run --release -- build --version 1
```

### Update Documentation

```bash
# After modifying Move doc comments
cargo run --release -- docs
```

### Check Package Info

```bash
# View package metadata
Get-Content kanari-system/Move.toml

# List source files
Get-ChildItem kanari-system/sources/
```

## Output Explained

### package.rpd Format

```json
{
  "package": "MoveStdlib",
  "version": "1",
  "modules": [
    {
      "name": "vector",
      "address": "0000000000000000000000000000000000000000000000000000000000000001",
      "bytecode": "a11ceb0b0600000007010002..."
    }
  ]
}
```

- Human-readable JSON (not binary)
- Bytecode encoded as hex string
- Can be version-controlled in Git

### Documentation Structure

```
kanari-system/
├── docs/
│   ├── overview.md          # Package overview
│   ├── transfer.md          # Module documentation
│   └── dependencies/
│       └── move-stdlib/
│           └── vector.md    # Dependency docs
└── doc_templates/
    ├── overview.md          # Template for overview
    └── references.md        # Template for references
```

## CLI Reference

### Build Command

```bash
cargo run --release -- build --version <VERSION>
```

**Arguments:**

- `--version <VERSION>`: Release version number (e.g., 1, 2, 3)

**Example:**

```bash
cargo run --release -- build --version 1
```

### Docs Command

```bash
cargo run --release -- docs [--package <NAME>]
```

**Options:**

- `--package <NAME>`: Optional, generate docs for specific package only

**Examples:**

```bash
# All packages
cargo run --release -- docs

# One package
cargo run --release -- docs --package MoveStdlib
```

## Troubleshooting

### Build Fails

**Problem:** Compilation errors in Move code

**Solution:**

```bash
# Check Move syntax
cat kanari-system/sources/transfer.move

# Verify Move.toml is correct
cat kanari-system/Move.toml
```

### Missing Dependencies

**Problem:** Can't find MoveStdlib

**Solution:**

```bash
# Ensure move-stdlib exists
Get-ChildItem move-stdlib/

# Check Move.toml dependencies section
cat kanari-system/Move.toml
```

### Documentation Not Generated

**Problem:** No docs/ directory created

**Solution:**

```bash
# Create doc_templates first
mkdir kanari-system/doc_templates
echo "# Overview" > kanari-system/doc_templates/overview.md
echo "# References" > kanari-system/doc_templates/references.md

# Then generate
cargo run --release -- docs --package KanariSystem
```

## Next Steps

1. **Explore Examples**: Check `move-stdlib/sources/` for Move code examples
2. **Read Documentation**: See generated docs in `{package}/docs/`
3. **Write Move Code**: Add modules to `kanari-system/sources/`
4. **Test Compilation**: Use `cargo run --release -- build --version 1`
5. **Customize Docs**: Edit templates in `doc_templates/`

## Additional Resources

- **Move Language Book**: <https://move-language.github.io/move/>
- **Move Tutorial**: <https://github.com/move-language/move/tree/main/language/documentation/tutorial>
- **Move Examples**: Check `move-stdlib/sources/` directory

## Tips

- Always use `--release` flag for faster compilation
- Commit `package.rpd` files to Git for version control
- Update docs after changing Move code
- Use descriptive module names in Move.toml
- Keep addresses sequential (0x1, 0x2, 0x3, ...)

## Support

For issues or questions:

1. Check error messages carefully
2. Verify Move.toml syntax
3. Ensure all dependencies exist
4. Review this quickstart guide
5. Check main README.md for details
