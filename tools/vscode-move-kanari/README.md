# Move on Kanari

VS Code extension for Move development on the Kanari network: syntax highlighting
for `.move` files plus the stock **`move-analyzer`** language server
(go-to-definition, find references, hover, completion, diagnostics).

**0.9.0: single binary, cross-platform.** `crates/kanari-move-analyzer` is now
a thin Rust binary (`move-analyzer` pulled as a library via
`move-analyzer = { path = "third_party/move/crates/move-analyzer" }`, not
forked, not spawned) that repairs `~/.move` caches then runs the LSP. One
binary (`kanari-move-analyzer[.exe]`) ships in the vsix. The TS side is a
**single thin launcher** (`src/main.ts`, ~230 lines) that spawns it.

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
| `move.restartOnSave` | `false`                       | Restart the language server (debounced) on `.move` save. Default `false` since 0.6.0 — restarting on every save wiped unsaved diagnostics until rescan finished |
| `move.server.path`   | `~/.kanari/bin/kanari-move-analyzer` | Path to the language server binary (`.exe` on Windows)                              |
| `move.kanari.path`   | `kanari`                      | Path to the Kanari CLI (for build/test commands)                                                     |

## 0.6.0 notes — rewrite

- **Layout** (`src/` rewritten, behaviour preserved):
  - `main.ts` — thin re-export to `app/activate.ts` (stable entry)
  - `app/` — activation orchestration + all commands
  - `config/` — settings + `move-analyzer` binary install
  - `deps/` — `git` runner + `~/.move` cache repair (reports unrepairable caches)
  - `lsp/` — language client, auto-reload watchers, **dependency diagnostics**
  - `features/` — formatter, import quick fixes, test CodeLens, status pill
  - `shared/` — log, extension info, `Move.toml` lookup

- **Bug fixed: “still no errors” when dependencies fail to resolve ★**
  - Root cause: when `move-analyzer` cannot `git fetch`/`git reset` a
    `git = ...` dependency (e.g. stale `~/.move/.../.git/index.lock` left
    behind by a crashed run), it reports a single transient toast
    (`Failed to resolve dependencies…`) and then sends **zero**
    `publishDiagnostics` for the whole package.
  - Fix: `lsp/diagnostics.ts` surfaces unrepairable caches (from the
    pre-start cache repair in `deps/cacheRepair.ts`) as a real
    `Error` diagnostic (`E_DEPENDENCIES_UNRESOLVED`) on every open
    `.move` file, so the failure is visible in the editor **and** in
    the Problems panel — not just a one-off toast. The repair now
    removes `index.lock` whenever no `git` process is running
    (the old code waited 60 s for the lock to look “stale”, during
    which every resolve failed with exit 128).
  - `move-analyzer` is built from this repo’s vendored fork:
    `third_party/move/crates/move-analyzer`
    (`cargo build --release -p move-analyzer --manifest-path third_party/move/Cargo.toml`).

## Install (local build)

Prerequisites: Rust toolchain, Node.js >= 16. **The vsix bundles a
platform-specific binary**, so build and install **on the target OS**
(Windows → `package.ps1`, macOS/Linux → `package.sh`).

**Windows (PowerShell):**
```powershell
powershell -File tools\vscode-move-kanari\scripts\package.ps1
```

**macOS / Linux (bash):**
```bash
bash tools/vscode-move-kanari/scripts/package.sh
```

Or manually:

```bash
# 1. Build the language server (single binary: move-analyzer linked as lib)
cargo build --release -p kanari-move-analyzer --manifest-path Cargo.toml

# 2. Bundle it with the extension (`.exe` on Windows, no suffix on macOS/Linux)
mkdir -p tools/vscode-move-kanari/language-server
cp target/release/kanari-move-analyzer* tools/vscode-move-kanari/language-server/

# 3. Build and package the extension
cd tools/vscode-move-kanari
npm install
npm run package
code --install-extension move-kanari.vsix
```

On activation the extension copies the bundled `kanari-move-analyzer[.exe]` binary to
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
