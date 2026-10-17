# Move on Kanari

VS Code extension for Move development on the Kanari network: syntax highlighting
for `.move` files plus the `move-analyzer` language server (go-to-definition, find
references, hover, completion, diagnostics).

Forked from the Move on Aptos/Sui extension (`third_party/move/crates/move-analyzer/editors/code`)
and rebranded for Kanari.

## Features

- Syntax highlighting for `.move` files (grammar vendored from
  [damirka/move-syntax](https://github.com/damirka/vscode-move-syntax), MIT)
- Move code blocks inside Markdown get highlighted too
- Move logo as the language icon on `.move` files (works with your current file icon
  theme — no theme switch needed)
- Language server via `move-analyzer` built from this repository:
  diagnostics, go-to-definition, find references, hover, completion, document symbols
- Import quick fixes (client-side, work with any language server): `Add import for
`addr::module`` on unresolved names, resolved from your workspace (`Move.toml` +
  sources on disk, own package first, then local dependencies, then well-known stdlib
  modules). 2024-aware: it stays quiet for `vector`/`option`/`Option` where the edition
  prelude already provides them, and offers `Remove redundant import` on duplicate-alias
  warnings instead
- Test CodeLens: `▶ Run Test` above every `#[test]` function and `▶ Run Tests` at the top
  of the file, running `kanari move test [filter]` in a terminal rooted at the package
- Move formatter (same rules as `kanari move fmt`): format on save for `.move`
- Status bar pill + language status with Open Logs / Stop server / Restart server
  (click the pill for the anchored menu, like Move on Aptos)
- Auto-reload: the server restarts when `.move` files are added/deleted or
  `Move.toml` changes (like rust-analyzer)
- Commands: `Move on Kanari: Show Server Version`, `Build a Move package`,
  `Test a Move package`, `Restart Language Server`, `Stop Language Server`,
  `Open Logs` (build/test run `kanari move build|test` in a terminal)

## Settings

| Setting              | Default                       | Description                                                                                          |
| -------------------- | ----------------------------- | ---------------------------------------------------------------------------------------------------- |
| `move.lint`          | `default`                     | Lint level: `default`, `all`, `none`                                                                 |
| `move.restartOnSave` | `true`                        | Restart the language server (debounced) on `.move` save so stale errors clear without manual restart |
| `move.server.path`   | `~/.kanari/bin/move-analyzer` | Path to the language server binary                                                                   |
| `move.kanari.path`   | `kanari`                      | Path to the Kanari CLI (for build/test commands)                                                     |

## Install (local build)

Prerequisites: Rust toolchain, Node.js >= 16.

```powershell
# From the repository root — builds move-analyzer, bundles it, packages and installs the extension
powershell -File tools\vscode-move-kanari\scripts\package.ps1
```

Or manually:

```powershell
# 1. Build the language server
cargo build --release -p move-analyzer --manifest-path third_party\move\Cargo.toml

# 2. Bundle it with the extension
Copy-Item third_party\move\target\release\move-analyzer.exe tools\vscode-move-kanari\language-server\

# 3. Build and package the extension
cd tools\vscode-move-kanari
npm install
npm run package
code --install-extension move-kanari.vsix
```

On activation the extension copies the bundled `move-analyzer` binary to
`~/.kanari/bin/` (override with `move.server.path`).

## Conflicting Move extensions

Other Move extensions (e.g. Move on Aptos/Sui) register for the same `.move` files. If
more than one is installed, VS Code may send your files to the wrong language server,
which then reports bogus errors such as `Unresolved reference 'vector'` on code that
compiles fine. For Kanari workspaces, disable the other Move extensions (or at least
their language server) and keep only **Move on Kanari** enabled, then reload the window
and run `Move on Kanari: Restart Language Server`.

## License

- Extension code: Apache-2.0 (see `LICENSE`)
- Syntax grammar: MIT (Damir Shamanaev, `damirka/move-syntax`)
- Language server: Apache-2.0 (`third_party/move/crates/move-analyzer`)
