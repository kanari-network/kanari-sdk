// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

import { Configuration } from './configuration';
import { Context } from './context';
import { Extension } from './extension';
import { MoveFormatter } from './formatter';
import { StatusBarController } from './status_bar';
import { log } from './log';

import * as childProcess from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';
import * as commands from './commands';

/**
 * An extension command that displays the version of the server that this extension
 * interfaces with.
 */
async function serverVersion(context: Readonly<Context>): Promise<void> {
  const version = childProcess.spawnSync(
    context.configuration.serverPath,
    ['--version'],
    {
      encoding: 'utf8',
    },
  );
  if (version.stdout) {
    await vscode.window.showInformationMessage(version.stdout);
  } else if (version.error) {
    await vscode.window.showErrorMessage(
      `Could not execute move-analyzer: ${version.error.message}.`,
    );
  } else {
    await vscode.window.showErrorMessage(
      `A problem occurred when executing '${context.configuration.serverPath}'.`,
    );
  }
}

async function findPkgRoot(): Promise<string | undefined> {
  const activeEditor = vscode.window.activeTextEditor;
  if (!activeEditor) {
    await vscode.window.showErrorMessage(
      'Cannot find package manifest (no active editor window)',
    );
    return undefined;
  }

  const containsManifest = (dir: string): boolean => {
    const filesInDir = fs.readdirSync(dir);
    return filesInDir.includes('Move.toml');
  };

  const activeFileDir = path.dirname(activeEditor.document.uri.fsPath);
  let currentDir = activeFileDir;
  while (currentDir !== path.parse(currentDir).root) {
    if (containsManifest(currentDir)) {
      return currentDir;
    }
    currentDir = path.resolve(currentDir, '..');
  }

  if (containsManifest(currentDir)) {
    return currentDir;
  }

  await vscode.window.showErrorMessage(
    `Cannot find package manifest for file in '${activeFileDir}' directory`,
  );
  return undefined;
}

async function kanariMoveCmd(
  context: Readonly<Context>,
  cmd: string,
): Promise<void> {
  const version = childProcess.spawnSync(
    context.configuration.kanariPath,
    ['--version'],
    {
      encoding: 'utf8',
    },
  );
  if (version.stdout) {
    const pkgRoot = await findPkgRoot();
    if (pkgRoot !== undefined) {
      const terminalName = 'kanari move';
      let terminal = vscode.window.terminals.find(
        (t) => t.name === terminalName,
      );
      if (!terminal) {
        terminal = vscode.window.createTerminal(terminalName);
      }
      terminal.show(true);
      terminal.sendText('cd ' + pkgRoot, true);
      terminal.sendText(`kanari move ${cmd}`, true);
    }
  } else {
    await vscode.window.showErrorMessage(
      `A problem occurred when executing the Kanari command: '${context.configuration.kanariPath}'`,
    );
  }
}

/**
 * An extension command that that builds the current Move project.
 */
async function buildProject(context: Readonly<Context>): Promise<void> {
  return kanariMoveCmd(context, 'build');
}

/**
 * An extension command that that builds the current Move project.
 */
async function testProject(context: Readonly<Context>): Promise<void> {
  return kanariMoveCmd(context, 'test');
}

/**
 * An extension command that restarts the Move language server (fresh package re-scan).
 */
async function restartServer(context: Readonly<Context>): Promise<void> {
  await context.restartClient();
  void vscode.window.showInformationMessage('Move language server restarted.');
}

/**
 * An extension command that stops the Move language server.
 */
async function stopServer(context: Readonly<Context>): Promise<void> {
  await context.stopClient();
  void vscode.window.showInformationMessage('Move language server stopped.');
}

/**
 * An extension command that reveals the extension output channel.
 */
async function openLogs(): Promise<void> {
  log.show();
}

/**
 * The entry point to this VS Code extension.
 *
 * As per [the VS Code documentation on activation
 * events](https://code.visualstudio.com/api/references/activation-events), "an extension must
 * export an `activate()` function from its main module and it will be invoked only once by
 * VS Code when any of the specified activation events [are] emitted."
 *
 * Activation events for this extension are listed in its `package.json` file, under the key
 * `"activationEvents"`.
 *
 * In order to achieve synchronous activation, mark the function as an asynchronous function,
 * so that you can wait for the activation to complete by await
 */
export async function activate(
  extensionContext: Readonly<vscode.ExtensionContext>,
): Promise<void> {
  const extension = new Extension();
  log.info(`${extension.identifier} version ${extension.version}`);

  const configuration = new Configuration();
  log.info(`configuration: ${configuration.toString()}`);

  // Register the Move document formatter (prettier-plugin-move, bundled in vendor/) first,
  // so that formatting works even if the language server fails to start.
  try {
    const formatter = new MoveFormatter(extensionContext);
    extensionContext.subscriptions.push(
      vscode.languages.registerDocumentFormattingEditProvider(
        'move',
        formatter,
      ),
    );
    log.info('Move document formatter registered');
  } catch (err) {
    log.info(`Could not register Move formatter: ${String(err)}`);
    void vscode.window.showErrorMessage(
      `Move on Kanari: formatter failed to load: ${err instanceof Error ? err.message : String(err)}`,
    );
  }

  // Status bar pill FIRST (before anything that can fail): if activation runs
  // at all, the pill is visible; any later failure flips it to the error state.
  // Its tooltip is a trusted Markdown menu (Open Logs / Stop server /
  // Restart server) anchored to the pill itself, like Move on Aptos.
  let statusBar: StatusBarController | undefined;
  try {
    statusBar = new StatusBarController(extension.version);
    extensionContext.subscriptions.push(statusBar);
  } catch (err) {
    log.info(`Could not create Move status bar: ${String(err)}`);
  }

  try {
    await activateLanguageServer(
      extensionContext,
      extension,
      configuration,
      statusBar,
    );
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    log.info(`Move on Kanari activation failed: ${message}`);
    statusBar?.setState('error', message);
    void vscode.window.showErrorMessage(
      `Move on Kanari activation failed: ${message}`,
    );
  }
}

/**
 * The remainder of activation (binary install, LSP client, watchers). Separated so
 * that any failure lands in the status bar + an error toast instead of silence.
 */
async function activateLanguageServer(
  extensionContext: Readonly<vscode.ExtensionContext>,
  extension: Extension,
  configuration: Configuration,
  statusBar: StatusBarController | undefined,
): Promise<void> {
  const globalMoveVersionKey = 'move-version';
  // Only (re)install the move-analyzer binary when the extension itself was
  // installed/upgraded, or when the binary is missing from its default location.

  const lastMoveVersion =
    extensionContext.globalState.get(globalMoveVersionKey);
  let doInstallBinary: boolean;
  let updateGlobalExtVersion: boolean;
  if (lastMoveVersion === null) {
    // Installation (no global variable set).
    doInstallBinary = true;
    updateGlobalExtVersion = true;
  } else if (lastMoveVersion === extension.version) {
    // Not an installation or an update (same version as seen before).
    const serverPathExists = await vscode.workspace.fs
      .stat(vscode.Uri.file(configuration.serverPath))
      .then(
        () => true,
        () => false,
      );
    doInstallBinary = !serverPathExists;
    updateGlobalExtVersion = false;
  } else {
    // Update (different versions).
    doInstallBinary = true;
    updateGlobalExtVersion = true;
  }

  if (doInstallBinary) {
    const success = await configuration.installServerBinary(extensionContext);
    if (!success) {
      statusBar?.setState('error', 'move-analyzer binary install failed');
      return;
    }
  }

  log.info('Creating extension context');
  const context = Context.create(extensionContext, configuration);
  // An error here -- for example, if the path to the `move-analyzer` binary that the user
  // specified in their settings is not valid -- prevents the extension from providing any
  // more utility, so return early.
  if (context instanceof Error) {
    statusBar?.setState('error', context.message);
    void vscode.window.showErrorMessage(
      `Could not activate Move: ${context.message}.`,
    );
    return;
  }
  context.statusBar = statusBar;

  // Register handlers for VS Code commands that the user explicitly issues.
  context.registerCommand('serverVersion', serverVersion);
  context.registerCommand('build', buildProject);
  context.registerCommand('test', testProject);
  context.registerCommand('restartServer', restartServer);
  context.registerCommand('stopServer', stopServer);
  extensionContext.subscriptions.push(
    vscode.commands.registerCommand('move-kanari.openLogs', openLogs),
  );

  // Configure other language features.
  context.configureLanguage();

  // Restart the language server automatically when the package layout changes
  // (new/deleted .move files, Move.toml edits) — like rust-analyzer's auto-reload.
  context.registerFileSystemWatchers();

  // All other utilities provided by this extension occur via the language server.
  // A failure here (e.g. the move-analyzer binary cannot analyze this package) should
  // not prevent the rest of the extension (formatter, commands) from working.
  try {
    await context.startClient();
    context.registerCommand(
      'textDocumentDocumentSymbol',
      commands.textDocumentDocumentSymbol,
    );
    context.registerCommand('textDocumentHover', commands.textDocumentHover);
    context.registerCommand(
      'textDocumentCompletion',
      commands.textDocumentCompletion,
    );
  } catch (err) {
    log.info(`Language server failed to start: ${String(err)}`);
    statusBar?.setState(
      'error',
      err instanceof Error ? err.message : String(err),
    );
    void vscode.window.showWarningMessage(
      'Move language server failed to start (formatting still works): ' +
        `${err instanceof Error ? err.message : String(err)}`,
    );
  }

  context.registerOnDidChangeConfiguration();

  if (updateGlobalExtVersion) {
    await extensionContext.globalState.update(
      globalMoveVersionKey,
      extension.version,
    );
  }
}
