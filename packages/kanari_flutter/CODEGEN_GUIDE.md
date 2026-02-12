# Flutter Rust Bridge Codegen Guide

This document explains how to use the `flutter_rust_bridge_codegen` tool to generate bindings between Rust and Flutter in the Kanari SDK project.

## Prerequisites

- **Flutter Rust Bridge (FRB) CLI**: Ensure you have the generator installed.
  ```bash
  cargo install flutter_rust_bridge_codegen
  ```

## Configuration File

The project uses a configuration file named `frb.yml` located in the `packages/kanari_flutter/rust/` directory. This file defines the input and output paths for the generator.

### Current Configuration (`rust/frb.yml`)

```yaml
rust_input: "crate::api"
dart_output: "../flutter/kanari_crypto/lib/src/frb_generated.dart"
rust_root: "."
dart_root: "../flutter/kanari_crypto"
```

- **rust_input**: The Rust module containing the functions to export (using FRB v2 syntax).
- **rust_root**: The root directory of the Rust crate.
- **dart_root**: The root directory of the Flutter package.
- **dart_output**: The destination for the generated Dart code.

## Generating Bindings

To generate or update the bindings, navigate to the Rust package directory and run the following command:

### Command

```bash
cd packages/kanari_flutter/rust
flutter_rust_bridge_codegen generate --config-file frb.yml
```

### What happens during generation?

1. **Rust Analysis**: The tool parses your Rust code in `src/api.rs` (referenced by `crate::api`).
2. **Binding Creation**: It generates `src/frb_generated.rs` in Rust and several files in Dart under `lib/src/frb_generated.dart/`.
3. **Module Injection**: It automatically adds `mod frb_generated;` to your `lib.rs` if it's missing.
4. **Formatting**: It runs `dart format` and `cargo fmt` on the generated files.

## Troubleshooting

### Common Errors

- **"Prefix not found"**: This usually happens if the command is run from the wrong directory. Always run it from the directory containing `frb.yml` or provide absolute paths.
- **"Please migrate configuration rust_input"**: FRB v2 requires `crate::api` instead of `src/api.rs`. Ensure your `frb.yml` follows the new syntax.
- **Path Canonicalization**: On Windows, ensure paths in `frb.yml` use forward slashes `/` or escaped backslashes `\\` to avoid issues with the tool's internal path handling.

## Integration in Flutter

After generating the code, the bindings are exposed through the `kanari_crypto` library. You can import them in your Flutter app:

```dart
import 'package:kanari_crypto/kanari_crypto.dart';

// Example usage
final mnemonic = await generateMnemonicApi(wordCount: BigInt.from(12));
```
