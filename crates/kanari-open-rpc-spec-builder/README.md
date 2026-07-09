# Kanari OpenRPC Spec Builder

Small CLI for generating and validating the Kanari OpenRPC schema.

## Usage

```bash
cargo run -p kanari-open-rpc-spec-builder -- print
cargo run -p kanari-open-rpc-spec-builder -- record
cargo run -p kanari-open-rpc-spec-builder -- test
```

## Commands

- `print` prints the generated OpenRPC JSON to stdout.
- `record` writes the generated schema to the recorded spec file.
- `test` checks that the generated schema is valid and matches the recorded file.

## Notes

- The spec content is built from `kanari-rpc-api::methods`.
- Use `record` after RPC API changes.
