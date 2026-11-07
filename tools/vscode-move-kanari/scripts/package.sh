#!/usr/bin/env bash
# Builds kanari-move-analyzer (single binary, move-analyzer linked as lib),
# bundles it into the extension, packages a .vsix and installs it.
# Usage (repo root, macOS / Linux):  bash tools/vscode-move-kanari/scripts/package.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
EXT_DIR="$REPO_ROOT/tools/vscode-move-kanari"

echo "==> Building kanari-move-analyzer (release)..."
cargo build --release -p kanari-move-analyzer --manifest-path "$REPO_ROOT/Cargo.toml"

SERVER_DIR="$EXT_DIR/language-server"
mkdir -p "$SERVER_DIR"
cp "$REPO_ROOT/target/release/kanari-move-analyzer" "$SERVER_DIR/kanari-move-analyzer"
chmod +x "$SERVER_DIR/kanari-move-analyzer"
echo "==> Bundled kanari-move-analyzer"

# prettier plugin (vendored, same as Windows)
PLUGIN_DIR="$REPO_ROOT/third_party/move/crates/move-analyzer/prettier-plugin"
if [ -d "$PLUGIN_DIR" ]; then
  echo "==> Building prettier-plugin-move..."
  (cd "$PLUGIN_DIR" && npm install --no-audit --no-fund && npm run compile)
  VENDOR_DIR="$EXT_DIR/vendor/prettier-plugin-move"
  rm -rf "$VENDOR_DIR"
  mkdir -p "$VENDOR_DIR"
  cp "$PLUGIN_DIR/package.json" "$VENDOR_DIR/"
  cp "$PLUGIN_DIR/tree-sitter-move.wasm" "$VENDOR_DIR/" 2>/dev/null || true
  cp -R "$PLUGIN_DIR/out" "$VENDOR_DIR/"
  echo "==> Vendored prettier-plugin-move"
fi

echo "==> npm install + compile + package..."
(cd "$EXT_DIR" && npm install && npm run package)

echo "==> Installing move-kanari.vsix..."
code --install-extension "$EXT_DIR/move-kanari.vsix" || echo "code CLI not found — install manually: code --install-extension $EXT_DIR/move-kanari.vsix"
echo "==> Done. Reload VS Code window to activate."
