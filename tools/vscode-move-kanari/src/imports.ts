// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * "Add import" / "Remove redundant import" quick fixes for Move files.
 *
 * The bundled move-analyzer reports unresolved names but does not offer import edits, and other
 * Move extensions (e.g. Aptos') resolve against the wrong prelude -- they suggest
 * `use std::vector;` even where the 2024 edition already provides `vector` by default, which
 * only produces a duplicate-alias warning. This provider works from the workspace itself
 * (Move.toml + sources on disk), so it suggests imports that actually resolve, and stays quiet
 * where the 2024 prelude already covers the name.
 */

import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';

/** A module that `name` can be imported from: `use <addr>::<mod>;` */
export interface ImportCandidate {
  addr: string;
  mod: string;
  /**
   * When set, a member import instead: `use <addr>::<mod>::<member>;`. Used for types,
   * functions, and constants referenced by bare name (`UID`, `TxContext`, `Coin`), where a
   * module import alone would still leave the usage unresolved.
   */
  member?: string;
}

/** Modules and members the 2024 edition provides without any `use`. */
const PRELUDE_MODULES = new Set(['vector', 'option']);
const PRELUDE_MEMBERS = new Set(['Option']);

/** Well-known stdlib modules, used when dependency scanning finds nothing (e.g. git deps). */
const WELL_KNOWN_STD: ReadonlyArray<ImportCandidate> = [
  { addr: 'std', mod: 'address' },
  { addr: 'std', mod: 'ascii' },
  { addr: 'std', mod: 'bcs' },
  { addr: 'std', mod: 'bit_vector' },
  { addr: 'std', mod: 'error' },
  { addr: 'std', mod: 'fixed_point32' },
  { addr: 'std', mod: 'hash' },
  { addr: 'std', mod: 'option' },
  { addr: 'std', mod: 'signer' },
  { addr: 'std', mod: 'string' },
  { addr: 'std', mod: 'type_name' },
  { addr: 'std', mod: 'unit_test' },
  { addr: 'std', mod: 'vector' },
];

/**
 * Well-known kanari-system modules, same fallback purpose. Only offered when the package
 * actually depends on something kanari-like (checked in `candidatesFor`), so we never suggest
 * an import that cannot resolve.
 */
const WELL_KNOWN_KANARI_SYSTEM: ReadonlyArray<ImportCandidate> = [
  { addr: 'kanari_system', mod: 'address' },
  { addr: 'kanari_system', mod: 'bag' },
  { addr: 'kanari_system', mod: 'balance' },
  { addr: 'kanari_system', mod: 'base64' },
  { addr: 'kanari_system', mod: 'bcs' },
  { addr: 'kanari_system', mod: 'borrow' },
  { addr: 'kanari_system', mod: 'clock' },
  { addr: 'kanari_system', mod: 'coin' },
  { addr: 'kanari_system', mod: 'collection' },
  { addr: 'kanari_system', mod: 'deny_list' },
  { addr: 'kanari_system', mod: 'dynamic_field' },
  { addr: 'kanari_system', mod: 'dynamic_object_field' },
  { addr: 'kanari_system', mod: 'event' },
  { addr: 'kanari_system', mod: 'gas_market' },
  { addr: 'kanari_system', mod: 'hex' },
  { addr: 'kanari_system', mod: 'kanari' },
  { addr: 'kanari_system', mod: 'linked_table' },
  { addr: 'kanari_system', mod: 'math' },
  { addr: 'kanari_system', mod: 'multisig' },
  { addr: 'kanari_system', mod: 'object' },
  { addr: 'kanari_system', mod: 'pay' },
  { addr: 'kanari_system', mod: 'signature_multisig' },
  { addr: 'kanari_system', mod: 'table' },
  { addr: 'kanari_system', mod: 'transfer' },
  { addr: 'kanari_system', mod: 'tx_context' },
  { addr: 'kanari_system', mod: 'types' },
  { addr: 'kanari_system', mod: 'url' },
  { addr: 'kanari_system', mod: 'usd' },
  { addr: 'kanari_system', mod: 'vec_map' },
  { addr: 'kanari_system', mod: 'vec_set' },
  { addr: 'kanari_system', mod: 'zklogin' },
];

//**************************************************************************************************
// Pure helpers (no vscode dependency, unit-testable in plain node)
//**************************************************************************************************

/** Pull the unresolved name out of a diagnostic message, if it is one we handle. */
export function extractUnresolvedName(message: string): string | undefined {
  const patterns = [
    // Kanari / move-analyzer: Could not resolve the name 'hex'
    /Could not resolve the name '([A-Za-z_][A-Za-z0-9_]*)'/,
    // Aptos language server: Unresolved reference `vector`
    /Unresolved reference `([A-Za-z_][A-Za-z0-9_]*)`/,
    // Compiler-style: Unbound variable 'for', Unbound function 'foo', Unbound type
    // 'UID' in current scope, unbound module 'm'
    /[Uu]nbound (?:variable|function|module|struct|constant|type|enum) '([A-Za-z_][A-Za-z0-9_]*)'/,
  ];
  for (const re of patterns) {
    const m = re.exec(message);
    if (m?.[1] !== undefined) {
      return m[1];
    }
  }
  return undefined;
}

/** True when the diagnostic complains that an import is redundant (provided by default). */
export function isDuplicateAliasDiagnostic(message: string): boolean {
  return (
    message.includes('Unnecessary alias') ||
    (message.includes('duplicate alias') &&
      message.includes('provided by default'))
  );
}

/**
 * True for "expected an address or module here, not a module member" diagnostics. This happens
 * when `m::item` is written but `m` resolves to something that is not a module -- typically a
 * local function shadowing a module of the same name (e.g. `fun transfer` shadowing the
 * `transfer` module). The fix is an explicit module import, which lives in a different
 * namespace than functions in Move 2024 and so does not conflict.
 */
export function isModuleMemberPositionDiagnostic(message: string): boolean {
  return (
    message.includes('Expected an address or module in this position') ||
    /unexpected name in this position/i.test(message)
  );
}

/**
 * Leading identifier of a range's text (`transfer` from `transfer` or `transfer::public_x`).
 * Used to recover the shadowed module name for module-member-position diagnostics, whose
 * message does not name it.
 */
export function leadingIdentifier(text: string): string | undefined {
  return /^([A-Za-z_][A-Za-z0-9_]*)/.exec(text)?.[1];
}

/** Walk up from a file looking for Move.toml. Returns the directory holding it. */
export function findPackageRoot(
  startPath: string,
  existsSync: (p: string) => boolean = fs.existsSync,
): string | undefined {
  let dir = path.dirname(startPath);
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

export interface ManifestInfo {
  /** Raw edition string from `[package]`, if present. */
  edition: string | undefined;
  /** Named addresses from `[addresses]` plus `[dev-addresses]`. */
  addresses: Map<string, string>;
  /** Local dependency package dirs: `{ alias, dir }` (dir is absolute). */
  localDeps: Array<{ alias: string; dir: string }>;
  /** Every dependency alias from `[dependencies]`, including git ones we cannot scan. */
  depAliases: string[];
}

/** Minimal Move.toml reader: edition, addresses, and `local` dependencies only. */
export function readManifest(
  pkgDir: string,
  readFileSync: (p: string) => string = (p) => fs.readFileSync(p, 'utf8'),
): ManifestInfo | undefined {
  let text: string;
  try {
    text = readFileSync(path.join(pkgDir, 'Move.toml'));
  } catch {
    return undefined;
  }
  const edition = /^edition\s*=\s*"([^"]+)"/m.exec(text)?.[1];
  const addresses = new Map<string, string>();
  const localDeps: ManifestInfo['localDeps'] = [];
  const depAliases: string[] = [];
  let section = '';
  for (const rawLine of text.split('\n')) {
    const line = rawLine.trim();
    const sectionMatch = /^\[([^\]]+)\]$/.exec(line);
    if (sectionMatch?.[1] !== undefined) {
      section = sectionMatch[1];
      continue;
    }
    if (section === 'addresses' || section === 'dev-addresses') {
      const m = /^([A-Za-z_][A-Za-z0-9_]*)\s*=\s*"([^"]+)"/.exec(line);
      if (m?.[1] !== undefined && m?.[2] !== undefined) {
        addresses.set(m[1], m[2]);
      }
    } else if (section === 'dependencies' || section === 'dev-dependencies') {
      const alias = /^([A-Za-z_][A-Za-z0-9_]*)\s*=/.exec(line)?.[1];
      if (alias !== undefined && !depAliases.includes(alias)) {
        depAliases.push(alias);
      }
      const m =
        /^([A-Za-z_][A-Za-z0-9_]*)\s*=\s*\{[^}]*local\s*=\s*"([^"]+)"[^}]*\}/.exec(
          line,
        );
      if (m?.[1] !== undefined && m?.[2] !== undefined) {
        localDeps.push({ alias: m[1], dir: path.resolve(pkgDir, m[2]) });
      }
    }
  }
  return { edition, addresses, localDeps, depAliases };
}

/** Well-known member imports for the most common bare-name references. */
const WELL_KNOWN_MEMBERS: ReadonlyArray<Required<ImportCandidate>> = [
  { addr: 'std', mod: 'option', member: 'Option' },
  { addr: 'std', mod: 'string', member: 'String' },
  { addr: 'std', mod: 'ascii', member: 'String' },
  { addr: 'kanari_system', mod: 'object', member: 'UID' },
  { addr: 'kanari_system', mod: 'object', member: 'ID' },
  { addr: 'kanari_system', mod: 'tx_context', member: 'TxContext' },
  { addr: 'kanari_system', mod: 'coin', member: 'Coin' },
  { addr: 'kanari_system', mod: 'coin', member: 'TreasuryCap' },
  { addr: 'kanari_system', mod: 'coin', member: 'CoinMetadata' },
  { addr: 'kanari_system', mod: 'balance', member: 'Balance' },
  { addr: 'kanari_system', mod: 'balance', member: 'Supply' },
  { addr: 'kanari_system', mod: 'clock', member: 'Clock' },
];

/** True when the 2024 prelude (`vector`, `option`, `Option`) is active for a package. */
export function hasPrelude(manifest: ManifestInfo | undefined): boolean {
  // The Kanari v2 toolchain defaults to 2024, so only an explicit `legacy` opts out.
  return manifest?.edition !== 'legacy';
}

/** Collect `module <addr>::<mod>` and 2024-style `module <mod>;` declarations in a dir. */
export function scanModules(
  sourcesDir: string,
  packageName: string | undefined,
  packageAddr: string | undefined,
  readdirSync: (p: string) => string[] = fs.readdirSync,
  readFileSync: (p: string) => string = (p) => fs.readFileSync(p, 'utf8'),
): ImportCandidate[] {
  const out: ImportCandidate[] = [];
  let files: string[];
  try {
    files = readdirSync(sourcesDir);
  } catch {
    return out;
  }
  for (const file of files) {
    if (!file.endsWith('.move')) {
      continue;
    }
    let text: string;
    try {
      text = readFileSync(path.join(sourcesDir, file));
    } catch {
      continue;
    }
    for (const m of text.matchAll(
      /module\s+([A-Za-z_][A-Za-z0-9_]*)\s*::\s*([A-Za-z_][A-Za-z0-9_]*)\s*\{/g,
    )) {
      const addr = m[1];
      const mod = m[2];
      if (addr !== undefined && mod !== undefined) {
        out.push({ addr, mod });
      }
    }
    // 2024 `module <name>;` form: address comes from the package's named addresses.
    if (packageName !== undefined && packageAddr !== undefined) {
      for (const m of text.matchAll(/module\s+([A-Za-z_][A-Za-z0-9_]*)\s*;/g)) {
        const mod = m[1];
        if (mod !== undefined) {
          out.push({ addr: packageName, mod });
        }
      }
    }
  }
  return out;
}

/**
 * Collect public members (`struct`, `enum`, `const`, `fun`) per module in a dir, for member
 * imports (`use addr::mod::Name;`). Only `public` members are offered: anything else cannot be
 * named from another module. Returns `{ addr, mod, member }` triples.
 */
export function scanMembers(
  sourcesDir: string,
  readdirSync: (p: string) => string[] = fs.readdirSync,
  readFileSync: (p: string) => string = (p) => fs.readFileSync(p, 'utf8'),
): Required<ImportCandidate>[] {
  const out: Required<ImportCandidate>[] = [];
  // Map each file's module path first, then attach its public members to it.
  let files: string[];
  try {
    files = readdirSync(sourcesDir);
  } catch {
    return out;
  }
  for (const file of files) {
    if (!file.endsWith('.move')) {
      continue;
    }
    let text: string;
    try {
      text = readFileSync(path.join(sourcesDir, file));
    } catch {
      continue;
    }
    const modMatch =
      /module\s+([A-Za-z_][A-Za-z0-9_]*)\s*::\s*([A-Za-z_][A-Za-z0-9_]*)\s*\{/.exec(
        text,
      );
    if (modMatch?.[1] === undefined || modMatch?.[2] === undefined) {
      continue;
    }
    const addr = modMatch[1];
    const mod = modMatch[2];
    const memberRe =
      /public\s+(?:struct|enum|fun|entry\s+fun|macro\s+fun|const)\s+([A-Za-z_][A-Za-z0-9_]*)/g;
    for (const m of text.matchAll(memberRe)) {
      if (m[1] !== undefined) {
        out.push({ addr, mod, member: m[1] });
      }
    }
    // Module-level `const` without `public` is still importable in practice for constants.
    for (const m of text.matchAll(/^\s*const\s+([A-Za-z_][A-Za-z0-9_]*)/gm)) {
      if (m[1] !== undefined) {
        out.push({ addr, mod, member: m[1] });
      }
    }
  }
  return out;
}

/**
 * Line (0-based) after which a new top-level `use` should be inserted: after the last existing
 * top-level `use ...;`, else after the module declaration line. Returns undefined when no module
 * declaration is found.
 */
export function findImportInsertLine(
  lines: ReadonlyArray<string>,
): { line: number; indent: string } | undefined {
  let moduleLine: number | undefined;
  let lastUseLine: number | undefined;
  let lastUseIndent = '';
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i] ?? '';
    if (/^\s*(?:#[^\n]*\s*)*module\s+[A-Za-z_][A-Za-z0-9_:]*\s*[{;]/.test(line)) {
      if (moduleLine === undefined) {
        moduleLine = i;
      }
      continue;
    }
    const useMatch = /^(\s*)use\s+[^;]+;/.exec(line);
    if (useMatch !== null && moduleLine !== undefined) {
      lastUseLine = i;
      lastUseIndent = useMatch[1] ?? '';
    }
  }
  if (lastUseLine !== undefined) {
    return { line: lastUseLine + 1, indent: lastUseIndent };
  }
  if (moduleLine !== undefined) {
    return { line: moduleLine + 1, indent: '    ' };
  }
  return undefined;
}

//**************************************************************************************************
// VS Code provider
//**************************************************************************************************

export class MoveImportCodeActionProvider implements vscode.CodeActionProvider {
  public static readonly providedKinds: ReadonlyArray<vscode.CodeActionKind> = [
    vscode.CodeActionKind.QuickFix,
  ];

  public provideCodeActions(
    document: vscode.TextDocument,
    _range: vscode.Range | vscode.Selection,
    context: vscode.CodeActionContext,
  ): Array<vscode.CodeAction> {
    const actions: vscode.CodeAction[] = [];
    // Every (module-only) candidate offered, grouped for the merged-import pass below.
    const offeredByModule = new Map<string, ImportCandidate[]>();
    const recordOffered = (c: ImportCandidate): void => {
      const key = `${c.addr}::${c.mod}`;
      const list = offeredByModule.get(key);
      if (list === undefined) {
        offeredByModule.set(key, [c]);
      } else {
        list.push(c);
      }
    };
    for (const diag of context.diagnostics) {
      // Offer to delete imports the compiler flags as provided-by-default.
      if (isDuplicateAliasDiagnostic(diag.message)) {
        const remove = this.removeImportAction(document, diag);
        if (remove !== undefined) {
          actions.push(remove);
        }
        continue;
      }
      const name = extractUnresolvedName(diag.message);
      if (name === undefined) {
        // A shadowed module (`transfer::f` where a local `fun transfer` hides the module)
        // reports no name; recover it from the squiggled range instead and suggest only the
        // module import, which fixes every `transfer::...` use at once.
        if (isModuleMemberPositionDiagnostic(diag.message)) {
          const shadowed = leadingIdentifier(document.getText(diag.range));
          if (shadowed !== undefined) {
            for (const candidate of this.candidatesFor(document, shadowed).filter(
              (c) => c.member === undefined,
            )) {
              const add = this.addImportAction(document, shadowed, candidate);
              if (add !== undefined) {
                actions.push(add);
                recordOffered(candidate);
              }
            }
          }
        }
        continue;
      }
      for (const candidate of this.candidatesFor(document, name)) {
        const add = this.addImportAction(document, name, candidate);
        if (add !== undefined) {
          actions.push(add);
          recordOffered(candidate);
        }
      }
    }
    // Where one module drew both a module need (`tx_context::sender`) and member needs (bare
    // `TxContext`), offer a single merged `use addr::mod::{Self, M1, ...}` that fixes both at
    // once, instead of two separate imports. Bare usages already in the file count too, so the
    // merged form is offered even when only the module name has a diagnostic so far.
    for (const [, group] of offeredByModule) {
      const modCandidate = group.find((c) => c.member === undefined);
      if (modCandidate === undefined) {
        continue;
      }
      const members = new Set<string>(
        group
          .map((c) => c.member)
          .filter((m): m is string => m !== undefined),
      );
      for (const m of this.bareUsedMembers(
        document,
        this.membersOfModule(document, modCandidate.addr, modCandidate.mod),
      )) {
        members.add(m);
      }
      if (
        members.size > 0 &&
        this.noNestedImportYet(document, modCandidate)
      ) {
        const merged = this.addMergedImportAction(
          document,
          modCandidate.addr,
          modCandidate.mod,
          [...members],
        );
        if (merged !== undefined) {
          actions.push(merged);
        }
      }
    }
    return actions;
  }

  /** True when the file has no nested `use addr::mod::{...}` yet (a merged offer is clean). */
  private noNestedImportYet(
    document: vscode.TextDocument,
    candidate: ImportCandidate,
  ): boolean {
    for (let i = 0; i < document.lineCount; i++) {
      if (
        new RegExp(
          `^\\s*use\\s+(?:${candidate.addr}::)?${candidate.mod}\\s*::\\s*\\{`,
        ).test(document.lineAt(i).text)
      ) {
        return false;
      }
    }
    return true;
  }

  /**
   * Member names of `addr::mod` known from the workspace and the well-known tables. Used to
   * offer a merged `use addr::mod::{Self, ...}` that also covers bare usages already in the
   * file, so one edit fixes both `mod::f` and bare `T` references.
   */
  private membersOfModule(
    document: vscode.TextDocument,
    addr: string,
    mod: string,
  ): string[] {
    const found = new Set<string>();
    const pkgDir = findPackageRoot(document.uri.fsPath);
    if (pkgDir !== undefined) {
      const dirs = [
        path.join(pkgDir, 'sources'),
        path.join(pkgDir, 'tests'),
      ];
      const manifest = readManifest(pkgDir);
      for (const dep of manifest?.localDeps ?? []) {
        dirs.push(path.join(dep.dir, 'sources'));
      }
      for (const dir of dirs) {
        for (const c of scanMembers(dir)) {
          if (c.addr === addr && c.mod === mod) {
            found.add(c.member);
          }
        }
      }
    }
    for (const c of WELL_KNOWN_MEMBERS) {
      if (c.addr === addr && c.mod === mod) {
        found.add(c.member);
      }
    }
    return [...found];
  }

  /**
   * Subset of `members` used bare in the document: appears as a word that is neither preceded
   * by `::` (i.e. not already `mod::M`) nor inside a `use` line.
   */
  private bareUsedMembers(
    document: vscode.TextDocument,
    members: ReadonlyArray<string>,
  ): string[] {
    const used: string[] = [];
    for (const m of members) {
      const re = new RegExp(`(?<!:)\\b${m}\\b(?!:)`);
      for (let i = 0; i < document.lineCount; i++) {
        const line = document.lineAt(i).text;
        if (/^\s*use\s/.test(line)) {
          continue;
        }
        if (re.test(line)) {
          used.push(m);
          break;
        }
      }
    }
    return used;
  }

  /** `Add import for 'addr::mod::{Self, M1, ...}'` as a single edit. */
  private addMergedImportAction(
    document: vscode.TextDocument,
    addr: string,
    mod: string,
    members: string[],
  ): vscode.CodeAction | undefined {
    const lines: string[] = [];
    for (let i = 0; i < document.lineCount; i++) {
      lines.push(document.lineAt(i).text);
    }
    const at = findImportInsertLine(lines);
    if (at === undefined) {
      return undefined;
    }
    const inner = ['Self', ...members].join(', ');
    const action = new vscode.CodeAction(
      `Add import for \`${addr}::${mod}::{${inner}}\``,
      vscode.CodeActionKind.QuickFix,
    );
    const edit = new vscode.WorkspaceEdit();
    edit.insert(
      document.uri,
      new vscode.Position(at.line, 0),
      `${at.indent}use ${addr}::${mod}::{${inner}};\n`,
    );
    action.edit = edit;
    return action;
  }

  private candidatesFor(
    document: vscode.TextDocument,
    name: string,
  ): ImportCandidate[] {    const pkgDir = findPackageRoot(document.uri.fsPath);
    const manifest =
      pkgDir === undefined ? undefined : readManifest(pkgDir);
    // The 2024 prelude already provides these; suggesting an import only creates noise.
    if (
      (PRELUDE_MODULES.has(name) || PRELUDE_MEMBERS.has(name)) &&
      hasPrelude(manifest)
    ) {
      return [];
    }
    const found: ImportCandidate[] = [];
    const seen = new Set<string>();
    const push = (c: ImportCandidate): void => {
      const key =
        c.member === undefined
          ? `${c.addr}::${c.mod}`
          : `${c.addr}::${c.mod}::${c.member}`;
      if (!seen.has(key)) {
        seen.add(key);
        found.push(c);
      }
    };
    // Member matches first: a bare `UID` / `TxContext` / `Coin` is fixed by a member import
    // (`use kanari_system::object::UID;`), which needs no rewrite of the usage site. A module
    // import alone would still leave `UID` unresolved.
    const pushMemberScans = (dirs: string[]): void => {
      for (const dir of dirs) {
        for (const c of scanMembers(dir)) {
          if (c.member === name) {
            push(c);
          }
        }
      }
    };
    if (pkgDir !== undefined) {
      // Own package first (both `sources` and `tests`): same-package modules are the most
      // likely target, e.g. `hex::encode` inside `kanari_system`.
      const pkgName = this.packageName(pkgDir, manifest);
      const pkgAddr = pkgName !== undefined ? manifest?.addresses.get(pkgName) : undefined;
      for (const sub of ['sources', 'tests']) {
        for (const c of scanModules(
          path.join(pkgDir, sub),
          pkgName,
          pkgAddr,
        )) {
          if (c.mod === name) {
            push(c);
          }
        }
      }
      pushMemberScans([path.join(pkgDir, 'sources'), path.join(pkgDir, 'tests')]);
      // Then local dependencies from Move.toml.
      const depDirs: string[] = [];
      for (const dep of manifest?.localDeps ?? []) {
        for (const c of scanModules(path.join(dep.dir, 'sources'), undefined, undefined)) {
          if (c.mod === name) {
            push(c);
          }
        }
        depDirs.push(path.join(dep.dir, 'sources'));
      }
      pushMemberScans(depDirs);
    }
    // Fallback so well-known modules always resolve even when dep scanning misses
    // (git dependencies, missing manifest). std is near-universal; kanari_system is only
    // offered when the package actually depends on something kanari-like, so we never suggest
    // an import that cannot resolve.
    for (const c of WELL_KNOWN_STD) {
      if (c.mod === name) {
        push(c);
      }
    }
    const wantsKanari = (manifest?.depAliases ?? []).some((a) =>
      /kanari/i.test(a),
    );
    if (wantsKanari) {
      for (const c of WELL_KNOWN_KANARI_SYSTEM) {
        if (c.mod === name) {
          push(c);
        }
      }
    }
    // Well-known member imports last (lowest priority after workspace-resolved ones).
    for (const c of WELL_KNOWN_MEMBERS) {
      if (c.member === name) {
        if (c.addr !== 'kanari_system' || wantsKanari) {
          push(c);
        }
      }
    }
    return found;
  }

  private packageName(
    pkgDir: string,
    manifest: ManifestInfo | undefined,
  ): string | undefined {
    if (manifest !== undefined) {
      try {
        const text = fs.readFileSync(path.join(pkgDir, 'Move.toml'), 'utf8');
        const name = /^name\s*=\s*"([^"]+)"/m.exec(text)?.[1];
        if (name !== undefined) {
          return name;
        }
      } catch {
        // fall through
      }
    }
    return path.basename(pkgDir);
  }

  private addImportAction(
    document: vscode.TextDocument,
    _name: string,
    candidate: ImportCandidate,
  ): vscode.CodeAction | undefined {
    const lines: string[] = [];
    for (let i = 0; i < document.lineCount; i++) {
      lines.push(document.lineAt(i).text);
    }
    const at = findImportInsertLine(lines);
    if (at === undefined) {
      return undefined;
    }
    const importPath =
      candidate.member === undefined
        ? `${candidate.addr}::${candidate.mod}`
        : `${candidate.addr}::${candidate.mod}::${candidate.member}`;
    // A bare module import does NOT count for a member candidate: `use m::object;` still
    // leaves a bare `UID` unresolved. Conversely, a nested `use m::{...};` does NOT count as
    // the module import: only a plain `use [addr::]mod;` provides the `mod::` prefix.
    if (candidate.member === undefined) {
      const alreadyThere = lines.some((l) =>
        new RegExp(
          `^\\s*use\\s+(?:${candidate.addr}::)?${candidate.mod}\\s*(?:as\\s+[A-Za-z_][A-Za-z0-9_]*)?\\s*;\\s*$`,
        ).test(l),
      );
      if (alreadyThere) {
        return undefined;
      }
      const action = new vscode.CodeAction(
        `Add import for \`${importPath}\``,
        vscode.CodeActionKind.QuickFix,
      );
      const edit = new vscode.WorkspaceEdit();
      edit.insert(
        document.uri,
        new vscode.Position(at.line, 0),
        `${at.indent}use ${importPath};\n`,
      );
      action.edit = edit;
      return action;
    }
    // Member candidate: merge into an existing member import of the same module instead of
    // adding another line, so `use m::{Coin}` + need `TreasuryCap` becomes `use m::{Coin,
    // TreasuryCap}` rather than a second `use m::TreasuryCap;` line.
    return this.addMemberImportAction(
      document,
      lines,
      at,
      candidate as Required<ImportCandidate>,
    );
  }

  private addMemberImportAction(
    document: vscode.TextDocument,
    lines: ReadonlyArray<string>,
    at: { line: number; indent: string },
    candidate: Required<ImportCandidate>,
  ): vscode.CodeAction | undefined {
    const { addr, mod, member } = candidate;
    const modRe = `(?:${addr}::)?${mod}`;
    // Collect existing member imports of this exact module (skip aliased ones, which cannot
    // be merged textually).
    const nestedIdx: number[] = [];
    const singleIdx: number[] = [];
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i] ?? '';
      if (new RegExp(`^\\s*use\\s+${modRe}::\\{([^}]*)\\};?\\s*$`).test(line)) {
        nestedIdx.push(i);
      } else if (
        new RegExp(`^\\s*use\\s+${modRe}::([A-Za-z_][A-Za-z0-9_]*)\\s*;\\s*$`).test(line)
      ) {
        singleIdx.push(i);
      }
    }
    const memberList = (inner: string): string[] =>
      inner
        .split(',')
        .map((s) => s.trim())
        .filter((s) => s.length > 0 && !/\bas\b/.test(s));
    // Already covered by a nested import?
    for (const i of nestedIdx) {
      const inner = new RegExp(
        `^\\s*use\\s+${modRe}::\\{([^}]*)\\};?\\s*$`,
      ).exec(lines[i] ?? '')?.[1];
      if (inner !== undefined && memberList(inner).includes(member)) {
        return undefined;
      }
    }
    const action = new vscode.CodeAction(
      `Add import for \`${addr}::${mod}::${member}\``,
      vscode.CodeActionKind.QuickFix,
    );
    const edit = new vscode.WorkspaceEdit();
    const retitle = (form: string): void => {
      action.title = `Add import for \`${form}\``;
    };
    if (nestedIdx.length > 0) {
      // Append to the first nested import; drop any other member lines for this module so
      // everything ends up in one place.
      const first = nestedIdx[0] ?? 0;
      const line = lines[first] ?? '';
      const inner =
        new RegExp(`^\\s*use\\s+${modRe}::\\{([^}]*)\\};?\\s*$`).exec(line)?.[1] ?? '';
      const members = memberList(inner);
      if (!members.includes(member)) {
        members.push(member);
      }
      const indent = /^(\s*)use/.exec(line)?.[1] ?? at.indent;
      edit.replace(
        document.uri,
        new vscode.Range(
          new vscode.Position(first, 0),
          new vscode.Position(first, line.length),
        ),
        `${indent}use ${addr}::${mod}::{${members.join(', ')}};`,
      );
      for (const i of [...nestedIdx.slice(1), ...singleIdx]) {
        edit.delete(
          document.uri,
          new vscode.Range(
            new vscode.Position(i, 0),
            new vscode.Position(i + 1, 0),
          ),
        );
      }
      retitle(`${addr}::${mod}::{${members.join(', ')}}`);
    } else if (singleIdx.length > 0) {
      // Combine separate single-member lines into one nested import.
      const members: string[] = [];
      for (const i of singleIdx) {
        const m = new RegExp(
          `^\\s*use\\s+${modRe}::([A-Za-z_][A-Za-z0-9_]*)\\s*;\\s*$`,
        ).exec(lines[i] ?? '')?.[1];
        if (m !== undefined && !members.includes(m)) {
          members.push(m);
        }
      }
      if (!members.includes(member)) {
        members.push(member);
      }
      const first = singleIdx[0] ?? 0;
      const indent =
        /^(\s*)use/.exec(lines[first] ?? '')?.[1] ?? at.indent;
      edit.replace(
        document.uri,
        new vscode.Range(
          new vscode.Position(first, 0),
          new vscode.Position(first, (lines[first] ?? '').length),
        ),
        `${indent}use ${addr}::${mod}::{${members.join(', ')}};`,
      );
      for (const i of singleIdx.slice(1)) {
        edit.delete(
          document.uri,
          new vscode.Range(
            new vscode.Position(i, 0),
            new vscode.Position(i + 1, 0),
          ),
        );
      }
      retitle(`${addr}::${mod}::{${members.join(', ')}}`);
    } else {
      edit.insert(
        document.uri,
        new vscode.Position(at.line, 0),
        `${at.indent}use ${addr}::${mod}::${member};\n`,
      );
    }
    action.edit = edit;
    return action;
  }

  private removeImportAction(
    document: vscode.TextDocument,
    diag: vscode.Diagnostic,
  ): vscode.CodeAction | undefined {
    // Delete the whole `use ...;` line the diagnostic points at.
    const line = document.lineAt(diag.range.start.line);
    if (!/^\s*use\s+[^;]+;/.test(line.text)) {
      return undefined;
    }
    const action = new vscode.CodeAction(
      'Remove redundant import (provided by default)',
      vscode.CodeActionKind.QuickFix,
    );
    const edit = new vscode.WorkspaceEdit();
    edit.delete(
      document.uri,
      new vscode.Range(
        new vscode.Position(diag.range.start.line, 0),
        new vscode.Position(diag.range.start.line + 1, 0),
      ),
    );
    action.edit = edit;
    return action;
  }
}
