// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * "Run Test" CodeLens above `#[test]` functions (and "Run Tests" above modules).
 *
 * The bundled language server does not provide test lenses, so the client adds them by
 * scanning the document text. Clicking runs `kanari move test [filter]` in a terminal rooted
 * at the package holding the file.
 */

import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';

/** Walk up from a file looking for Move.toml. */
export function findPackageRootFor(
  filePath: string,
  existsSync: (p: string) => boolean = fs.existsSync,
): string | undefined {
  let dir = path.dirname(filePath);
  for (;;) {
    if (existsSync(path.join(dir, 'Move.toml'))) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      return undefined;
    }
    dir = parent;
  }
}

/** Line numbers (0-based) of `#[test]` functions and the test name on each. */
export function findTestFns(
  lines: ReadonlyArray<string>,
): Array<{ line: number; name: string }> {
  const out: Array<{ line: number; name: string }> = [];
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i] ?? '';
    // `#[test]`, optionally with args like `#[test, expected_failure]`, followed by `fun name`.
    // Attributes may sit on their own line or share it with `fun`.
    const attrOnLine = /#\s*\[\s*test\b/.test(line);
    const sameLineFun = /fun\s+([A-Za-z_][A-Za-z0-9_]*)/.exec(line);
    if (attrOnLine && sameLineFun?.[1] !== undefined) {
      out.push({ line: i, name: sameLineFun[1] });
      continue;
    }
    if (/^\s*#\s*\[\s*test\b/.test(line)) {
      for (let j = i + 1; j < Math.min(i + 4, lines.length); j++) {
        const funMatch = /fun\s+([A-Za-z_][A-Za-z0-9_]*)/.exec(
          lines[j] ?? '',
        );
        if (funMatch?.[1] !== undefined) {
          out.push({ line: j, name: funMatch[1] });
          break;
        }
        // Stop at anything that is not an attribute or blank line.
        if (!/^\s*(#[^\n]*|\/\/[^\n]*)?$/.test(lines[j] ?? '')) {
          break;
        }
      }
    }
  }
  return out;
}

export class MoveTestCodeLensProvider implements vscode.CodeLensProvider {
  public onDidChangeCodeLenses?: vscode.Event<void>;

  public provideCodeLenses(
    document: vscode.TextDocument,
  ): Array<vscode.CodeLens> {
    const lines: string[] = [];
    for (let i = 0; i < document.lineCount; i++) {
      lines.push(document.lineAt(i).text);
    }
    const lenses: vscode.CodeLens[] = [];
    // Whole-file lens first.
    lenses.push(
      new vscode.CodeLens(new vscode.Range(0, 0, 0, 0), {
        title: '▶ Run Tests',
        command: 'move-kanari.runTestFile',
        arguments: [document.uri],
      }),
    );
    for (const t of findTestFns(lines)) {
      lenses.push(
        new vscode.CodeLens(new vscode.Range(t.line, 0, t.line, 0), {
          title: '▶ Run Test',
          command: 'move-kanari.runTest',
          arguments: [document.uri, t.name],
        }),
      );
    }
    return lenses;
  }
}

/** Run `kanari move test [filter]` in a terminal rooted at the file's package. */
export async function runMoveTest(
  kanariPath: string,
  fileUri: vscode.Uri,
  filter?: string,
): Promise<void> {
  const pkgRoot = findPackageRootFor(fileUri.fsPath);
  if (pkgRoot === undefined) {
    void vscode.window.showErrorMessage(
      'Cannot run Move test: no Move.toml found above this file.',
    );
    return;
  }
  const terminalName = 'kanari move';
  let terminal = vscode.window.terminals.find((t) => t.name === terminalName);
  if (terminal === undefined) {
    terminal = vscode.window.createTerminal(terminalName);
  }
  terminal.show(true);
  terminal.sendText(`cd ${pkgRoot}`, true);
  terminal.sendText(
    filter === undefined
      ? `${kanariPath} move test`
      : `${kanariPath} move test ${filter}`,
    true,
  );
}
