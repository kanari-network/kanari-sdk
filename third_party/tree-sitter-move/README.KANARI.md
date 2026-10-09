# tree-sitter-move (vendored)

Vendored copy of the tree-sitter Move grammar with Rust bindings, used by
`crates/kanari-move-fmt` (and therefore `kanari move fmt`).

- Upstream: <https://github.com/tzakian/tree-sitter-move> (MIT, see `LICENSE`)
- Contents: `src/parser.c`, `src/tree_sitter/*` (runtime headers),
  `src/node-types.json`, `grammar.js`, `bindings/rust/*`.
- Local changes vs upstream: `bindings/rust/lib.rs` uses `unsafe extern`
  (required by the current Rust toolchain); `Cargo.toml` is adapted to this
  workspace (the `tree-sitter` dependency itself is unchanged: `~0.20.10`).

Do NOT upgrade the grammar casually: the printer in `kanari-move-fmt` (and
the TypeScript `prettier-plugin-move`) depends on exact node type names.
After any grammar update, re-run the formatter golden tests plus a full
differential run over the framework packages before committing.

Test corpus for the grammar lives in `tests/*.move` (parsed + formatted by
`kanari-move-fmt` tests for parse errors and idempotency).
