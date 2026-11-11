# kanari-move-fmt

Native Rust formatter for the Move language, exposed as `kanari move fmt`.

Built on the vendored tree-sitter Move grammar
(`third_party/tree-sitter-move`). This crate is the source of truth for the
Move formatting rules; the TypeScript `prettier-plugin-move` under
`third_party` is no longer the reference implementation.

## Blank-line policy

Blank lines are preserved exactly as authored. The formatter reproduces the
author's line breaks verbatim between items — module members, statements,
struct fields, and the lines right after `{` / right before `}` — and never
adds, removes, collapses or extends a blank line, so runs of blank lines
survive unchanged. The only exception: trailing whitespace is stripped from
every line, so a whitespace-only line becomes an empty line.

## Usage

```powershell
# Format the current package (sources/ + tests/)
kanari move fmt

# Check only (for CI; exits non-zero when files are unformatted)
kanari move fmt --check

# Format specific files or directories
kanari move fmt example_move/dex_v1/sources/dex_v1.move
```

Formatting rules live in `src/printer.rs` and are verified by:

- `cargo test -p kanari-move-fmt` — golden fixtures in `tests/fixtures/`
  plus vendored grammar samples (parse + idempotency).
- After an intentional rule change, regenerate the goldens with
  `KANARI_FMT_UPDATE=1 cargo test -p kanari-move-fmt` and review the diff.

## Layout

- `src/lib.rs` — `format_source()` entry point and tests.
- `src/printer.rs` — tree-sitter tree to pretty-document printer.
