// Copyright (c) The Kanari Network Contributors
// SPDX-License-Identifier: Apache-2.0

import * as path from 'path';
import * as prettier from 'prettier';
import * as vscode from 'vscode';

/**
 * Document formatting provider for Move files, backed by the vendored
 * `prettier-plugin-move` (tree-sitter based) from
 * `third_party/move/crates/move-analyzer/prettier-plugin`.
 */
export class MoveFormatter implements vscode.DocumentFormattingEditProvider {
  private readonly plugin: prettier.Plugin;

  constructor(extensionContext: Readonly<vscode.ExtensionContext>) {
    const pluginPath = extensionContext.asAbsolutePath(
      path.join('vendor', 'prettier-plugin-move'),
    );
    // Loaded from the bundled vendor directory (CommonJS require of the plugin's
    // package.json "main").
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    this.plugin = require(pluginPath);
  }

  async provideDocumentFormattingEdits(
    document: Readonly<vscode.TextDocument>,
  ): Promise<vscode.TextEdit[]> {
    const text = document.getText();
    let formatted: string;
    try {
      formatted = await prettier.format(text, {
        parser: 'move-parse',
        plugins: [this.plugin],
        tabWidth: 4,
      });
    } catch (err) {
      void vscode.window.showErrorMessage(
        `Move formatting failed: ${err instanceof Error ? err.message : String(err)}`,
      );
      return [];
    }
    if (formatted === text) {
      return [];
    }
    const lastLine = document.lineAt(document.lineCount - 1);
    const range = new vscode.Range(
      0,
      0,
      lastLine.lineNumber,
      lastLine.range.end.character,
    );
    return [vscode.TextEdit.replace(range, formatted)];
  }
}
