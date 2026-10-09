// Copyright (c) The Kanari Network Contributors
// SPDX-License-Identifier: Apache-2.0

import * as vscode from 'vscode';

/** Language server lifecycle state shown in the status bar. */
export type ServerState = 'stopped' | 'starting' | 'running' | 'error';

/** Minimal status interface for the language server status pill. */
export interface ServerStatus {
  setState(state: ServerState, errorMessage?: string): void;
}

/**
 * Status bar pill for the Move language server, modeled after the Move on
 * Aptos extension: a left-aligned item whose *tooltip* is a trusted Markdown
 * document with clickable command links, so the Open Logs / Stop server /
 * Restart server menu appears anchored to the pill itself (not as a top
 * QuickPick).
 */
export class StatusBarController implements vscode.Disposable, ServerStatus {
  private readonly item: vscode.StatusBarItem;

  constructor(private readonly extensionVersion: string) {
    this.item = vscode.window.createStatusBarItem(
      'move-kanari.status',
      vscode.StatusBarAlignment.Left,
      100,
    );
    this.item.name = 'Move on Kanari';
    this.item.command = 'move-kanari.openLogs';
    this.item.show();
    this.setState('stopped');
  }

  /** Updates the pill to reflect the given server state. */
  setState(state: ServerState, errorMessage?: string): void {
    const tooltip = new vscode.MarkdownString('', true);
    tooltip.isTrusted = true;
    tooltip.supportHtml = false;
    if (errorMessage !== undefined) {
      tooltip.appendMarkdown(`${errorMessage}\n\n`);
    }
    if (tooltip.value) {
      tooltip.appendMarkdown('\n\n---\n\n');
    }
    tooltip.appendMarkdown(
      `[Extension Info](command:move-kanari.serverVersion "Show version and server binary info"): Version ${this.extensionVersion}\n\n` +
        `---\n\n` +
        `[$(terminal) Open Logs](command:move-kanari.openLogs "Open the server logs")\n\n` +
        `[$(stop-circle) Stop server](command:move-kanari.stopServer "Stop the server")\n\n` +
        `[$(debug-restart) Restart server](command:move-kanari.restartServer "Restart the server")`,
    );
    this.item.tooltip = tooltip;

    switch (state) {
      case 'running':
        this.item.text = '$(check) Move on Kanari';
        this.item.color = undefined;
        this.item.backgroundColor = undefined;
        break;
      case 'starting':
        this.item.text = '$(loading~spin) Move on Kanari';
        this.item.color = undefined;
        this.item.backgroundColor = undefined;
        break;
      case 'stopped':
        this.item.text = '$(stop-circle) Move on Kanari';
        this.item.color = new vscode.ThemeColor('statusBarItem.warningForeground');
        this.item.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
        break;
      case 'error':
        this.item.text = '$(error) Move on Kanari';
        this.item.color = new vscode.ThemeColor('statusBarItem.errorForeground');
        this.item.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
        break;
      default:
        break;
    }
  }

  dispose(): void {
    this.item.dispose();
  }
}
