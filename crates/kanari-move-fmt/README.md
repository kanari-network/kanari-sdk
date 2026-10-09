# kanari-move-fmt

Native Rust formatter for the Move language, exposed as `kanari move fmt`.

Implements the same formatting rules as the TypeScript
`prettier-plugin-move` (used by the Move on Kanari VS Code extension) on top
of the vendored tree-sitter Move grammar (`third_party/tree-sitter-move`).

## Usage

```powershell
# Format the current package (sources/ + tests/)
kanari move fmt

# Check only (for CI; exits non-zero when files are unformatted)
kanari move fmt --check

# Format specific files or directories
kanari move fmt example_move/dex_v1/sources/dex_v1.move
```

Formatting rules live in `src/printer.rs` and must stay byte-compatible with
`third_party/move/crates/move-analyzer/prettier-plugin/src/printer.ts`.
Rule changes go here first then get back-ported (or vice versa), verified by:

- `cargo test -p kanari-move-fmt` — golden fixtures shared with the
  TypeScript suite plus vendored grammar samples (parse + idempotency).
- Manual differential run against the TypeScript output over
  `crates/kanari-frameworks/packages/*/sources` and `example_move/*/sources`
  (currently byte-identical on all 45 files).

## Layout

- `src/lib.rs` — `format_source()` entry point and tests.
- `src/printer.rs` — tree-sitter tree to pretty-document printer.
