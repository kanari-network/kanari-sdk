# Kanari Framework Builder

This crate provides a build helper for compiling the Move packages used by the Kanari frameworks. It is implemented as a small Rust crate with a `build.rs` script that programmatically invokes the `move-package` APIs to compile each Move package under the `kanari-frameworks/packages` directory.

**Purpose**: compile Move packages and place compiled outputs into each package's `build` directory so other crates can depend on the compiled artifacts.

## Usage

- **Build the builder (invokes the Move compilation step):**

 ```powershell
 cd C:\Users\Pukpuy\Desktop\kanari-cp\crates
 cargo build -p kanari-framework-builder
 ```

- **Build the entire workspace (runs the build script when appropriate):**

 ```powershell
 cd C:\Users\Pukpuy\Desktop\kanari-cp
 cargo build
 ```

### Output

- Each Move package in `crates\..\kanari-frameworks\packages\<pkg>` will have a `build` directory created (e.g., `...\packages\move-stdlib\build`) containing the compiled package artifacts.

### Prerequisites

- Rust toolchain (the repo uses edition specified in `Cargo.toml`).
- The repository's Move-related dependencies are expected to be available in `third_party/move` or as workspace dependencies. Ensure the workspace is checked out completely.

### Troubleshooting

- If Move package compilation fails with "Unresolved addresses found", open the package's `Move.toml` and set concrete addresses in the `[addresses]` section (for example `std = "0x1"`) or provide `dev-addresses` and build with the dev flag if appropriate.
- If Cargo reports "no targets specified in the manifest" for this crate, ensure `src/lib.rs` or `src/main.rs` exists. This crate provides a no-op `src/lib.rs` so the build script can run.

### Development notes

- The build script prints `cargo:warning` messages while compiling packages; warnings from Move or Rust toolchains may appear during the process.
- To add or modify the set of packages being built, edit the `packages` directory under `crates\..\kanari-frameworks`.
