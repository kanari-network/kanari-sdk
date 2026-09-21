# Quick Start Guide - Kanari Packages

Get started with the Kanari Move package management tool in 5 minutes.

## Prerequisites

- Rust toolchain installed (`rustc`, `cargo`)
- Basic Move language knowledge

## Installation

```powershell
cargo build --release -p kanari-frameworks
```

This produces `target/release/kanari-frameworks.exe` (or `kanari-frameworks` on Unix).

## Basic Usage

Run all commands from the repository root (`kanari-sdk/`). The tool locates
the Move packages directory automatically, whether you run from the repo root
or from `crates/kanari-frameworks`.

### 1. Compile All Packages

```powershell
cargo run --release -p kanari-frameworks -- build --version 11
```

**What it does:**

- Compiles every configured Move package (`move-stdlib` at `0x1`, `kanari-system` at `0x2`)
- Source files are collected recursively, so modules may live in
  subdirectories such as `sources/crypto/`
- Writes one binary artifact per package:
  `crates/kanari-frameworks/released/<version>/<address>/package.rpd`
- Exits nonzero if any package fails to compile; errors are printed to stderr

**Expected result:**

- `released/11/0x1/package.rpd` (MoveStdlib)
- `released/11/0x2/package.rpd` (KanariSystem, including `crypto/` modules
  such as `ecdsa_k1`, `ecdsa_r1`, `ed25519` and `signature_multisig`)

### 2. Generate Documentation

Generate docs for all packages:

```powershell
cargo run --release -p kanari-frameworks -- docs
```

Generate docs for one package:

```powershell
cargo run --release -p kanari-frameworks -- docs --package KanariSystem
```

**What it does:**

- Creates Markdown documentation in `{package}/docs/`
- Includes module docs, function signatures, and constants
- Auto-generated from Move doc comments; also writes
  `{package}/error_description.errmap`

### 3. View Results

Check compiled output:

```powershell
# Binary artifact (bincode with KANARI_RPD magic header, not plain text)
Get-ChildItem crates/kanari-frameworks/released/11/ -Recurse -Filter package.rpd |
    Select-Object FullName, Length

# List documentation files
Get-ChildItem crates/kanari-frameworks/packages/kanari-system/docs/
```

Unit tests live in each package's `tests/` directory (for example
`packages/move-stdlib/tests/hash_tests.move`) following the standard Move
package layout. They are not included in the release build, which compiles
only `sources/`.

## Adding a New Package

### Step 1: Create Package Structure

```powershell
# Create directories (paths relative to crates/kanari-frameworks/packages/)
New-Item -ItemType Directory crates/kanari-frameworks/packages/my-package/sources
New-Item -ItemType Directory crates/kanari-frameworks/packages/my-package/tests
```

Create `crates/kanari-frameworks/packages/my-package/Move.toml`:

```toml
[package]
name = "MyPackage"

[addresses]
my_package = "0x3"
std = "0x1"

[dependencies]
MoveStdlib = { local = "../move-stdlib" }
```

### Step 2: Register the Package

Add one entry in `crates/kanari-frameworks/src/packages_config.rs`
(the file already contains a commented template line showing the exact shape):

```rust
PackageConfig {
    name: "MyPackage",
    directory: "my-package",
    address: "0x3",
    address_name: "my_package",
},
```

Non-stdlib packages automatically depend on `move-stdlib`; no other
registration step is needed. Documentation picks up the new package
from the same list.

### Step 3: Build and Test

```powershell
# Compile (writes released/<version>/0x3/package.rpd on success)
cargo run --release -p kanari-frameworks -- build --version 11

# Generate docs
cargo run --release -p kanari-frameworks -- docs --package MyPackage
```

## Common Tasks

### Rebuild Everything

```powershell
cargo clean
cargo build --release -p kanari-frameworks
cargo run --release -p kanari-frameworks -- build --version 11
```

### Update Documentation

```powershell
# After modifying Move doc comments
cargo run --release -p kanari-frameworks -- docs
```

### Check Package Info

```powershell
# View package metadata
Get-Content crates/kanari-frameworks/packages/kanari-system/Move.toml

# List source files (including subdirectories)
Get-ChildItem crates/kanari-frameworks/packages/kanari-system/sources/ -Recurse -Filter *.move |
    Select-Object FullName
```

## Output Explained

### package.rpd Format

`package.rpd` is a **binary** file, not JSON:

- Magic header: `KANARI_RPD\0v1\n`
- Payload: bincode-serialized `KanariPackage` with `package` name, `version`,
  `timestamp`, and a `modules` list of `{ name, address, bytecode }`
  (bytecode is raw bytes, not a hex string)
- Committing `package.rpd` files to Git is the supported way to version
  releases; nodes load these artifacts as the genesis framework

### Documentation Structure

```text
packages/kanari-system/
  docs/                       # Generated output
    overview.md               # Package overview
    transfer.md               # Per-module documentation
    dependencies/
      move-stdlib/
  doc_templates/
    overview.md               # Template for overview
    references.md             # Template for references
```

## CLI Reference

### Build Command

```powershell
cargo run --release -p kanari-frameworks -- build --version <VERSION>
```

**Arguments:**

- `--version <VERSION>`: release version directory to write (for example `11`;
  defaults to `latest` when omitted)

### Docs Command

```powershell
cargo run --release -p kanari-frameworks -- docs [--package <NAME>]
```

**Options:**

- `--package <NAME>`: optional, generate docs for one package only
  (for example `MoveStdlib` or `KanariSystem`)

## Troubleshooting

### Build Fails With E03002 Unbound Module

**Problem:** errors like `Invalid 'use'. Unbound module: 'kanari_system::ecdsa_k1'`

**Cause:** the imported module's `.move` file is not passed to the compiler.
Source collection is recursive over `sources/`, so check that the file exists
under `sources/` (subdirectories are fine) and that the module name matches
`kanari_system::<name>`.

### Build Reports "Directory not found"

**Problem:** every package fails with `Directory not found`.

**Cause:** the tool resolved the wrong `packages/` directory. The repository
root contains an unrelated SDK `packages/` folder; the tool skips any
candidate that does not contain `move-stdlib` and `kanari-system`. Run from
the repository root or from `crates/kanari-frameworks` so the Move packages
directory is found.

### Missing Dependencies

**Problem:** `kanari-system` cannot resolve `std` or `MoveStdlib`.

**Solution:**

```powershell
# Ensure move-stdlib exists next to kanari-system
Get-ChildItem crates/kanari-frameworks/packages/

# Check the [dependencies] section of kanari-system/Move.toml
Get-Content crates/kanari-frameworks/packages/kanari-system/Move.toml
```

### Documentation Not Generated

**Problem:** no `docs/` directory created.

**Solution:**

```powershell
# Create doc_templates first
New-Item -ItemType Directory crates/kanari-frameworks/packages/kanari-system/doc_templates
"# Overview" | Out-File crates/kanari-frameworks/packages/kanari-system/doc_templates/overview.md
"# References" | Out-File crates/kanari-frameworks/packages/kanari-system/doc_templates/references.md

# Then generate
cargo run --release -p kanari-frameworks -- docs --package KanariSystem
```

### Stale Binary

**Problem:** behavior does not match recently changed Rust code in
`crates/kanari-frameworks/src/`.

**Solution:** rebuild the binary before running it directly:

```powershell
cargo build --release -p kanari-frameworks
./target/release/kanari-frameworks.exe build --version 11
```

(`cargo run --release -p kanari-frameworks -- ...` always rebuilds first and
does not have this problem.)

## Next Steps

1. **Explore examples**: check `packages/move-stdlib/sources/` for Move code examples
2. **Read documentation**: see generated docs in `packages/{package}/docs/`
3. **Write Move code**: add modules under `packages/kanari-system/sources/`
   (subdirectories allowed) and put unit tests in `packages/kanari-system/tests/`
4. **Test compilation**: use `cargo run --release -p kanari-frameworks -- build --version 11`
5. **Customize docs**: edit templates in `{package}/doc_templates/`

## Additional Resources

- **Move Language Book**: <https://move-language.github.io/move/>
- **Move Tutorial**: <https://github.com/move-language/move/tree/main/language/documentation/tutorial>
- **Move Examples**: check the `packages/move-stdlib/sources/` directory

## Tips

- Always use the `--release` flag for faster compilation
- Commit `package.rpd` files to Git for version control
- Update docs after changing Move code
- Use descriptive module names in `Move.toml`
- Keep addresses sequential (`0x1`, `0x2`, `0x3`, ...)
- Keep unit tests in the package `tests/` directory, not in `sources/`;
  `sources/` files ship into the release artifact

## Support

For issues or questions:

1. Check error messages carefully (build errors go to stderr, exit code is nonzero on failure)
2. Verify `Move.toml` syntax
3. Ensure all dependencies exist
4. Review this quickstart guide
5. Check the main README.md for details
