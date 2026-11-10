// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Move source audit detectors.
//!
//! Walks the tree-sitter parse tree for a Move source file and emits
//! `AuditFinding`s for common security issues.

use crate::finding::{AuditFinding, RuleId, Severity};
use std::path::Path;
use tree_sitter::{Node, Parser};
use tree_sitter_move;

/// Run every detector over `source` and return findings.
pub fn detect_all(source: &str, file: &str) -> Vec<AuditFinding> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_move::language())
        .expect("failed to load Move grammar");
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return vec![],
    };
    let root = tree.root_node();

    let mut findings: Vec<AuditFinding> = Vec::new();

    findings.extend(detect_entry_without_signer(source, file, root));
    findings.extend(detect_borrow_global_without_exists(source, file, root));
    findings.extend(detect_hardcoded_address(source, file, root));
    findings.extend(detect_external_call_in_entry(source, file, root));
    findings.extend(detect_unused_variables(source, file, root));
    findings.extend(detect_missing_doc(source, file, root));

    findings
}

// Helpers ------------------------------------------------------------------

fn node_text<'a>(node: Node<'a>, source: &'a str) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

fn line_col(node: Node) -> (usize, usize) {
    let pos = node.start_position();
    (pos.row + 1, pos.column + 1)
}

fn child_by_field<'a>(node: Node<'a>, field: &str) -> Option<Node<'a>> {
    node.child_by_field_name(field)
}

/// Return the first named child with the given grammar kind.
fn child_by_name<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    for i in 0..node.named_child_count() {
        let c = node.named_child(i as u32).unwrap();
        if c.kind() == kind {
            return Some(c);
        }
    }
    None
}

fn descendants_by_kind<'a>(node: Node<'a>, kind: &str, out: &mut Vec<Node<'a>>) {
    if node.kind() == kind {
        out.push(node);
    }
    for i in 0..node.named_child_count() {
        if let Some(c) = node.named_child(i as u32) {
            descendants_by_kind(c, kind, out);
        }
    }
}

/// Return the modifier texts (`"public"`, `"entry"`, `"native"`) on a function definition.
fn func_modifiers<'a>(func: Node<'a>, source: &'a str) -> Vec<&'a str> {
    let mut mods = Vec::new();
    for i in 0..func.named_child_count() {
        let c = func.named_child(i as u32).unwrap();
        if c.kind() == "modifier" {
            mods.push(node_text(c, source));
        }
    }
    mods
}

/// Return the type text of the first function parameter, if any.
fn func_has_authenticated_param<'a>(func: Node<'a>, source: &'a str) -> bool {
    for i in 0..func.named_child_count() {
        let Some(params) = func.named_child(i as u32) else {
            continue;
        };
        if params.kind() != "function_parameters" {
            continue;
        }
        for j in 0..params.named_child_count() {
            let Some(param) = params.named_child(j as u32) else {
                continue;
            };
            if param.kind() != "function_parameter" && param.kind() != "mut_function_parameter" {
                continue;
            }
            let actual_param = if param.kind() == "mut_function_parameter" {
                param.named_child(0).unwrap_or(param)
            } else {
                param
            };
            if let Some(ty) = child_by_field(actual_param, "type") {
                let t = node_text(ty, source);
                if t == "signer" || t.contains("TxContext") {
                    return true;
                }
            }
        }
    }
    false
}

/// Return the function name identifier text.
fn func_name_from_def<'a>(func: Node<'a>, source: &'a str) -> Option<&'a str> {
    for i in 0..func.named_child_count() {
        let c = func.named_child(i as u32).unwrap();
        if c.kind() == "function_identifier" {
            return Some(node_text(c, source));
        }
    }
    None
}

/// Return true if the call expression invokes `borrow_global` (possibly qualified).
fn is_borrow_global<'a>(call: Node<'a>, source: &'a str) -> bool {
    let Some(name_expr) = child_by_name(call, "name_expression") else {
        return false;
    };
    node_text(name_expr, source).starts_with("borrow_global")
}

/// Find the nearest enclosing `function_definition` ancestor of `node`.
fn enclosing_function<'a>(node: Node<'a>) -> Option<Node<'a>> {
    let mut cur = node;
    while let Some(parent) = cur.parent() {
        if parent.kind() == "function_definition" {
            return Some(parent);
        }
        cur = parent;
    }
    None
}

/// Return true if any ancestor of `node` (including itself) has the given kind.
fn has_ancestor_kind(node: Node, kind: &str) -> bool {
    let mut cur = node;
    while let Some(parent) = cur.parent() {
        if parent.kind() == kind {
            return true;
        }
        cur = parent;
    }
    false
}

// Detectors ----------------------------------------------------------------

fn detect_entry_without_signer(source: &str, file: &str, root: Node) -> Vec<AuditFinding> {
    let mut out = Vec::new();
    let mut funcs = Vec::new();
    descendants_by_kind(root, "function_definition", &mut funcs);

    for func in funcs {
        let mods = func_modifiers(func, source);
        if !mods.contains(&"entry") {
            continue;
        }
        let is_authenticated = func_has_authenticated_param(func, source);
        if !is_authenticated {
            let (line, col) = line_col(func);
            let name = func_name_from_def(func, source).unwrap_or("<unknown>");
            out.push(AuditFinding::new(
                RuleId::PublicEntryWithoutSigner,
                Severity::High,
                format!("entry function `{name}` lacks a `signer` parameter"),
                "Entry functions must authenticate the caller with a `signer` \
                 as the first parameter. Without it the function cannot be \
                 authorized.",
                file,
                line,
                col,
                Some(
                    node_text(func, source)
                        .lines()
                        .next()
                        .unwrap_or("")
                        .to_string(),
                ),
            ));
        }
    }
    out
}

fn detect_borrow_global_without_exists(source: &str, file: &str, root: Node) -> Vec<AuditFinding> {
    let mut out = Vec::new();
    let mut calls = Vec::new();
    descendants_by_kind(root, "call_expression", &mut calls);

    for call in calls {
        if !is_borrow_global(call, source) {
            continue;
        }
        let Some(func) = enclosing_function(call) else {
            continue;
        };
        let before = &source[func.start_byte()..call.start_byte()];
        // `exists` is often wrapped in `assert!(...)` (a macro), so a simple
        // substring check on the function text is the most robust signal.
        if !before.contains("exists") {
            let (line, col) = line_col(call);
            out.push(AuditFinding::new(
                RuleId::BorrowGlobalWithoutCheck,
                Severity::High,
                "borrow_global without an existence check",
                "borrow_global aborts if the resource does not exist. Check \
                 `exists<T>(addr)` before calling borrow_global, or handle the \
                 abort.",
                file,
                line,
                col,
                Some(
                    source[call.start_byte()..call.end_byte()]
                        .lines()
                        .next()
                        .unwrap_or("")
                        .to_string(),
                ),
            ));
        }
    }
    out
}

fn detect_hardcoded_address(source: &str, file: &str, root: Node) -> Vec<AuditFinding> {
    let mut out = Vec::new();
    let mut addrs = Vec::new();
    descendants_by_kind(root, "num_literal", &mut addrs);

    for addr in addrs {
        let text = node_text(addr, source);
        // Only flag hex address literals; skip module-address declarations
        // (whose ancestor is `module_identity`) and the zero/one addresses.
        if !text.starts_with("0x")
            || text == "0x0"
            || text == "0x1"
            || has_ancestor_kind(addr, "module_identity")
        {
            continue;
        }
        let (line, col) = line_col(addr);
        out.push(AuditFinding::new(
            RuleId::HardcodedAddress,
            Severity::Medium,
            "hardcoded address literal",
            format!(
                "Hardcoded address `{text}` couples the module to a specific \
                 account. Consider using a named address or a constant."
            ),
            file,
            line,
            col,
            Some(text.to_string()),
        ));
    }
    out
}

fn detect_external_call_in_entry(source: &str, file: &str, root: Node) -> Vec<AuditFinding> {
    let mut out = Vec::new();
    let mut funcs = Vec::new();
    descendants_by_kind(root, "function_definition", &mut funcs);

    for func in funcs {
        let mods = func_modifiers(func, source);
        if !mods.contains(&"entry") {
            continue;
        }
        let mut external: Option<Node> = None;
        find_external_call(func, source, &mut external);
        if let Some(call) = external {
            let (line, col) = line_col(call);
            let name = func_name_from_def(func, source).unwrap_or("<unknown>");
            out.push(AuditFinding::new(
                RuleId::Reentrancy,
                Severity::High,
                format!("entry function `{name}` makes an external call"),
                "Entry functions that call into other modules before state \
                 changes can be reentrancy targets. Perform state updates first, \
                 or use reentrancy guards.",
                file,
                line,
                col,
                Some(
                    source[call.start_byte()..call.end_byte()]
                        .lines()
                        .next()
                        .unwrap_or("")
                        .to_string(),
                ),
            ));
        }
    }
    out
}

/// Find a call expression inside `node` whose target is qualified with `::`.
fn find_external_call<'a>(node: Node<'a>, source: &'a str, found: &mut Option<Node<'a>>) {
    if found.is_some() {
        return;
    }
    if node.kind() == "call_expression" {
        let Some(name_expr) = child_by_name(node, "name_expression") else {
            return;
        };
        let text = node_text(name_expr, source);
        // `signer::address_of` / `signer::borrow_address` only read the signer
        // address — they are not cross-module state-changing calls, so they do
        // Standard library / built-in modules are not cross-module
        // reentrancy risks.
        let module = text.split("::").next().unwrap_or(text);
        let is_stdlib = matches!(
            module,
            "std"
                | "coin"
                | "vector"
                | "table"
                | "balance"
                | "object"
                | "signer"
                | "math"
                | "event"
                | "transfer"
                | "hash"
                | "type_name"
        );
        if text.contains("::") && !is_stdlib {
            let last_seg = text.split("::").last().unwrap_or(text);
            let read_only = matches!(
                last_seg,
                "address_of" | "borrow_address" | "length" | "contains" | "value" | "borrow"
            );
            if !read_only {
                // Only flag calls that mutate state (carry an `&mut` argument);
                // pure reads do not open a reentrancy window.
                let mutates = match child_by_field(node, "args") {
                    Some(a) => node_text(a, source).contains("&mut"),
                    None => false,
                };
                if mutates {
                    *found = Some(node);
                    return;
                }
            }
        }
    }
    for i in 0..node.named_child_count() {
        if let Some(c) = node.named_child(i as u32) {
            find_external_call(c, source, found);
        }
    }
}

fn detect_unused_variables(source: &str, file: &str, root: Node) -> Vec<AuditFinding> {
    let mut out = Vec::new();
    let mut decls = Vec::new();
    descendants_by_kind(root, "variable_declaration", &mut decls);

    for decl in decls {
        let Some(name_node) = child_by_field(decl, "name") else {
            continue;
        };
        let name = node_text(name_node, source);
        if name == "_" || name.starts_with('_') {
            continue;
        }
        let decl_byte = decl.start_byte();
        let mut referenced = false;
        collect_identifiers_after(root, decl_byte, name, source, &mut referenced);
        if !referenced {
            let (line, col) = line_col(decl);
            out.push(AuditFinding::new(
                RuleId::UnusedVariable,
                Severity::Low,
                format!("unused variable `{name}`"),
                format!(
                    "Variable `{name}` is declared but never used. Remove it \
                 or prefix with `_` to silence."
                ),
                file,
                line,
                col,
                Some(name.to_string()),
            ));
        }
    }
    out
}

/// Collect identifiers appearing after `start_byte`; set `found` when `name` is seen.
fn collect_identifiers_after(
    node: Node,
    start_byte: usize,
    name: &str,
    source: &str,
    found: &mut bool,
) {
    if *found {
        return;
    }
    if node.kind() == "identifier"
        && node.start_byte() > start_byte
        && node_text(node, source) == name
    {
        *found = true;
        return;
    }
    for i in 0..node.named_child_count() {
        if let Some(c) = node.named_child(i as u32) {
            collect_identifiers_after(c, start_byte, name, source, found);
        }
    }
}

fn detect_missing_doc(source: &str, file: &str, root: Node) -> Vec<AuditFinding> {
    let mut out = Vec::new();
    let mut funcs = Vec::new();
    descendants_by_kind(root, "function_definition", &mut funcs);

    for func in funcs {
        let mods = func_modifiers(func, source);
        if !mods.contains(&"public") {
            continue;
        }
        let (line, col) = line_col(func);
        let before = &source[..func.start_byte()];
        let has_doc = before.lines().rev().take(3).any(|l| {
            let t = l.trim();
            t.starts_with("/**") || t.starts_with("///") || t.starts_with("//!")
        });
        if !has_doc {
            let name = func_name_from_def(func, source).unwrap_or("<unknown>");
            out.push(AuditFinding::new(
                RuleId::MissingDoc,
                Severity::Info,
                format!("public function `{name}` is undocumented"),
                "Public functions should have a doc comment describing purpose, \
                 parameters, and abort conditions.",
                file,
                line,
                col,
                Some(
                    node_text(func, source)
                        .lines()
                        .next()
                        .unwrap_or("")
                        .to_string(),
                ),
            ));
        }
    }
    out
}

// Public API ---------------------------------------------------------------

/// Parse `source` (a single Move file) and run all detectors.
pub fn audit_source(source: &str, file: &str) -> Vec<AuditFinding> {
    detect_all(source, file)
}

/// Parse every `.move` file under `dir` (recursively) and run all detectors.
pub fn audit_directory(dir: &Path) -> Vec<AuditFinding> {
    let mut findings = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "move")
                && let Ok(source) = std::fs::read_to_string(&path)
            {
                let file = path.to_string_lossy().to_string();
                findings.extend(detect_all(&source, &file));
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
module 0x1::Example {
    entry fun bad_entry(x: u64) { }
    entry fun good_entry(signer: signer) { }
    public fun uses_borrow(addr: address) {
        let c = borrow_global<Coin>(addr);
    }
    public fun safe_borrow(addr: address) {
        assert!(exists<Coin>(addr), 0);
        let c = borrow_global<Coin>(addr);
    }
    public fun hardcoded() {
        let a = 0x42;
    }
    public entry fun external_call(signer: signer) {
        let _ = 0x2::Other::foo(&mut ctx);
    }
    public fun undocumented() { }
    public fun documented() {
        // docs
    }
}
"#;

    #[test]
    fn detects_missing_signer() {
        let findings = detect_all(SAMPLE, "test.move");
        assert!(
            findings
                .iter()
                .any(|f| f.rule == RuleId::PublicEntryWithoutSigner)
        );
    }

    #[test]
    fn does_not_flag_signer_present() {
        let src = r#"
module 0x1::M {
    public entry fun place_bid<Base, Quote>(signer: signer, book: &mut OrderBook<Base, Quote>) { }
    entry fun good_entry(signer: signer) { }
    public entry fun no_type_params(signer: signer) { }
}
"#;
        let findings = detect_all(src, "test.move");
        assert!(
            findings
                .iter()
                .all(|f| f.rule != RuleId::PublicEntryWithoutSigner),
            "unexpected findings: {:?}",
            findings
        );
    }

    #[test]
    fn does_not_flag_signer_address_of_as_reentrancy() {
        let src = r#"
module 0x1::M {
    entry fun foo(signer: signer) {
        let addr = signer::address_of(&signer);
    }
}
"#;
        let findings = detect_all(src, "test.move");
        assert!(
            findings.iter().all(|f| f.rule != RuleId::Reentrancy),
            "unexpected findings: {:?}",
            findings
        );
    }

    #[test]
    fn does_not_flag_object_model_entry_function() {
        let src = r#"
module 0x1::M {
    entry fun foo(ctx: &mut TxContext) { }
}
"#;
        let findings = detect_all(src, "test.move");
        assert!(
            findings
                .iter()
                .all(|f| f.rule != RuleId::PublicEntryWithoutSigner),
            "unexpected findings: {:?}",
            findings
        );
    }

    #[test]
    fn does_not_flag_object_model_entry_function_with_params() {
        let src = r#"
module 0x1::M {
    entry fun cancel_order<Base, Quote>(
        book: &mut OrderBook<Base, Quote>,
        order_id: u64,
        ctx: &mut TxContext
    ) { }
}
"#;
        let findings = detect_all(src, "test.move");
        assert!(
            findings
                .iter()
                .all(|f| f.rule != RuleId::PublicEntryWithoutSigner),
            "unexpected findings: {:?}",
            findings
        );
    }

    #[test]
    fn does_not_flag_read_only_external_call() {
        let src = r#"
module 0x1::M {
    entry fun foo(signer: signer, book: &mut OrderBook<Base, Quote>) {
        let _n = vector::length(&book.bids);
        let _a = table::contains(&registry.admins, addr);
        let _v = coin::value(&coin);
        let _s = signer::address_of(&signer);
    }
}
"#;
        let findings = detect_all(src, "test.move");
        assert!(
            findings.iter().all(|f| f.rule != RuleId::Reentrancy),
            "unexpected findings: {:?}",
            findings
        );
    }

    #[test]
    fn flags_state_changing_external_call() {
        let src = r#"
module 0x1::M {
    entry fun foo(signer: signer, registry: &mut Registry) {
        0x2::External::mutate(&mut registry.state);
    }
}
"#;
        let findings = detect_all(src, "test.move");
        assert!(
            findings.iter().any(|f| f.rule == RuleId::Reentrancy),
            "expected a reentrancy finding for state-changing call"
        );
    }

    #[test]
    fn does_not_flag_immutable_borrow() {
        let src = r#"
module 0x1::M {
    entry fun foo(signer: signer, book: &mut OrderBook<Base, Quote>) {
        let _b = vector::borrow(&book.bids, 0);
        let _t = table::borrow(&registry.admins, addr);
    }
}
"#;
        let findings = detect_all(src, "test.move");
        assert!(
            findings.iter().all(|f| f.rule != RuleId::Reentrancy),
            "unexpected findings: {:?}",
            findings
        );
    }

    #[test]
    fn detects_borrow_global_without_exists() {
        let findings = detect_all(SAMPLE, "test.move");
        assert!(
            findings
                .iter()
                .any(|f| f.rule == RuleId::BorrowGlobalWithoutCheck)
        );
    }

    #[test]
    fn detects_hardcoded_address() {
        let findings = detect_all(SAMPLE, "test.move");
        assert!(findings.iter().any(|f| f.rule == RuleId::HardcodedAddress));
    }

    #[test]
    fn detects_external_call_in_entry() {
        let findings = detect_all(SAMPLE, "test.move");
        assert!(findings.iter().any(|f| f.rule == RuleId::Reentrancy));
    }

    #[test]
    fn detects_missing_doc() {
        let findings = detect_all(SAMPLE, "test.move");
        assert!(findings.iter().any(|f| f.rule == RuleId::MissingDoc));
    }

    #[test]
    fn does_not_flag_good_code() {
        let good = r#"
module 0x1::Good {
    entry fun ok(signer: signer) { }
    public fun checked(addr: address) {
        assert!(exists<Coin>(addr), 0);
        let c = borrow_global<Coin>(addr);
    }
}
"#;
        let findings = detect_all(good, "test.move");
        let high = findings
            .iter()
            .filter(|f| f.severity >= Severity::High)
            .count();
        assert_eq!(high, 0);
    }
}
