# Kanari Packages - Universal Move Package Management Tool

A unified tool for compiling, building, and generating documentation for Move language packages in the Kanari ecosystem.

## Overview

This tool provides a centralized system to manage multiple Move packages without code duplication. It supports:

- **Package Compilation**: Compile Move source code to bytecode
- **Documentation Generation**: Auto-generate Markdown documentation from Move code
- **Multi-Package Support**: Handle multiple packages with a single configuration
- **Human-Readable Output**: JSON format with hex-encoded bytecode

## Architecture

```
crates/packages/
├── src/
│   ├── main.rs              # CLI entry point with subcommands
│   ├── compiler.rs          # Package compilation logic
│   ├── doc_generator.rs     # Documentation generation
│   └── packages_config.rs   # Package configuration
├── move-stdlib/             # Move Standard Library (address 0x1)
├── kanari-system/           # Kanari System Package (address 0x2)
└── released/                # Compiled output
    └── {version}/
        └── {address}/
            └── package.rpd  # JSON with hex-encoded bytecode
```

## Quick Start

### Build Packages

Compile all packages to bytecode:

```bash
cargo run --release -- build
```

This creates `released/latest/{address}/package.rpd` files by default.

Or specify a version:

```bash
cargo run --release -- build --version 1
```

This creates `released/1/{address}/package.rpd` files containing:

- Package metadata (name, version)
- Compiled modules with hex-encoded bytecode
- Module names and addresses

### Generate Documentation

Generate documentation for all packages:

```bash
cargo run --release -- docs
```

Generate documentation for a specific package:

```bash
cargo run --release -- docs --package KanariSystem
```

Documentation is generated in `{package}/docs/` directories.

## Package Configuration

Packages are configured in `src/packages_config.rs`:

```rust
const PACKAGES: &[PackageConfig] = &[
    PackageConfig {
        name: "MoveStdlib",
        directory: "move-stdlib",
        address: "0x1"
    },
    PackageConfig {
        name: "KanariSystem",
        directory: "kanari-system",
        address: "0x2"
    },
];
```

### Adding a New Package

1. **Create package directory structure:**

   ```
   packages/
   └── your-package/
       ├── Move.toml
       ├── sources/
       │   └── your_module.move
       └── doc_templates/
           ├── overview.md
           └── references.md
   ```

2. **Add to configuration** in `src/packages_config.rs`:

   ```rust
   PackageConfig {
       name: "YourPackage",
       directory: "your-package",
       address: "0x3"
   }
   ```

3. **Add documentation config** in `src/main.rs`:

   ```rust
   PackageDocConfig::new("your-package")
       .with_address("YourPackage", "0x3")
       .with_dependency("MoveStdlib", "0x1"),
   ```

4. **Build and generate docs:**

   ```bash
   cargo run --release -- build
   cargo run --release -- docs
   ```

## Output Format

### package.rpd Structure

```json
{
  "package": "MoveStdlib",
  "version": "1",
  "modules": [
    {
      "name": "vector",
      "address": "0000000000000000000000000000000000000000000000000000000000000001",
      "bytecode": "a11ceb0b0600000007010002030206050807..."
    }
  ]
}
```

- **package**: Package name
- **version**: Release version
- **modules**: Array of compiled modules
  - **name**: Module name
  - **address**: 64-character hex address (padded)
  - **bytecode**: Hex-encoded compiled bytecode

## Commands

### `build`

Compile all packages to bytecode.

**Usage:**

```bash
cargo run --release -- build [--version <VERSION>]
```

**Options:**

- `--version <VERSION>`: Release version number (default: "latest")

**Examples:**

```bash
# Build latest version
cargo run --release -- build

# Build specific version
cargo run --release -- build --version 1
```

**Output:**

- Creates `released/{version}/{address}/package.rpd` files
- JSON format with hex-encoded bytecode
- Human-readable and version-controllable

### `docs`

Generate documentation from Move source code.

**Usage:**

```bash
cargo run --release -- docs [OPTIONS]
```

**Options:**

- `--package <NAME>`: Generate docs for specific package only

**Output:**

- Markdown files in `{package}/docs/` directory
- Includes module documentation, function signatures, and constants
- Auto-generated from Move doc comments

## Helper Functions

The codebase includes helper functions to reduce complexity:

### `get_package_name(path: &Path) -> Result<String>`

Extract package name from Move.toml file.

### `collect_move_files(dir: &Path) -> Vec<PathBuf>`

Recursively collect all .move files in a directory.

### `is_stdlib(config: &PackageConfig) -> bool`

Check if package is the Move Standard Library.

### `load_stdlib_dependencies(root: &Path) -> Result<Vec<String>>`

Load standard library Move files as dependencies.

### `get_named_addresses(config: &PackageConfig) -> BTreeMap<String, AccountAddress>`

Parse named addresses for a package.

## Dependencies

- **move-compiler**: Compile Move source to bytecode
- **move-binary-format**: Work with compiled Move bytecode
- **move-docgen**: Generate documentation from Move code
- **move-errmapgen**: Generate error descriptions
- **serde/serde_json**: JSON serialization
- **hex**: Encode/decode bytecode as hex strings
- **clap**: CLI argument parsing

## Development

### Project Structure

```rust
// Main CLI
main.rs
  ├── build_packages()      // Compile all packages
  ├── generate_docs()       // Generate documentation
  └── get_doc_configs()     // Documentation configuration

// Compilation
compiler.rs
  ├── compile_package()     // Main compilation logic
  └── Helper functions      // Utilities for compilation

// Documentation
doc_generator.rs
  └── generate_documentation()  // Generate Markdown docs

// Configuration
packages_config.rs
  └── PACKAGES             // Package definitions
```

### Testing

Build and verify output:

```bash
# Compile packages (latest version)
cargo run --release -- build

# Check output
Get-Content released/latest/0x1/package.rpd | Select-Object -First 20

# Compile specific version
cargo run --release -- build --version 1

# Check output
Get-Content released/1/0x1/package.rpd | Select-Object -First 20

# Generate docs
cargo run --release -- docs

# Verify docs
Get-ChildItem move-stdlib/docs/
```

## Error Handling

The tool provides clear error messages:

- **Missing Move.toml**: Package directory must contain Move.toml
- **Compilation errors**: Shows Move compiler errors with line numbers
- **Invalid addresses**: Address format must be valid hex (e.g., "0x1")
- **Missing dependencies**: Checks for required dependency packages

## Best Practices

1. **Version Control**: Commit `package.rpd` files for reproducibility
2. **Documentation**: Update `doc_templates/` when adding modules
3. **Testing**: Verify compilation after configuration changes
4. **Naming**: Use PascalCase for package names in configuration
5. **Addresses**: Use sequential addresses (0x1, 0x2, 0x3, ...)

## Troubleshooting

### Compilation Fails

```bash
# Check Move.toml syntax
cat kanari-system/Move.toml

# Verify source files exist
Get-ChildItem kanari-system/sources/
```

### Documentation Not Generated

```bash
# Check doc_templates exist
Get-ChildItem kanari-system/doc_templates/

# Generate with verbose output
cargo run --release -- docs --package KanariSystem
```

### Invalid JSON Output

```bash
# Verify hex crate is installed
cargo tree | Select-String "hex"

# Rebuild from scratch
cargo clean; cargo build --release
```

## License

Part of the Kanari cryptocurrency project.
