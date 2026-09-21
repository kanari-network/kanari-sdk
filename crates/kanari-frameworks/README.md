# Kanari Frameworks - Move Package Management Tool

A unified tool for compiling, packaging, and documenting the Move language
packages of the Kanari ecosystem.

## Overview

This tool manages multiple Move packages from a single configuration:

- **Package compilation**: compile Move sources to bytecode and pack them
  into versioned binary artifacts
- **Documentation generation**: auto-generate Markdown docs and error maps
  from Move doc comments
- **Multi-package support**: one package list drives builds and docs
- **Binary output**: deterministic `package.rpd` artifacts the node loads
  as its genesis framework

## Architecture

```text
crates/kanari-frameworks/
  src/
    main.rs              # CLI entry point (build/docs subcommands)
    compiler.rs          # Package compilation and .rpd packing
    doc_generator.rs     # Documentation and error-map generation
    packages_config.rs   # Package list (single registration point)
  packages/
    move-stdlib/         # Move Standard Library (address 0x1)
    kanari-system/       # Kanari System Package (address 0x2)
  released/              # Compiled output
    {version}/
      {address}/
        package.rpd      # Binary artifact (bincode + magic header)
```

## Quick Start

Run commands from the repository root. The tool finds the Move `packages/`
directory automatically (it skips lookalike directories that do not contain
`move-stdlib` and `kanari-system`).

### Build Packages

Compile all packages to bytecode:

```powershell
cargo run --release -p kanari-frameworks -- build
```

This creates `released/latest/{address}/package.rpd` files by default.

Or specify a version:

```powershell
cargo run --release -p kanari-frameworks -- build --version 11
```

This creates `released/11/{address}/package.rpd` files containing:

- Package metadata (name, version, timestamp)
- Compiled modules with raw bytecode bytes

The process exits nonzero if any package fails; errors are printed to stderr.

### Generate Documentation

Generate documentation for all packages:

```powershell
cargo run --release -p kanari-frameworks -- docs
```

Generate documentation for a specific package:

```powershell
cargo run --release -p kanari-frameworks -- docs --package KanariSystem
```

Documentation is generated in `packages/{package}/docs/` directories, with an
error map at `packages/{package}/error_description.errmap`.

## Package Configuration

Packages are configured in exactly one place,
`src/packages_config.rs`:

```rust
const PACKAGES: &[PackageConfig] = &[
    PackageConfig {
        name: "MoveStdlib",
        directory: "move-stdlib",
        address: Address::STD_ADDRESS,
        address_name: "std",
    },
    PackageConfig {
        name: "KanariSystem",
        directory: "kanari-system",
        address: Address::KANARI_SYSTEM_ADDRESS,
        address_name: "kanari_system",
    },
    // Add new packages here:
    // PackageConfig { name: "MyPackage", directory: "my-package", address: "0x3", address_name: "my_package" },
];
```

Both `build` and `docs` read this list. Non-stdlib packages automatically
depend on `move-stdlib`; no second registration step exists.

### Adding a New Package

1. **Create package directory structure** (paths relative to
   `crates/kanari-frameworks/packages/`):

   ```text
   packages/
     your-package/
       Move.toml
       sources/
         your_module.move
       tests/
         your_module_tests.move
       doc_templates/
         overview.md
         references.md
   ```

   Source files may live in subdirectories of `sources/` (for example
   `sources/crypto/`); collection is recursive. Unit tests belong in
   `tests/` following the standard Move package layout - they are not
   compiled into the release artifact, which only packs `sources/`.

   Minimal `Move.toml`:

   ```toml
   [package]
   name = "MyPackage"

   [addresses]
   my_package = "0x3"
   std = "0x1"

   [dependencies]
   MoveStdlib = { local = "../move-stdlib" }
   ```

2. **Register the package** in `src/packages_config.rs` (see snippet above).

3. **Build and generate docs:**

   ```powershell
   cargo run --release -p kanari-frameworks -- build --version 11
   cargo run --release -p kanari-frameworks -- docs --package MyPackage
   ```

## Output Format

### package.rpd Structure

`package.rpd` is a **binary** file:

- Magic header bytes: `KANARI_RPD\0v1\n`
- Payload: bincode-serialized record with `package` name, `version`,
  `timestamp`, and a `modules` list of `{ name, address, bytecode }`,
  where `bytecode` is raw compiled bytes

Fields:

- **package**: package name
- **version**: release version string
- **timestamp**: Unix timestamp of the build
- **modules**: array of compiled modules
  - **name**: module name
  - **address**: publisher address
  - **bytecode**: raw compiled module bytes

## Commands

### `build`

Compile all packages to bytecode.

**Usage:**

```powershell
cargo run --release -p kanari-frameworks -- build [--version <VERSION>]
```

**Options:**

- `--version <VERSION>`: release version directory to write (default: `latest`)

**Examples:**

```powershell
# Build latest version
cargo run --release -p kanari-frameworks -- build

# Build specific version
cargo run --release -p kanari-frameworks -- build --version 11
```

**Output:**

- Creates `released/{version}/{address}/package.rpd` files (binary format)
- Prints failures to stderr and exits nonzero on error

### `docs`

Generate documentation from Move source code.

**Usage:**

```powershell
cargo run --release -p kanari-frameworks -- docs [--package <NAME>]
```

**Options:**

- `--package <NAME>`: generate docs for one package only
  (for example `MoveStdlib` or `KanariSystem`)

**Output:**

- Markdown files in `packages/{package}/docs/` directory
- Error map at `packages/{package}/error_description.errmap`
- Covers module documentation, function signatures, and constants

## Helper Functions

### `compile_package(package_dir, output_dir, version, address) -> Result<PathBuf>`

Main compilation entry point in `compiler.rs`: collects sources, compiles
with dependencies, packs the `.rpd` file, and returns its path.

### `collect_move_files(dir: &Path) -> Result<Vec<PathBuf>>`

Recursively collect all `.move` files under a directory, sorted for
deterministic builds. Recursion matters: modules in subdirectories such as
`sources/crypto/` would otherwise be silently dropped and surface later as
`E03002 unbound module` errors.

### `is_stdlib(address: &str) -> Result<bool>`

Check whether an address is the Move Standard Library address (`0x1`).
Stdlib packages compile with no dependencies.

### `load_stdlib_dependencies(package_dir: &Path) -> Result<Vec<PathBuf>>`

Collect `../move-stdlib/sources` files (also recursive) as compile
dependencies for non-stdlib packages.

### `get_named_addresses() -> BTreeMap<Symbol, NumericalAddress>`

Build the named-address table (`std`, `kanari_system`, ...) from the package
list for the Move compiler.

## Dependencies

- **move-compiler**: compile Move source to bytecode
- **move-binary-format**: serialize compiled modules
- **move-docgen / move-errmapgen**: documentation and error maps
- **move-command-line-common / move-symbol-pool**: compiler address and symbol handling
- **bincode**: binary serialization of `package.rpd`
- **hex / serde**: hex helpers and struct serialization
- **clap**: CLI argument parsing
- **anyhow / log**: error handling and logging

## Development

### Project Structure

```text
// Main CLI
main.rs
  build_packages()      // Compile all packages, nonzero exit on failure
  generate_docs()       // Generate documentation
  get_doc_configs()     // Documentation configuration from package list
  get_packages_dir()    // Locate Move packages dir (validates contents)

// Compilation
compiler.rs
  compile_package()     // Main compilation and packing logic
  collect_move_files()  // Recursive .move file collection

// Documentation
doc_generator.rs
  generate_documentation()  // Generate Markdown docs and error map

// Configuration
packages_config.rs
  PACKAGES              // Single package registry
```

### Testing

```powershell
# Rust unit tests (includes a regression test for recursive collection)
cargo test -p kanari-frameworks

# Compile packages (version 11)
cargo run --release -p kanari-frameworks -- build --version 11

# Check output artifacts exist
Get-ChildItem crates/kanari-frameworks/released/11/ -Recurse -Filter package.rpd |
    Select-Object FullName, Length

# Generate docs
cargo run --release -p kanari-frameworks -- docs
```

## Error Handling

The tool reports failures through stderr and a nonzero exit code:

- **Missing Move.toml / sources**: package directory must contain both
- **Compilation errors**: Move compiler diagnostics with file and line numbers
- **Invalid addresses**: address must be valid hex (for example `0x1`)
- **Missing dependencies**: required dependency packages must exist
- **Wrong packages directory**: candidates that do not contain `move-stdlib`
  and `kanari-system` are skipped so an unrelated `packages/` folder can
  never shadow the Move packages

## Best Practices

1. **Version control**: commit `package.rpd` files for reproducibility
2. **Documentation**: update `doc_templates/` when adding modules
3. **Testing**: keep unit tests in the package `tests/` directory, not in `sources/`
4. **Naming**: use PascalCase for package names in configuration
5. **Addresses**: use sequential addresses (`0x1`, `0x2`, `0x3`, ...)
6. **Subdirectories**: freely organize `sources/` into subdirectories;
   collection is recursive, but module names (`kanari_system::<name>`)
   must still be unique package-wide

## Troubleshooting

### Compilation Fails With E03002 Unbound Module

```powershell
# The imported module's file must exist under sources/ (subdirectories are fine)
Get-ChildItem crates/kanari-frameworks/packages/kanari-system/sources/ -Recurse -Filter *.move |
    Select-Object FullName
```

Check that the module declaration (`module kanari_system::<name>`) matches
the name used in `use` statements.

### Build Reports "Directory not found"

```powershell
# Run from the repository root or crates/kanari-frameworks
Get-ChildItem crates/kanari-frameworks/packages/
```

Every package fails this way when the tool resolves an unrelated `packages/`
directory. Current versions validate candidates; older binaries did not.

### Documentation Not Generated

```powershell
# Check doc_templates exist
Get-ChildItem crates/kanari-frameworks/packages/kanari-system/doc_templates/

# Generate with explicit package selection
cargo run --release -p kanari-frameworks -- docs --package KanariSystem
```

### Stale Binary Behavior

```powershell
# Rebuild the binary before running it directly
cargo build --release -p kanari-frameworks
./target/release/kanari-frameworks.exe build --version 11
```

(`cargo run --release -p kanari-frameworks -- ...` always rebuilds first and
does not have this problem.)

## License

Part of the Kanari cryptocurrency project.
