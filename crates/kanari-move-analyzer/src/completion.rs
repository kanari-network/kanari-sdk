//! Code completion for Move, in the spirit of rust-analyzer.
//!
//! The stock `move-analyzer` completion is *textual*: it offers every
//! identifier that appears anywhere in the file, which is noise (typing
//! `coin::` suggests `mint`, `recipient`, ... — names that have nothing to
//! do with `coin`). This module computes completions from the symbols the
//! `move-analyzer` library already builds (`Symbols` + `DefInfo`):
//!
//! * `mod::` — the members (structs / functions / constants) of that module,
//!   with their real signatures;
//! * `expr.` — the fields of the struct the expression has;
//! * inside a `use` — the module paths that can be imported;
//! * everywhere else — the members of the enclosing module, the names
//!   imported into scope, the local variables in scope, Move's keywords and
//!   a few snippets.
//!
//! Only the cursor context (what is being typed, where) comes from the
//! document text; everything else comes from the analyzer's own symbol
//! information.

use crate::code_action::{
    insert_import_edit, package_root, scan_all_members_and_modules, uri_to_file_path,
};
use lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, InlayHint, InlayHintKind, InlayHintLabel,
    InsertTextFormat, Position, TextEdit,
};
use move_analyzer::symbols::{DefInfo, DefLoc, Symbols};
use move_compiler::parser::keywords::{BUILTINS, CONTEXTUAL_KEYWORDS, KEYWORDS, PRIMITIVE_TYPES};
use std::path::Path;

/// Move's type abilities — not lexer keywords, but what one writes after
/// `has`.
const ABILITIES: [&str; 4] = ["copy", "drop", "store", "key"];

/// Snippets offered at statement level: (label, body).
const SNIPPETS: [(&str, &str); 5] = [
    ("fun", "fun ${1:name}(${2}) {\n\t$0\n}"),
    ("public fun", "public fun ${1:name}(${2}) {\n\t$0\n}"),
    ("struct", "struct ${1:Name} has ${2:drop} {\n\t$0\n}"),
    ("const", "const ${1:NAME}: ${2:u64} = ${3:0};"),
    (
        "init",
        "init(witness: ${1:WITNESS}, ctx: &mut TxContext) {\n\t$0\n}",
    ),
];

/// A completion candidate.
#[derive(Debug, Clone)]
struct Candidate {
    label: String,
    kind: CompletionItemKind,
    detail: Option<String>,
    /// Text inserted instead of the label (snippets, module paths).
    insert_text: Option<String>,
    /// Extra edits applied together with the completion (the `use` line an
    /// auto-import needs).
    additional_edits: Vec<TextEdit>,
    /// Lower sorts first (code before keywords before snippets).
    priority: u8,
}

impl Candidate {
    fn new(label: impl Into<String>, kind: CompletionItemKind) -> Self {
        Self {
            label: label.into(),
            kind,
            detail: None,
            insert_text: None,
            additional_edits: Vec::new(),
            priority: 0,
        }
    }
}

/// What the cursor is doing, as far as completion is concerned.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompletionContext {
    /// Identifier fragment the user has typed so far (may be empty).
    pub prefix: String,
    /// Module path qualifier, e.g. `coin`, when the cursor sits behind `::`.
    pub qualifier: Option<String>,
    /// Expression the cursor sits behind a `.` (field completion).
    pub dot_target: Option<String>,
    /// True when the cursor is inside a `use` statement.
    pub in_use: bool,
    /// Path of the module the cursor is in, e.g. `james::james`.
    pub module: Option<String>,
}

/// A name that is in scope, with the definition it refers to.
#[derive(Debug, Clone)]
struct ScopeEntry {
    /// Name as written in the file.
    alias: String,
    /// What the name refers to.
    info: DefInfo,
    /// Where the definition lives.
    def_loc: DefLoc,
    /// True when the definition is in the current file.
    is_local: bool,
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Text of `line` up to (but excluding) `character`, clamped to the line
/// length and counted in UTF-16 code units (what LSP uses).
fn line_prefix(line: &str, character: u32) -> String {
    let mut out = String::new();
    let mut units = 0u32;
    for c in line.chars() {
        if units >= character {
            break;
        }
        units += c.len_utf16() as u32;
        out.push(c);
    }
    out
}

impl CompletionContext {
    /// Parse the completion context out of the document text.
    pub fn parse(text: &str, position: Position) -> Self {
        let lines: Vec<&str> = text.lines().collect();
        let line = lines.get(position.line as usize).copied().unwrap_or("");
        let before = line_prefix(line, position.character);

        // The identifier fragment the user is currently typing.
        let prefix: String = before
            .chars()
            .rev()
            .take_while(|c| is_ident_char(*c))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        let head: String = before
            .chars()
            .take(before.chars().count() - prefix.chars().count())
            .collect();
        let head_trimmed = head.trim_end();

        // `mod::` — path completion.
        let mut qualifier = None;
        if head_trimmed.ends_with("::") {
            let path: String = head_trimmed
                .chars()
                .rev()
                .take_while(|c| is_ident_char(*c) || *c == ':')
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            let path = path.trim_end_matches(':').to_string();
            if !path.is_empty() {
                qualifier = Some(path);
            }
        }

        // `expr.` — field completion.
        let mut dot_target = None;
        if qualifier.is_none() && head_trimmed.ends_with('.') {
            let before_dot = &head_trimmed[..head_trimmed.len() - 1];
            let expr: String = before_dot
                .chars()
                .rev()
                .take_while(|c| is_ident_char(*c) || *c == ':')
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            let expr = expr.trim_end_matches(':').to_string();
            if !expr.is_empty() {
                dot_target = Some(expr);
            }
        }

        // Inside a `use` statement?
        let statement = line.trim_start();
        let in_use = statement.starts_with("use ") || statement == "use";

        // The module the cursor is in: the closest `module <path> {` above.
        let module = lines
            .iter()
            .take(position.line as usize + 1)
            .rev()
            .find_map(|l| parse_module_header(l));

        CompletionContext {
            prefix,
            qualifier,
            dot_target,
            in_use,
            module,
        }
    }
}

/// Extract the module path from a `module <path> {` header line.
pub fn parse_module_header(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("module ")?;
    let path = rest.split('{').next()?.trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

/// Relevance of a candidate for the prefix the user typed.
///
/// Deliberately conservative: a prefix must match the start of the name
/// (or be contained in it). A loose "subsequence" match would offer
/// `deny_remove` for the prefix `dro`, which is noise.
fn matches_prefix(name: &str, prefix: &str) -> Option<u8> {
    if prefix.is_empty() {
        return Some(2);
    }
    if name.starts_with(prefix) {
        return Some(0);
    }
    let lower_name = name.to_lowercase();
    let lower_prefix = prefix.to_lowercase();
    if lower_name.starts_with(&lower_prefix) {
        return Some(1);
    }
    if lower_name.contains(&lower_prefix) {
        return Some(2);
    }
    None
}

/// Everything the file refers to: its own members and the names it imports.
fn file_scope(text: &str, file: &Path, symbols: &Symbols) -> Vec<ScopeEntry> {
    let file_fhash = symbols
        .file_mods()
        .get(file)
        .and_then(|mods| mods.iter().next())
        .map(|m| m.fhash());
    let mut out = Vec::new();
    for (line, l) in text.lines().enumerate() {
        for u in symbols.line_uses(file, line as u32) {
            let Some(info) = symbols.def_info(&u.def_loc()) else {
                continue;
            };
            let def_loc = u.def_loc();
            out.push(ScopeEntry {
                alias: l
                    .chars()
                    .skip(u.col_start() as usize)
                    .take((u.col_end() - u.col_start()) as usize)
                    .collect(),
                info: info.clone(),
                def_loc,
                is_local: file_fhash == Some(def_loc.fhash()),
            });
        }
    }
    out
}

/// The name under which a scope entry is visible.
///
/// For modules the analyzer's column range is not reliable (it points into
/// the middle of the path), so the module path decides the name.
fn entry_name(entry: &ScopeEntry) -> String {
    match &entry.info {
        DefInfo::Module(path) => path.rsplit("::").next().unwrap_or(path).to_string(),
        _ => entry
            .alias
            .rsplit("::")
            .next()
            .unwrap_or(&entry.alias)
            .trim_start_matches(':')
            .to_string(),
    }
}

/// The members of the module declared in `file`.
fn module_members(symbols: &Symbols, file: &Path) -> Vec<(String, DefInfo)> {
    let Some(fhash) = symbols
        .file_mods()
        .get(file)
        .and_then(|mods| mods.iter().next())
        .map(|m| m.fhash())
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in 0..400 {
        // generous upper bound for a source file
        for u in symbols.line_uses(file, line) {
            if u.def_loc().fhash() != fhash {
                continue;
            }
            let Some(info) = symbols.def_info(&u.def_loc()) else {
                continue;
            };
            let name = match info {
                DefInfo::Function(_, _, name, ..) => name.as_str().to_string(),
                DefInfo::Struct(_, name, ..) => name.as_str().to_string(),
                DefInfo::Const(_, name, ..) => name.as_str().to_string(),
                _ => continue,
            };
            out.push((name, info.clone()));
        }
    }
    out
}

/// The fields of a struct definition, as `name` / `name: type`.
///
/// `DefInfo` renders the whole struct (including its fields) — the field
/// list is read back from that rendering, so no extra API is needed.
fn struct_fields(info: &DefInfo) -> Vec<(String, Option<String>)> {
    let rendered = info.to_string();
    let mut out = Vec::new();
    for line in rendered.lines() {
        let l = line.trim().trim_end_matches(',');
        let Some((name, ty)) = l.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() || name.contains(' ') {
            continue;
        }
        out.push((name.to_string(), Some(ty.trim().to_string())));
    }
    out
}

/// The base type name of a rendered type
/// (`&kanari_system::deny_list::DenyList` -> `DenyList`).
///
/// The analyzer renders local types fully qualified but parameter types
/// short, so the comparison has to look at the last segment.
fn base_type_name(ty: &str) -> String {
    let ty = ty
        .split('<')
        .next()
        .unwrap_or(ty)
        .trim()
        .trim_start_matches('&')
        .trim_start_matches("mut ")
        .trim();
    ty.rsplit("::").next().unwrap_or(ty).trim().to_string()
}

/// The type of a local, as rendered by `DefInfo::Local` (`let x: u64`).
fn local_type(info: &DefInfo) -> Option<String> {
    let rendered = info.to_string();
    let (_, ty) = rendered.rsplit_once(':')?;
    Some(ty.trim().to_string())
}

/// The type of the first parameter of a function, as rendered by
/// `DefInfo::Function` (`public fun f(cap: &mut Cap<T>): u64`).
fn first_param_type(info: &DefInfo) -> Option<String> {
    let rendered = info.to_string();
    let open = rendered.find('(')?;
    let close = rendered[open..].find(')')? + open;
    let first = rendered[open + 1..close].split(',').next()?.trim();
    let (_, ty) = first.split_once(':')?;
    Some(ty.trim().to_string())
}

/// The path typed so far inside a `use` statement.
fn typed_use_path(text: &str, position: Position) -> String {
    let Some(line) = text.lines().nth(position.line as usize) else {
        return String::new();
    };
    let before = line_prefix(line, position.character);
    let Some(rest) = before.trim_start().strip_prefix("use") else {
        return String::new();
    };
    rest.trim().trim_end_matches(':').to_string()
}

/// Keywords, contextual keywords, primitive types, abilities and builtins.
fn keywords() -> Vec<(String, CompletionItemKind)> {
    let mut out = Vec::new();
    for k in KEYWORDS.iter().chain(CONTEXTUAL_KEYWORDS.iter()) {
        out.push((k.to_string(), CompletionItemKind::KEYWORD));
    }
    for t in PRIMITIVE_TYPES.iter() {
        out.push((t.to_string(), CompletionItemKind::KEYWORD));
    }
    for a in ABILITIES.iter() {
        out.push((a.to_string(), CompletionItemKind::KEYWORD));
    }
    for b in BUILTINS.iter() {
        out.push((b.to_string(), CompletionItemKind::FUNCTION));
    }
    out
}

/// The kind of a completion item for a definition.
fn kind_of(info: &DefInfo) -> CompletionItemKind {
    match info {
        DefInfo::Function(..) => CompletionItemKind::FUNCTION,
        DefInfo::Struct(..) => CompletionItemKind::STRUCT,
        DefInfo::Const(..) => CompletionItemKind::CONSTANT,
        DefInfo::Module(_) => CompletionItemKind::MODULE,
        DefInfo::Field(..) => CompletionItemKind::FIELD,
        DefInfo::Local(..) => CompletionItemKind::VARIABLE,
        DefInfo::Type(_) => CompletionItemKind::TYPE_PARAMETER,
    }
}

/// Build the completion items for a request.
pub fn completion_items(
    text: &str,
    position: Position,
    file: Option<&Path>,
    symbols: &Symbols,
) -> Vec<CompletionItem> {
    let ctx = CompletionContext::parse(text, position);
    let Some(file) = file else {
        return Vec::new();
    };
    let scope = file_scope(text, file, symbols);
    let mut candidates: Vec<Candidate> = Vec::new();

    if ctx.in_use {
        // Inside `use ...` — the modules (and their items) that can be
        // imported.
        let typed = typed_use_path(text, position);
        let (members, modules) = package_root(file)
            .map(|root| scan_all_members_and_modules(&root))
            .unwrap_or_default();
        let mut paths: Vec<(&String, &(String, String))> = modules.iter().collect();
        paths.sort();
        for (module, (addr, _)) in paths {
            let full = if addr.is_empty() {
                (*module).clone()
            } else {
                format!("{addr}::{module}")
            };
            if !typed.is_empty() && !full.starts_with(&typed) && !module.starts_with(&typed) {
                continue;
            }
            let last = module.rsplit("::").next().unwrap_or(module).to_string();
            let mut c = Candidate::new(last, CompletionItemKind::MODULE);
            c.detail = Some(full.clone());
            c.insert_text = Some(full);
            candidates.push(c);
        }
        // Items of the module being typed: `use kanari_system::coin::` -> Coin
        let mut names: Vec<&String> = members.keys().collect();
        names.sort();
        for name in names {
            let (addr, module) = &members[name];
            let full = if addr.is_empty() {
                format!("{module}::{name}")
            } else {
                format!("{addr}::{module}::{name}")
            };
            if !typed.is_empty() && !full.starts_with(&typed) {
                continue;
            }
            let mut c = Candidate::new(name.clone(), CompletionItemKind::TEXT);
            c.detail = Some(full.clone());
            c.insert_text = Some(full);
            candidates.push(c);
        }
    } else if let Some(qualifier) = &ctx.qualifier {
        // `mod::` — the members of that module.
        if let Some(entry) = scope
            .iter()
            .find(|e| entry_name(e) == *qualifier && matches!(e.info, DefInfo::Module(_)))
        {
            let DefInfo::Module(path) = &entry.info else {
                unreachable!()
            };
            let target_fhash = entry.def_loc.fhash();
            let target_file = symbols
                .file_mods()
                .iter()
                .find(|(_, mods)| mods.iter().any(|m| m.fhash() == target_fhash))
                .map(|(p, _)| p.clone());
            if let Some(target) = target_file {
                for (name, info) in module_members(symbols, &target) {
                    let mut c = Candidate::new(name, kind_of(&info));
                    c.detail = Some(info.to_string());
                    candidates.push(c);
                }
            }
            let mut c = Candidate::new(qualifier.clone(), CompletionItemKind::MODULE);
            c.detail = Some(path.clone());
            candidates.push(c);
        }
    } else if let Some(target) = &ctx.dot_target {
        // `expr.` — the fields of the struct behind the expression and the
        // methods that take it (what Sui's IDE offers).
        let ty = scope
            .iter()
            .find(|e| e.alias == *target)
            .and_then(|e| local_type(&e.info))
            .unwrap_or_default();
        let base = {
            let base = base_type_name(&ty);
            if base.is_empty() {
                target.clone()
            } else {
                base
            }
        };
        for entry in &scope {
            let DefInfo::Struct(_, name, ..) = &entry.info else {
                continue;
            };
            if name.as_str() != base {
                continue;
            }
            for (field, field_ty) in struct_fields(&entry.info) {
                let mut c = Candidate::new(field.clone(), CompletionItemKind::FIELD);
                c.detail = Some(
                    field_ty
                        .map(|t| format!("{field}: {t}"))
                        .unwrap_or_else(|| field.clone()),
                );
                candidates.push(c);
            }
            // Methods: the functions of the struct's module that take the
            // struct as their first argument.
            let fhash = entry.def_loc.fhash();
            if let Some(file) = symbols
                .file_mods()
                .iter()
                .find(|(_, mods)| mods.iter().any(|m| m.fhash() == fhash))
                .map(|(p, _)| p.clone())
            {
                for (name, info) in module_members(symbols, &file) {
                    let takes = first_param_type(&info)
                        .map(|t| base_type_name(&t) == base)
                        .unwrap_or(false);
                    if !takes {
                        continue;
                    }
                    let mut c = Candidate::new(name, CompletionItemKind::METHOD);
                    c.detail = Some(info.to_string());
                    candidates.push(c);
                }
            }
        }
    } else {
        // Scope: the module's own members, the imported names, the locals in
        // scope, keywords and snippets.
        for entry in &scope {
            let name = if entry.is_local {
                match &entry.info {
                    DefInfo::Function(_, _, name, ..)
                    | DefInfo::Struct(_, name, ..)
                    | DefInfo::Const(_, name, ..) => name.as_str().to_string(),
                    _ => continue,
                }
            } else {
                entry_name(entry)
            };
            if name.is_empty() {
                continue;
            }
            let mut c = Candidate::new(name, kind_of(&entry.info));
            c.detail = Some(entry.info.to_string());
            candidates.push(c);
        }
        // Auto-import: a name that exists in another module of the package
        // but is not in scope yet — offer it together with the `use` line it
        // needs (what rust-analyzer does).
        if !ctx.prefix.is_empty() {
            let (members, _) = package_root(file)
                .map(|root| scan_all_members_and_modules(&root))
                .unwrap_or_default();
            let mut names: Vec<&String> = members.keys().collect();
            names.sort();
            let lines: Vec<&str> = text.lines().collect();
            for name in names {
                if matches_prefix(name, &ctx.prefix).is_none() {
                    continue;
                }
                if scope.iter().any(|e| e.alias == *name) {
                    continue;
                }
                let (addr, module) = &members[name];
                let path = if addr.is_empty() {
                    format!("{module}::{name}")
                } else {
                    format!("{addr}::{module}::{name}")
                };
                let Some(edit) = insert_import_edit(&lines, &path) else {
                    continue;
                };
                let mut c = Candidate::new(name.clone(), CompletionItemKind::FUNCTION);
                c.detail = Some(path);
                c.additional_edits = vec![edit];
                c.priority = 1;
                candidates.push(c);
            }
        }
        for (k, kind) in keywords() {
            let mut c = Candidate::new(k, kind);
            c.priority = 2;
            candidates.push(c);
        }
        for (name, body) in SNIPPETS {
            let mut c = Candidate::new(name.to_string(), CompletionItemKind::SNIPPET);
            c.detail = Some(body.to_string());
            c.insert_text = Some(body.to_string());
            c.priority = 3;
            candidates.push(c);
        }
        // `struct X has key {` — offer the `id: UID` field, the way the Sui
        // IDE does.
        let line = text.lines().nth(position.line as usize).unwrap_or("");
        if line.trim_end().ends_with('{') && line.contains("has key") {
            let mut c = Candidate::new("id: UID".to_string(), CompletionItemKind::SNIPPET);
            c.detail = Some("id: UID,\n$1".to_string());
            c.insert_text = Some("id: UID,\n$1".to_string());
            candidates.push(c);
        }
    }

    let mut items: Vec<(u8, u8, CompletionItem)> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for c in candidates {
        if seen.contains(&c.label) {
            continue;
        }
        seen.push(c.label.clone());
        let Some(score) = matches_prefix(&c.label, &ctx.prefix) else {
            continue;
        };
        // Snippets carry placeholders (`${1:name}`) — without the snippet
        // format the editor would insert them verbatim.
        let insert_text_format = match (c.kind, &c.insert_text) {
            (CompletionItemKind::SNIPPET, Some(_)) => Some(InsertTextFormat::SNIPPET),
            _ => None,
        };
        items.push((
            score,
            c.priority,
            CompletionItem {
                label: c.label,
                kind: Some(c.kind),
                detail: c.detail,
                documentation: None::<Documentation>,
                insert_text: c.insert_text,
                insert_text_format,
                additional_text_edits: (!c.additional_edits.is_empty())
                    .then_some(c.additional_edits),
                ..Default::default()
            },
        ));
    }
    items.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.label.cmp(&b.2.label))
    });
    items.into_iter().map(|(_, _, item)| item).collect()
}

/// Type hints for the locals of a file (`let x = 1` -> `let x: u64`).
///
/// The types come from the analyzer's own symbol information
/// (`DefInfo::Local`), so they are the compiler's types, not guesses.
pub fn inlay_hints(text: &str, file: &Path, symbols: &Symbols) -> Vec<InlayHint> {
    let mut out = Vec::new();
    for entry in file_scope(text, file, symbols) {
        let DefInfo::Local(name, _, is_decl, _) = &entry.info else {
            continue;
        };
        let Some(ty) = local_type(&entry.info) else {
            continue;
        };
        if ty.is_empty() {
            continue;
        }
        let start = entry.def_loc.start();
        // `let x` — the name starts after "let ".
        let offset = if *is_decl { 4 } else { 0 };
        out.push(InlayHint {
            position: Position::new(
                start.line,
                start.character + offset + name.as_str().chars().count() as u32,
            ),
            label: InlayHintLabel::String(format!(": {ty}")),
            kind: Some(InlayHintKind::TYPE),
            padding_left: Some(false),
            padding_right: None,
            data: None,
            text_edits: None,
            tooltip: None,
        });
    }
    out
}

/// Path of the document a request refers to.
pub fn uri_path(uri: &lsp_types::Uri) -> Option<std::path::PathBuf> {
    uri_to_file_path(uri).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cursor right after `needle` on the given line.
    fn pos_after(text: &str, line: u32, needle: &str) -> Position {
        let l = text.lines().nth(line as usize).unwrap();
        let idx = (l.find(needle).unwrap() + needle.chars().count()) as u32;
        Position::new(line, idx)
    }

    #[test]
    fn parses_plain_prefix() {
        let text = "module a::b {\n    fun mint() {}\n}\n";
        let ctx = CompletionContext::parse(text, pos_after(text, 1, "mint"));
        assert_eq!(ctx.prefix, "mint");
        assert_eq!(ctx.qualifier, None);
        assert_eq!(ctx.module.as_deref(), Some("a::b"));
    }

    #[test]
    fn parses_qualifier() {
        let text = "module a::b {\n    fun f() { let x = coin::mi; }\n}\n";
        let ctx = CompletionContext::parse(text, pos_after(text, 1, "coin::mi"));
        assert_eq!(ctx.prefix, "mi");
        assert_eq!(ctx.qualifier.as_deref(), Some("coin"));
    }

    #[test]
    fn parses_dot_target() {
        let text = "module a::b {\n    fun f() { let c = coin; c.fie; }\n}\n";
        let ctx = CompletionContext::parse(text, pos_after(text, 1, "c.fie"));
        assert_eq!(ctx.prefix, "fie");
        assert_eq!(ctx.dot_target.as_deref(), Some("c"));
    }

    #[test]
    fn prefix_matching_is_not_fuzzy_noisy() {
        // `dro` must not offer `deny_remove` (subsequence match) but must
        // offer `drop` and names that contain the prefix.
        assert_eq!(matches_prefix("drop", "dro"), Some(0));
        assert_eq!(matches_prefix("deny_remove", "dro"), None);
        assert_eq!(matches_prefix("deny_remove", "remove"), Some(2));
        assert_eq!(matches_prefix("DenyList", "deny"), Some(1));
        assert_eq!(matches_prefix("anything", ""), Some(2));
    }

    #[test]
    fn abilities_are_offered() {
        let items = keywords();
        let labels: Vec<&str> = items.iter().map(|(n, _)| n.as_str()).collect();
        for a in ABILITIES.iter() {
            assert!(labels.contains(a), "{labels:?}");
        }
    }

    #[test]
    fn base_type_name_normalizes_qualified_types() {
        // The analyzer renders local types fully qualified but parameter
        // types short — both have to end up as the same name.
        assert_eq!(
            base_type_name("&kanari_system::deny_list::DenyList"),
            "DenyList"
        );
        assert_eq!(base_type_name("&mut Coin<JAMES>"), "Coin");
        assert_eq!(base_type_name("u64"), "u64");
        assert_eq!(base_type_name("&mut vector<u8>"), "vector");
    }
}
