// Thin launcher for kanari-move-analyzer (Rust wrapper around move-analyzer).
// The Rust server owns symbolication, code actions, formatting, code lens,
// dependency repair and diagnostics. This file only spawns it, installs its
// binary, and wires VS Code commands/status.

import * as childProcess from 'child_process';
import { sync as commandExistsSync } from 'command-exists';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import * as vscode from 'vscode';
import * as lc from 'vscode-languageclient/node';

const VERSION_KEY = 'move-version';
const binaryName = process.platform === 'win32' ? 'kanari-move-analyzer.exe' : 'kanari-move-analyzer';
const defaultServerDir = vscode.Uri.joinPath(vscode.Uri.file(os.homedir()), '.kanari', 'bin');
const output = vscode.window.createOutputChannel('Move on Kanari');
const trace = vscode.window.createOutputChannel('Move Language Server Trace', { log: true });

function log(msg: string): void {
  output.appendLine(`INFO [${new Date().toLocaleString()}]: ${msg}`);
}

function resolveUserPath(v: string): string {
  return v.startsWith('~/') ? os.homedir() + v.slice(1) : path.resolve(v);
}
function findPkgRoot(file: string): string | undefined {
  if (typeof file !== 'string' || file.length === 0) return undefined;
  let dir = path.dirname(file);
  for (;;) {
    if (fs.existsSync(path.join(dir, 'Move.toml'))) return dir;
    const p = path.dirname(dir);
    if (p === dir) return undefined;
    dir = p;
  }
}

let client: lc.LanguageClient | undefined;
let queue: Promise<void> = Promise.resolve();
let statusItem: vscode.StatusBarItem | undefined;
let serverPath = '';
let kanariPath = 'kanari';
let lintLevel = 'default';

function setStatus(state: 'stopped' | 'starting' | 'running' | 'error', msg?: string): void {
  if (statusItem === undefined) return;
  const tip = new vscode.MarkdownString('', true);
  tip.isTrusted = true;
  if (msg !== undefined) tip.appendMarkdown(`${msg}\n\n---\n\n`);
  tip.appendMarkdown('[Open Logs](command:move-kanari.openLogs)\n\n[Stop](command:move-kanari.stopServer)\n\n[Restart](command:move-kanari.restartServer)');
  statusItem.tooltip = tip;
  if (state === 'running') { statusItem.text = '$(check) Move on Kanari'; statusItem.color = undefined; statusItem.backgroundColor = undefined; }
  else if (state === 'starting') { statusItem.text = '$(loading~spin) Move on Kanari'; }
  else if (state === 'stopped') { statusItem.text = '$(stop-circle) Move on Kanari'; statusItem.color = new vscode.ThemeColor('statusBarItem.warningForeground'); statusItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground'); }
  else { statusItem.text = '$(error) Move on Kanari'; statusItem.color = new vscode.ThemeColor('statusBarItem.errorForeground'); statusItem.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground'); }
}

function enqueue<T>(op: () => Promise<T>): Promise<T> {
  const run = queue.then(op, op);
  queue = run.then(() => undefined, () => undefined);
  return run;
}
async function startInternal(): Promise<void> {
  if (client !== undefined) return;
  setStatus('starting');
  if (!commandExistsSync(serverPath)) throw new Error(`language server executable '${serverPath}' could not be found`);
  trace.clear();
  const exe: lc.Executable = { command: serverPath };
  const c = new lc.LanguageClient('move', 'Move', { run: exe, debug: exe }, {
    documentSelector: [{ scheme: 'file', language: 'move' }],
    traceOutputChannel: trace,
    initializationOptions: { lintLevel },
  });
  log('Starting move-analyzer...');
  await c.start();
  client = c;
  setStatus('running');
  log('move-analyzer running');
}
async function stopInternal(): Promise<void> {
  log('Stopping move-analyzer...');
  if (client !== undefined) { const c = client; client = undefined; await c.stop(); }
  setStatus('stopped');
}
// Two windows / a Reload can leave a stale `kanari-move-analyzer` behind;
// the leftover keeps pushing (0) diagnostics and hides real errors.
// Kill any stale instances before starting (singleton per machine).
function killStaleServers(): void {
  try {
    if (process.platform === 'win32') {
      childProcess.spawnSync('taskkill', ['/F', '/IM', binaryName], { stdio: 'ignore' });
    } else {
      childProcess.spawnSync('pkill', ['-f', binaryName], { stdio: 'ignore' });
    }
  } catch { /* ignore */ }
}
const start = (): Promise<void> => enqueue(startInternal);
const stop = (): Promise<void> => enqueue(stopInternal);
const restart = (): Promise<void> => enqueue(async () => { await stopInternal(); await startInternal(); });

async function uriExists(uri: vscode.Uri): Promise<boolean> {
  try { await vscode.workspace.fs.stat(uri); return true; } catch { return false; }
}
async function ensureBinary(ctx: vscode.ExtensionContext): Promise<boolean> {
  const bundled = vscode.Uri.joinPath(ctx.extensionUri, 'language-server', binaryName);
  const bundledExists = await uriExists(bundled);
  const target = vscode.Uri.file(serverPath);
  const targetExists = await uriExists(target);
  const defaultPath = vscode.Uri.joinPath(defaultServerDir, binaryName);
  if (serverPath !== defaultPath.fsPath) {
    if (!targetExists) { void vscode.window.showErrorMessage(`move-analyzer not found at '${serverPath}'.`); return false; }
    return true;
  }
  if (!bundledExists) {
    void vscode.window.showErrorMessage('Pre-built kanari-move-analyzer not found. Run tools/vscode-move-kanari/scripts/package.ps1, or set move.server.path.');
    return false;
  }
  if (!(await uriExists(defaultServerDir))) await vscode.workspace.fs.createDirectory(defaultServerDir);
  try {
    // Copy overwrites the target — no delete beforehand (on Windows the
    // running binary would be locked and the delete would fail, leaving
    // no binary at all).
    await vscode.workspace.fs.copy(bundled, target);
  } catch (e) {
    log(`binary install copy failed: ${String(e)}`);
    void vscode.window.showErrorMessage(`Failed to install kanari-move-analyzer: ${String(e)}`);
    return false;
  }
  log(`Installed kanari-move-analyzer to ${serverPath}`);
  return true;
}

async function kanariMove(subcommand: string): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (editor === undefined) { void vscode.window.showErrorMessage('Cannot find package manifest (no active editor)'); return; }
  const pkgRoot = findPkgRoot(editor.document.uri.fsPath);
  if (pkgRoot === undefined) { void vscode.window.showErrorMessage(`Cannot find package manifest for file in '${path.dirname(editor.document.uri.fsPath)}'`); return; }
  const name = 'kanari move';
  let t = vscode.window.terminals.find((x) => x.name === name);
  t ??= vscode.window.createTerminal({ name, cwd: pkgRoot });
  t.show(true);
  t.sendText(`${kanariPath} move ${subcommand}`, true);
}

// CodeLens args arrive as JSON strings ("file:///…" from the LSP server),
// not vscode.Uri objects. Accept both so `runTest`/`runTestFile` never
// crash with `The "path" argument must be of type string. Received undefined`.
function toFsPath(uriLike: unknown): string | undefined {
  if (uriLike === undefined || uriLike === null) return undefined;
  if (typeof uriLike === 'string') {
    try { return vscode.Uri.parse(uriLike).fsPath; } catch { return uriLike; }
  }
  const u = uriLike as { fsPath?: unknown; path?: unknown };
  if (typeof u.fsPath === 'string') return u.fsPath;
  if (typeof u.path === 'string') return u.path;
  return undefined;
}

async function runMoveTest(uriLike: unknown, filter?: string): Promise<void> {
  const fsPath = toFsPath(uriLike);
  if (fsPath === undefined) { void vscode.window.showErrorMessage('Cannot run Move test: invalid file URI'); return; }
  const pkgRoot = findPkgRoot(fsPath);
  if (pkgRoot === undefined) { void vscode.window.showErrorMessage('Cannot run Move test: no Move.toml found above this file.'); return; }
  const name = 'kanari move';
  let t = vscode.window.terminals.find((x) => x.name === name);
  t ??= vscode.window.createTerminal({ name, cwd: pkgRoot });
  t.show(true);
  t.sendText(filter === undefined ? `${kanariPath} move test` : `${kanariPath} move test ${filter}`, true);
}

export async function activate(ctx: vscode.ExtensionContext): Promise<void> {
  const ext = vscode.extensions.getExtension('kanari-network.move-kanari');
  const version = typeof ext?.packageJSON?.['version'] === 'string' ? String(ext.packageJSON['version']) : 'unknown';
  log(`Move on Kanari ${version} activating`);

  const ws = vscode.workspace.getConfiguration('move');
  serverPath = resolveUserPath(ws.get<string | null>('server.path') ?? vscode.Uri.joinPath(defaultServerDir, binaryName).fsPath);
  const kp = ws.get<string | null>('kanari.path') ?? 'kanari';
  kanariPath = kp.includes(path.sep) || kp.startsWith('~') ? resolveUserPath(kp) : kp;
  lintLevel = ws.get<string>('lint') ?? 'default';
  log(`server=${serverPath} kanari=${kanariPath} lint=${lintLevel}`);

  statusItem = vscode.window.createStatusBarItem('move-kanari.status', vscode.StatusBarAlignment.Left, 100);
  statusItem.name = 'Move on Kanari';
  statusItem.command = 'move-kanari.openLogs';
  statusItem.show();
  setStatus('stopped');
  ctx.subscriptions.push(statusItem);

  ctx.subscriptions.push(vscode.languages.setLanguageConfiguration('move', {
    onEnterRules: [
      { beforeText: /^\s*\/{3}.*$/, action: { indentAction: vscode.IndentAction.None, appendText: '/// ' } },
      { beforeText: /^\s*\/{2}!.*$/, action: { indentAction: vscode.IndentAction.None, appendText: '//! ' } },
    ],
  }));

  // Diagnostics dump (file/range/code/message) â€” keeps red squiggles debuggable.
  ctx.subscriptions.push(vscode.languages.onDidChangeDiagnostics((event) => {
    for (const uri of event.uris) {
      const diags = vscode.languages.getDiagnostics(uri);
      const lines = diags.map((d) => `  ${d.range.start.line + 1}:${d.range.start.character} [${d.severity === vscode.DiagnosticSeverity.Error ? 'E' : 'W'}] ${String(d.code ?? '')} ${d.source ?? ''}: ${d.message.replace(/\n/g, ' ')}`);
      log(`Diagnostics in ${uri.fsPath} (${diags.length}):\n${lines.join('\n') || '  (none)'}`);
    }
  }));

  const reg = (id: string, cb: (...a: Array<any>) => Promise<any>): void => {
    ctx.subscriptions.push(vscode.commands.registerCommand(id, (...a: Array<any>) => cb(...a)));
  };
  reg('move-kanari.serverVersion', async () => {
    const r = childProcess.spawnSync(serverPath, ['--version'], { encoding: 'utf8' });
    if (r.stdout) void vscode.window.showInformationMessage(r.stdout);
    else if (r.error) void vscode.window.showErrorMessage(`Could not execute move-analyzer: ${r.error.message}.`);
    else void vscode.window.showErrorMessage(`A problem occurred when executing '${serverPath}'.`);
  });
  reg('move-kanari.build', async () => kanariMove('build'));
  reg('move-kanari.test', async () => kanariMove('test'));
  reg('move-kanari.restartServer', async () => { await restart(); void vscode.window.showInformationMessage('Move language server restarted.'); });
  reg('move-kanari.stopServer', async () => { await stop(); void vscode.window.showInformationMessage('Move language server stopped.'); });
  reg('move-kanari.openLogs', async () => output.show());
  reg('move-kanari.runTest', async (uri: unknown, filter?: string) => runMoveTest(uri, filter));
  reg('move-kanari.runTestFile', async (uri: unknown) => runMoveTest(uri));

  // Restart on new/deleted .move or Move.toml (ignoring build output),
  // plus on save (so stale diagnostics clear without manual restart).
  let timer: ReturnType<typeof setTimeout> | undefined;
  const schedule = (reason: string): void => {
    if (timer !== undefined) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = undefined;
      log(`Restarting language server (${reason})`);
      restart().then(() => log(`Language server restarted (${reason})`), (e) => { log(`Language server restart failed: ${String(e)}`); setStatus('error', String(e)); });
    }, 750);
  };
  const ignore = (u: vscode.Uri): boolean => u.fsPath.split(/[\\/]/).some((p) => ['build', 'target', 'node_modules', '.git', 'bin', 'obj'].includes(p));
  const w1 = vscode.workspace.createFileSystemWatcher('**/*.move');
  w1.onDidCreate((u) => { if (!ignore(u)) schedule(`created ${u.fsPath}`); });
  w1.onDidDelete((u) => { if (!ignore(u)) schedule(`deleted ${u.fsPath}`); });
  ctx.subscriptions.push(w1);
  const w2 = vscode.workspace.createFileSystemWatcher('**/Move.toml');
  w2.onDidCreate((u) => { if (!ignore(u)) schedule(`created ${u.fsPath}`); });
  w2.onDidChange((u) => { if (!ignore(u)) schedule(`changed ${u.fsPath}`); });
  w2.onDidDelete((u) => { if (!ignore(u)) schedule(`deleted ${u.fsPath}`); });
  ctx.subscriptions.push(w2);
  const saveListener = vscode.workspace.onDidSaveTextDocument((doc) => {
    if (doc.languageId === 'move' && !ignore(doc.uri)) {
      schedule(`saved ${doc.uri.fsPath}`);
    }
  });
  ctx.subscriptions.push(saveListener);

  // Install/verify binary on version change.
  let updateVersion = false;
  try {
    const last = ctx.globalState.get<string | null>(VERSION_KEY);
    let doInstall = last === null || last !== version;
    if (last === version) doInstall = !(await uriExists(vscode.Uri.file(serverPath)));
    updateVersion = doInstall;
    if (doInstall && !(await ensureBinary(ctx))) { setStatus('error', 'move-analyzer binary install failed'); return; }
  } catch (e) { log(`install check failed: ${String(e)}`); }

  // Kill any stale server from a previous Reload / second window before
  // spawning, so two instances don't fight over diagnostics (one pushes
  // stale `(0)` and hides real errors).
  await stopInternal();
  killStaleServers();
  try { await start(); }
  catch (e) {
    const m = e instanceof Error ? e.message : String(e);
    log(`Move on Kanari activation failed: ${m}`);
    setStatus('error', m);
    void vscode.window.showWarningMessage(`Move language server failed to start: ${m}`);
  }

  vscode.workspace.onDidChangeConfiguration((event) => {
    if (!event.affectsConfiguration('move.lint')) return;
    lintLevel = vscode.workspace.getConfiguration('move').get<string>('lint') ?? 'default';
    restart().catch((e) => log(`Restart on lint change failed: ${String(e)}`));
  });

  if (updateVersion) await ctx.globalState.update(VERSION_KEY, version);
  log('Move on Kanari activated');
  // Ensure the LSP process dies with the extension host (Reload / Close).
  ctx.subscriptions.push({ dispose: () => { void stopInternal(); } });
}

export async function deactivate(): Promise<void> { await stopInternal(); }


