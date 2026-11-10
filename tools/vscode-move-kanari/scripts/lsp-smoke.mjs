#!/usr/bin/env node
// End-to-end smoke test for the kanari-move-analyzer language server.
//
//   node tools/vscode-move-kanari/scripts/lsp-smoke.mjs [path-to-binary]
//
// It starts the server over stdio, opens a small Move package and checks
// that the server
//   1. answers `initialize`,
//   2. publishes a diagnostic for an unused private function,
//   3. answers a completion request with items.
//
// Exits 0 when everything is fine, 1 otherwise.

import { spawn } from 'node:child_process';
import { mkdtempSync, writeFileSync, mkdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const binary = process.argv[2] || 'target/release/kanari-move-analyzer';
const timeoutMs = 120_000;

const pkg = mkdtempSync(join(tmpdir(), 'mika-lsp-smoke-'));
mkdirSync(join(pkg, 'sources'), { recursive: true });
writeFileSync(
  join(pkg, 'Move.toml'),
  ['[package]', 'name = "smoke"', 'edition = "legacy"', '', '[addresses]', 'smoke = "0x42"', ''].join('\n'),
);
const source = [
  'module smoke::smoke {',
  '',
  '    struct Thing has drop {}',
  '',
  '    fun never_called() {}',
  '',
  '    fun used() {',
  '        let t = Thing {};',
  '    }',
  '}',
  '',
].join('\n');
const file = join(pkg, 'sources', 'smoke.move');
writeFileSync(file, source);

const uri = 'file://' + file.replace(/\\/g, '/');
const proc = spawn(binary, [], { stdio: ['pipe', 'pipe', 'pipe'] });
let buffer = Buffer.alloc(0);
let finished = false;
const diagnostics = [];
let completionCount = -1;

function fail(message) {
  console.error(`FAIL: ${message}`);
  const log = serverLog.join('').trim();
  if (log) console.error(`--- server stderr ---\n${log}\n---------------------`);
  cleanup(1);
}

function cleanup(code) {
  if (finished) return;
  finished = true;
  try { proc.kill(); } catch { /* ignore */ }
  try { rmSync(pkg, { recursive: true, force: true }); } catch { /* ignore */ }
  process.exit(code);
}

function send(message) {
  const body = JSON.stringify(message);
  proc.stdin.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
}

// The server is chatty on stderr; keep it for failure reports only.
let serverLog = [];
proc.stderr.on('data', (chunk) => {
  serverLog.push(chunk.toString());
  if (serverLog.length > 200) serverLog.shift();
});

proc.on('exit', (code) => {
  if (!finished) fail(`server exited early (code ${code})`);
});

proc.stdout.on('data', (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  for (;;) {
    const headerEnd = buffer.indexOf('\r\n\r\n');
    if (headerEnd < 0) return;
    const header = buffer.subarray(0, headerEnd).toString();
    const match = /Content-Length: (\d+)/i.exec(header);
    if (!match) {
      buffer = buffer.subarray(headerEnd + 4);
      continue;
    }
    const length = Number(match[1]);
    const start = headerEnd + 4;
    if (buffer.length < start + length) return;
    const body = buffer.subarray(start, start + length).toString();
    buffer = buffer.subarray(start + length);
    let message;
    try {
      message = JSON.parse(body);
    } catch {
      continue;
    }
    if (message.method && message.id !== undefined && message.id !== null) {
      send({ jsonrpc: '2.0', id: message.id, result: null });
      continue;
    }
    if (message.method === 'textDocument/publishDiagnostics') {
      diagnostics.push(...message.params.diagnostics);
      continue;
    }
    if (message.id === 100) {
      const result = message.result;
      completionCount = Array.isArray(result)
        ? result.length
        : ((result && result.items && result.items.length) || 0);
    }
  }
});

send({
  jsonrpc: '2.0',
  id: 1,
  method: 'initialize',
  params: {
    processId: process.pid,
    rootUri: 'file://' + pkg.replace(/\\/g, '/'),
    capabilities: {},
    initializationOptions: { lintLevel: 'default' },
  },
});

setTimeout(() => {
  send({ jsonrpc: '2.0', method: 'initialized', params: {} });
  send({
    jsonrpc: '2.0',
    method: 'textDocument/didOpen',
    params: { textDocument: { uri, languageId: 'move', version: 1, text: source } },
  });
}, 1500);

// Ask for completions inside the module body (after `struct`).
setTimeout(() => {
  const line = source.split('\n').findIndex((l) => l.includes('struct Thing'));
  send({
    jsonrpc: '2.0',
    id: 100,
    method: 'textDocument/completion',
    params: { textDocument: { uri }, position: { line, character: 4 } },
  });
}, 8000);

setTimeout(() => {
  const unused = diagnostics.filter((d) => String(d.message).includes('never_called'));
  if (unused.length === 0) {
    fail(`expected a diagnostic for the unused function, got ${JSON.stringify(diagnostics)}`);
    return;
  }
  if (unused[0].severity !== 2) {
    fail(`expected the unused-function diagnostic to be a warning (2), got ${unused[0].severity}`);
    return;
  }
  if (completionCount <= 0) {
    fail(`expected completion items, got ${completionCount}`);
    return;
  }
  console.log(
    `PASS: diagnostics=${diagnostics.length} (unused-function warning present) completions=${completionCount}`,
  );
  cleanup(0);
}, timeoutMs);
