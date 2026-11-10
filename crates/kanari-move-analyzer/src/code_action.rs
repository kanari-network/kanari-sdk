// Import quick-fixes + formatting + code lens, ported from the old
// TS providers (features/imports/*, features/formatProvider.ts,
// features/codelens.ts). Everything runs in the Rust LS now so the TS
// client stays a thin launcher.
//
// Grouping rule (user requirement):
//  Items coming from the same module must stay in the SAME import:
//     use deepbook::deepbook::{OrderBook, create_order_book_with_decimals};
//  not five separate `use deepbook::deepbook::X;` lines. When an existing
//  grouped/single import for that module is present, the new members are
//  merged into it (replace first, delete the rest).

use crate::edition::{MoveEdition, edition_for_file, is_prelude};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionParams, Range, TextEdit, Uri, WorkspaceEdit,
};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// member name -> (addr, module)
pub type MembersMap = HashMap<String, (String, String)>;
pub type ModulesMap = HashMap<String, (String, String)>;

fn uri_to_file_path(uri: &lsp_types::Uri) -> Result<std::path::PathBuf, String> {
    url::Url::parse(uri.as_str())
        .map_err(|e| e.to_string())?
        .to_file_path()
        .map_err(|_| "URI is not a valid file URI".to_string())
}

#[allow(clippy::mutable_key_type)]
pub fn on_code_action_request(params: &CodeActionParams) -> Vec<CodeAction> {
    on_code_action_request_with_text(params, None)
}

/// Code action with the editor's *current* (possibly unsaved) text, so
/// quick-fixes (underscore rename, import insert) act on what the user
/// sees — not on stale disk content. `None` falls back to disk.
#[allow(clippy::mutable_key_type)]
pub fn on_code_action_request_with_text(
    params: &CodeActionParams,
    current_text: Option<String>,
) -> Vec<CodeAction> {
    let file_path = uri_to_file_path(&params.text_document.uri).ok();
    let edition = file_path
        .as_deref()
        .map(edition_for_file)
        .unwrap_or(MoveEdition::Unknown);
    let file_text = match current_text {
        Some(t) => t,
        None => file_path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_default(),
    };
    let lines: Vec<&str> = file_text.lines().collect();

    let (all_members, all_modules) = file_path
        .as_deref()
        .and_then(package_root)
        .map(|root| scan_all_members_and_modules(&root))
        .unwrap_or_default();

    // --- 1. group unresolved names by (addr, module) ---------------------
    // key -> (diag indices, member names)
    let mut groups: HashMap<(String, String), (Vec<usize>, HashSet<String>)> = HashMap::new();
    let mut module_only: HashMap<(String, String), Vec<usize>> = HashMap::new();
    let mut actions = Vec::new();

    for (i, diag) in params.context.diagnostics.iter().enumerate() {
        // Unused param/var → offer rename to `_name`.
        if let Some(unused_name) = extract_unused_name(&diag.message) {
            if let Some(edit) = rename_to_underscored(&lines, diag.range, &unused_name) {
                let mut changes = HashMap::new();
                changes.insert(params.text_document.uri.clone(), edit);
                actions.push(CodeAction {
                    title: format!("Rename '{unused_name}' to '_{unused_name}'"),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![diag.clone()]),
                    edit: Some(WorkspaceEdit {
                        changes: Some(changes),
                        ..Default::default()
                    }),
                    is_preferred: Some(true),
                    ..Default::default()
                });
            }
            continue;
        }
        if diag.message.contains("Unnecessary alias")
            || (diag.message.contains("duplicate alias")
                && diag.message.contains("provided by default"))
        {
            continue;
        }
        let Some(name) = extract_unresolved_name(&diag.message) else {
            continue;
        };
        if is_prelude(&name, &edition) {
            continue;
        }
        // When the file uses `name::...` the unresolved identifier is a
        // module alias, not a bare member. Prefer the module over a member
        // with the same name in another module (e.g. `balance::transfer`
        // must NOT shadow `transfer` module itself).
        let module_alias_used = module_used_as_alias(&file_text, &name);
        if module_alias_used && let Some((addr, module)) = all_modules.get(&name) {
            module_only
                .entry((addr.clone(), module.clone()))
                .or_default()
                .push(i);
            continue;
        }
        if let Some((addr, module)) = all_members.get(&name) {
            let e = groups
                .entry((addr.clone(), module.clone()))
                .or_insert((Vec::new(), HashSet::new()));
            e.0.push(i);
            e.1.insert(name.clone());
        } else if let Some((addr, module)) = all_modules.get(&name) {
            module_only
                .entry((addr.clone(), module.clone()))
                .or_default()
                .push(i);
        }
    }

    // --- 2. one merged action per member-group ---------------------------
    for ((addr, module), (diag_idxs, members)) in groups {
        let needs_self =
            module_used_as_alias(&file_text, &module) || existing_has_self(&lines, &module);
        let diags: Vec<_> = diag_idxs
            .iter()
            .map(|&i| params.context.diagnostics[i].clone())
            .collect();
        if let Some(we) = merged_import_edit(&lines, &addr, &module, &members, needs_self) {
            let mut changes = HashMap::new();
            changes.insert(params.text_document.uri.clone(), we);
            let title = format!(
                "Add import for `{addr}::{module}::{{{}}}`",
                merged_members(&members, needs_self).join(", ")
            );
            actions.push(CodeAction {
                title,
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(diags),
                edit: Some(WorkspaceEdit {
                    changes: Some(changes),
                    ..Default::default()
                }),
                is_preferred: Some(needs_self || members.len() > 1),
                ..Default::default()
            });
        }
    }

    // --- 3. module-only groups -------------------------------------------
    for ((addr, module), diag_idxs) in module_only {
        let diags: Vec<_> = diag_idxs
            .iter()
            .map(|&i| params.context.diagnostics[i].clone())
            .collect();
        let import_path = format!("{addr}::{module}");
        if let Some(edit) = insert_import_edit(&lines, &import_path) {
            let mut we = WorkspaceEdit::default();
            we.changes
                .get_or_insert_with(HashMap::new)
                .entry(params.text_document.uri.clone())
                .or_default()
                .push(edit);
            actions.push(CodeAction {
                title: format!("Add import for `{import_path}`"),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(diags),
                edit: Some(we),
                is_preferred: None,
                ..Default::default()
            });
        }
    }

    actions.sort_by_key(|a| (!a.is_preferred.unwrap_or(false), a.title.clone()));
    actions
}

/// Members of `addr::mod` already present in `use` lines, plus whether any
/// of them is the nested `{...}` form.
fn existing_import_line_indices(
    lines: &[&str],
    _addr: &str,
    module: &str,
) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
    let mut nested = Vec::new();
    let mut single = Vec::new();
    let mut module_only = Vec::new();
    for (i, raw) in lines.iter().enumerate() {
        let t = raw.trim();
        if !t.starts_with("use ") || !t.ends_with(';') {
            continue;
        }
        let body = t.trim_start_matches("use ").trim_end_matches(';').trim();
        let nested_pat = format!("::{module}::{{");
        let single_pat = format!("::{module}::");
        let module_only_pat = format!("::{module}");
        if body.contains(&nested_pat) {
            nested.push(i);
        } else if body.contains(&single_pat) {
            single.push(i);
        } else if body.ends_with(&module_only_pat) {
            module_only.push(i);
        } else if body.starts_with(&format!("{module}::")) {
            single.push(i);
        }
    }
    (nested, single, module_only)
}

fn existing_has_self(lines: &[&str], module: &str) -> bool {
    for raw in lines {
        let t = raw.trim();
        if !t.starts_with("use ") {
            continue;
        }
        if t.contains(&format!("::{module}::{{"))
            && t.split("::{")
                .nth(1)
                .is_some_and(|inner| inner.trim().starts_with("Self"))
        {
            return true;
        }
    }
    false
}

fn existing_members(lines: &[&str], addr: &str, module: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let nested_pat = format!("::{module}::{{");
    let single_pat = format!("::{module}::");
    let addr_prefix = format!("{addr}::{module}");
    for raw in lines {
        let t = raw.trim();
        if !t.starts_with("use ") || !t.ends_with(';') {
            continue;
        }
        let body = t.trim_start_matches("use ").trim_end_matches(';').trim();
        if body.contains(&nested_pat) || body.contains(&single_pat) || body == addr_prefix {
            // extract members inside { ... } or after the last ::
            let inner = if let Some(lib) = body.find("::{") {
                &body[lib + 3..]
            } else if let Some(idx) = body.rfind("::") {
                &body[idx + 2..]
            } else {
                body
            };
            for token in inner.split([',', '{', '}']) {
                let tok = token.trim();
                if !tok.is_empty() && tok != "as" && !tok.starts_with("as ") {
                    out.insert(tok.to_string());
                }
            }
        }
    }
    out
}

fn merged_members(members: &HashSet<String>, needs_self: bool) -> Vec<String> {
    let mut v: Vec<String> = members
        .iter()
        .filter(|m| m.as_str() != "Self")
        .cloned()
        .collect();
    v.sort();
    if needs_self {
        v.insert(0, "Self".to_string());
    }
    v
}

/// Build a WorkspaceEdit that ends with exactly ONE line
/// `use addr::mod::{Self, A, B};` for this module (merging into any
/// existing import line for it).
fn merged_import_edit(
    lines: &[&str],
    addr: &str,
    module: &str,
    members: &HashSet<String>,
    needs_self: bool,
) -> Option<Vec<TextEdit>> {
    let mut all: HashSet<String> = members.clone();
    for m in existing_members(lines, addr, module) {
        // strip Self marker temporarily, re-added by needs_self
        if m != "Self" && !m.is_empty() {
            all.insert(m);
        }
    }
    let inner = merged_members(&all, needs_self);
    if inner.is_empty() {
        return None;
    }
    let new_line = format!("use {addr}::{module}::{{{}}};", inner.join(", "));
    let (nested, single, module_only) = existing_import_line_indices(lines, addr, module);
    let mut edits = Vec::new();
    let first_existing = nested.first().or(single.first()).or(module_only.first());
    if let Some(&first) = first_existing {
        // replace first existing line with merged form
        edits.push(TextEdit {
            range: Range::new(
                lsp_types::Position::new(first as u32, 0),
                lsp_types::Position::new(first as u32, lines[first].len() as u32),
            ),
            new_text: new_line,
        });
        // delete other lines for the same module
        for idx in nested.iter().chain(single.iter()).chain(module_only.iter()) {
            if idx != &first {
                edits.push(TextEdit {
                    range: Range::new(
                        lsp_types::Position::new(*idx as u32, 0),
                        lsp_types::Position::new(*idx as u32 + 1, 0),
                    ),
                    new_text: String::new(),
                });
            }
        }
        Some(edits)
    } else {
        insert_import_edit(
            lines,
            &format!("{addr}::{module}::{{{}}}", inner.join(", ")),
        )
        .map(|e| vec![e])
    }
}

pub fn extract_unresolved_name(message: &str) -> Option<String> {
    let patterns = [
        "Could not resolve the name '",
        "Unresolved reference `",
        "Unbound variable '",
        "Unbound function '",
        "Unbound module '",
        "Unbound module alias '",
        "Unbound struct '",
        "Unbound constant '",
        "Unbound type '",
        "Unbound enum '",
    ];
    let mut best: Option<(usize, String)> = None;
    for pat in patterns {
        if let Some(idx) = message.find(pat) {
            if idx > 0 {
                let prev = message[..idx].chars().next_back();
                if prev.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
                    continue;
                }
            }
            let rest = &message[idx + pat.len()..];
            let end = rest
                .find(|c: char| c == '\'' || c == '`' || (!c.is_ascii_alphanumeric() && c != '_'))
                .unwrap_or(rest.len());
            let name = &rest[..end];
            if !name.is_empty()
                && name
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && best.as_ref().is_none_or(|(bidx, _)| idx < *bidx)
            {
                best = Some((idx, name.to_string()));
            }
        }
    }
    best.map(|(_, n)| n)
}

/// `Unused parameter 'x'. Consider ... prefixing with an underscore: '_x'`
/// (Move compiler `UnusedItem::Variable`). Also matches the
/// `Unused local variable 'x'` / `Unused variable 'x'` forms.
pub fn extract_unused_name(message: &str) -> Option<String> {
    let patterns = [
        "Unused parameter '",
        "Unused local variable '",
        "Unused variable '",
    ];
    let mut best: Option<(usize, String)> = None;
    for pat in patterns {
        if let Some(idx) = message.find(pat) {
            if idx > 0 {
                let prev = message[..idx].chars().next_back();
                if prev.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
                    continue;
                }
            }
            let rest = &message[idx + pat.len()..];
            let end = rest.find('\'').unwrap_or(rest.len());
            let name = &rest[..end];
            if !name.is_empty()
                && name
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && !name.starts_with('_')
                && best.as_ref().is_none_or(|(bidx, _)| idx < *bidx)
            {
                best = Some((idx, name.to_string()));
            }
        }
    }
    best.map(|(_, n)| n)
}

/// Build a `TextEdit` that renames the identifier at `range` to
/// `_{name}`. Returns `None` when the name already starts with `_`
/// or the identifier cannot be located on the target line.
pub fn rename_to_underscored(lines: &[&str], range: Range, name: &str) -> Option<Vec<TextEdit>> {
    if name.is_empty() || name.starts_with('_') {
        return None;
    }
    let line_idx = range.start.line as usize;
    let line_text = lines.get(line_idx)?;
    let start = range.start.character as usize;
    let end = range.end.character as usize;
    let new_name = format!("_{name}");
    // Fast path: the diagnostic range exactly covers the identifier.
    if start < end && line_text.get(start..end) == Some(name) {
        return Some(vec![TextEdit {
            range: Range::new(
                lsp_types::Position::new(range.start.line, range.start.character),
                lsp_types::Position::new(range.end.line, range.end.character),
            ),
            new_text: new_name,
        }]);
    }
    // Zero-length range: insert `_` before the identifier.
    if start == end && start <= line_text.len() && line_text[start..].starts_with(name) {
        return Some(vec![TextEdit {
            range: Range::new(
                lsp_types::Position::new(range.start.line, range.start.character),
                lsp_types::Position::new(range.start.line, range.start.character),
            ),
            new_text: "_".to_string(),
        }]);
    }
    // Fallback: search the line for `name` as an identifier token
    // near the reported column (handles off-by-one / UTF-16 units).
    let search_from = start.min(line_text.len());
    if let Some(pos) = find_identifier_token(&line_text[search_from..], name) {
        let s = (search_from + pos) as u32;
        return Some(vec![TextEdit {
            range: Range::new(
                lsp_types::Position::new(range.start.line, s),
                lsp_types::Position::new(range.start.line, s + name.len() as u32),
            ),
            new_text: new_name,
        }]);
    }
    // Last resort: first token occurrence anywhere on the line.
    if let Some(pos) = find_identifier_token(line_text, name) {
        let s = pos as u32;
        return Some(vec![TextEdit {
            range: Range::new(
                lsp_types::Position::new(range.start.line, s),
                lsp_types::Position::new(range.start.line, s + name.len() as u32),
            ),
            new_text: new_name,
        }]);
    }
    None
}

fn find_identifier_token(haystack: &str, needle: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(rel) = haystack[from..].find(needle) {
        let pos = from + rel;
        let before = haystack[..pos].chars().next_back();
        let after = haystack[pos + needle.len()..].chars().next();
        let before_ok = before.is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_');
        let after_ok = after.is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_');
        if before_ok && after_ok {
            return Some(pos);
        }
        from = pos + needle.len();
    }
    None
}

pub fn module_used_as_alias(text: &str, module: &str) -> bool {
    let needle = format!("{module}::");
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with("use ") || t.starts_with("//") {
            continue;
        }
        let code = if let Some(rest) = t.strip_prefix("module ") {
            // strip module header up to first `{` — inline code after it still counts
            rest.find('{').map(|i| &rest[i + 1..]).unwrap_or("")
        } else {
            line
        };
        let code_part = code.split("//").next().unwrap_or("");
        if code_part.contains(&needle) {
            return true;
        }
    }
    false
}

pub fn insert_import_edit(lines: &[&str], import_path: &str) -> Option<TextEdit> {
    let (module_line, last_use_line, indent) = find_insert_line(lines)?;
    let at = last_use_line.unwrap_or(module_line) + 1;
    let indent_str = if last_use_line.is_some() {
        indent
    } else {
        "    ".to_string()
    };
    let range = Range::new(
        lsp_types::Position::new(at as u32, 0),
        lsp_types::Position::new(at as u32, 0),
    );
    Some(TextEdit {
        range,
        new_text: format!("{indent_str}use {import_path};\n"),
    })
}

fn find_insert_line(lines: &[&str]) -> Option<(usize, Option<usize>, String)> {
    let mut module_line = None;
    let mut last_use = None;
    let mut indent = String::new();
    for (i, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        if line.starts_with("module ")
            && (line.contains('{') || line.ends_with(';'))
            && module_line.is_none()
        {
            module_line = Some(i);
            continue;
        }
        if line.starts_with("use ") && line.ends_with(';') && module_line.is_some() {
            last_use = Some(i);
            indent = raw.chars().take_while(|c| c.is_whitespace()).collect();
        }
    }
    Some((module_line?, last_use, indent))
}

// ---------------------------------------------------------------------------
// Filesystem scan: own package + local deps + git dep caches.
// No hardcoded fallback tables — members/modules come from disk.

pub fn scan_package_members_and_modules(root: &Path) -> (MembersMap, ModulesMap) {
    let mut members = std::collections::HashMap::new();
    let mut modules = std::collections::HashMap::new();
    for sub in ["sources", "tests"] {
        let dir = root.join(sub);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e != "move").unwrap_or(true) {
                continue;
            }
            let Ok(txt) = std::fs::read_to_string(&p) else {
                continue;
            };
            let Some((addr, module)) = txt.lines().find_map(|line| {
                let t = line.trim();
                let rest = t.strip_prefix("module ")?;
                let head = rest.split(['{', ';']).next()?.trim();
                if let Some(idx) = head.find("::") {
                    Some((
                        head[..idx].trim().to_string(),
                        head[idx + 2..].trim().to_string(),
                    ))
                } else if !head.is_empty() {
                    Some((String::new(), head.to_string()))
                } else {
                    None
                }
            }) else {
                continue;
            };
            let addr = if addr.is_empty() {
                // 2024-style `module name;` — address comes from [package] name
                read_package_name(root).unwrap_or_default()
            } else {
                addr
            };
            modules
                .entry(module.clone())
                .or_insert((addr.clone(), module.clone()));
            for line in txt.lines() {
                for marker in [
                    "public entry fun ",
                    "public fun ",
                    "public struct ",
                    "public enum ",
                    "public const ",
                    "native fun ",
                    "struct ",
                    "enum ",
                    "const ",
                ] {
                    if let Some(idx) = line.find(marker) {
                        let rest = &line[idx + marker.len()..];
                        let name = rest
                            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                            .next()
                            .unwrap_or("");
                        if !name.is_empty() {
                            members
                                .entry(name.to_string())
                                .or_insert((addr.clone(), module.clone()));
                        }
                    }
                }
            }
        }
    }
    (members, modules)
}

pub fn scan_package_members(root: &Path) -> MembersMap {
    scan_package_members_and_modules(root).0
}

pub fn local_dep_dirs(root: &Path) -> Vec<std::path::PathBuf> {
    let Ok(text) = std::fs::read_to_string(root.join("Move.toml")) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut in_deps = false;
    for line in text.lines() {
        let t = line.trim_start_matches('\u{FEFF}').trim();
        if t.starts_with('[') {
            in_deps = t == "[dependencies]" || t == "[dev-dependencies]";
            continue;
        }
        if in_deps
            && let Some(idx) = t.find("local")
            && let Some(q1) = t[idx..].find('"')
        {
            let rest = &t[idx + q1 + 1..];
            if let Some(q2) = rest.find('"') {
                out.push(root.join(&rest[..q2]));
            }
        }
    }
    out
}

pub fn git_cache_dirs() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let home = crate::deps::move_home();
    for sub in [home.clone(), home.join("git")] {
        let Ok(entries) = std::fs::read_dir(&sub) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() && (p.join(".git").exists() || p.join("sources").exists()) {
                out.push(p);
            }
        }
    }
    out
}

pub fn scan_all_members_and_modules(root: &Path) -> (MembersMap, ModulesMap) {
    let mut members = std::collections::HashMap::new();
    let mut modules = std::collections::HashMap::new();
    let (m, mods) = scan_package_members_and_modules(root);
    members.extend(m);
    modules.extend(mods);
    for dep in local_dep_dirs(root) {
        let (m, mods) = scan_package_members_and_modules(&dep);
        for (k, v) in m {
            members.entry(k).or_insert(v);
        }
        for (k, v) in mods {
            modules.entry(k).or_insert(v);
        }
    }
    for cache in git_cache_dirs() {
        let (m, mods) = scan_package_members_and_modules(&cache);
        for (k, v) in m {
            members.entry(k).or_insert(v);
        }
        for (k, v) in mods {
            modules.entry(k).or_insert(v);
        }
        for sub in ["sources", "crates"] {
            let subdir = cache.join(sub);
            if subdir.is_dir() {
                let (m2, mods2) = scan_package_members_and_modules(&subdir);
                for (k, v) in m2 {
                    members.entry(k).or_insert(v);
                }
                for (k, v) in mods2 {
                    modules.entry(k).or_insert(v);
                }
            }
        }
    }
    (members, modules)
}

pub fn package_root(file: &Path) -> Option<std::path::PathBuf> {
    let mut dir = file.parent()?;
    loop {
        if dir.join("Move.toml").exists() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
}

fn read_package_name(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("Move.toml")).ok()?;
    let mut in_pkg = false;
    for line in text.lines() {
        let t = line.trim_start_matches('\u{FEFF}').trim();
        if t.starts_with('[') {
            in_pkg = t == "[package]";
            continue;
        }
        if in_pkg
            && let Some(rest) = t.strip_prefix("name")
            && let Some(q) = rest.find('"')
        {
            let rest2 = &rest[q + 1..];
            if let Some(end) = rest2.find('"') {
                return Some(rest2[..end].to_string());
            }
        }
    }
    None
}

// Formatting: use the in-process kanari-move-fmt library directly so
// `kanari move fmt` (CLI) and the LSP share exactly one implementation.

pub fn format_document(params: &lsp_types::DocumentFormattingParams, text: &str) -> Vec<TextEdit> {
    let _ = params;
    if text.trim().is_empty() {
        return Vec::new();
    }
    match kanari_move_fmt::format_source(text, &kanari_move_fmt::FormatOptions::default()) {
        Ok(out) if out != text => {
            let last_line = text.lines().count().saturating_sub(1) as u32;
            let last_col = text.lines().last().map(|l| l.len() as u32).unwrap_or(0);
            vec![TextEdit {
                range: Range::new(
                    lsp_types::Position::new(0, 0),
                    lsp_types::Position::new(last_line, last_col),
                ),
                new_text: out,
            }]
        }
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// CodeLens: "Run Tests" / "Run Test" above #[test] fns.

pub fn code_lenses(params: &lsp_types::CodeLensParams, text: &str) -> Vec<lsp_types::CodeLens> {
    let uri = params.text_document.uri.clone();
    let mut lenses = Vec::new();
    lenses.push(lsp_types::CodeLens {
        range: Range::new(
            lsp_types::Position::new(0, 0),
            lsp_types::Position::new(0, 0),
        ),
        command: Some(lsp_types::Command {
            title: "▶ Run Tests".to_string(),
            command: "move-kanari.runTestFile".to_string(),
            arguments: Some(vec![
                serde_json::to_value(&uri).unwrap_or(serde_json::Value::Null),
            ]),
        }),
        data: None,
    });
    for (i, line) in text.lines().enumerate() {
        let t = line.trim();
        if !(t.starts_with("#[test") || t.starts_with("#[ test")) {
            continue;
        }
        let mut name = None;
        if let Some(idx) = line.find("fun ") {
            let rest = &line[idx + 4..];
            name = Some(
                rest.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .next()
                    .unwrap_or("")
                    .to_string(),
            );
        } else {
            for l in text.lines().skip(i + 1).take(3) {
                if let Some(idx) = l.find("fun ") {
                    let rest = &l[idx + 4..];
                    name = Some(
                        rest.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                            .next()
                            .unwrap_or("")
                            .to_string(),
                    );
                    break;
                }
            }
        }
        if let Some(n) = name
            && !n.is_empty()
        {
            lenses.push(lsp_types::CodeLens {
                range: Range::new(
                    lsp_types::Position::new(i as u32, 0),
                    lsp_types::Position::new(i as u32, 0),
                ),
                command: Some(lsp_types::Command {
                    title: "▶ Run Test".to_string(),
                    command: "move-kanari.runTest".to_string(),
                    arguments: Some(vec![
                        serde_json::to_value(&uri).unwrap_or(serde_json::Value::Null),
                        serde_json::Value::String(n),
                    ]),
                }),
                data: None,
            });
        }
    }
    lenses
}

pub fn read_disk_text(uri: &Uri) -> String {
    uri_to_file_path(uri)
        .ok()
        .and_then(|p| std::fs::read_to_string(&p).ok())
        .unwrap_or_default()
}
