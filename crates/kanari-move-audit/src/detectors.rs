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

/// Modules considered part of the standard library / framework (not external).
const STDLIB_MODULES: &[&str] = &[
    "std",
    "coin",
    "vector",
    "table",
    "balance",
    "object",
    "option",
    "signer",
    "math",
    "event",
    "transfer",
    "hash",
    "type_name",
    "tx_context",
    "borrow",
];

/// Method names that mutate state.
const STATE_CHANGING_METHODS: &[&str] = &[
    "save_object",
    "freeze_object",
    "share_object",
    "merge",
    "burn",
    "mint",
    "increase_supply",
    "decrease_supply",
    "push_back",
    "pop_back",
    "swap_remove",
    "insert",
    "remove",
    "set",
    "unset",
];

/// Run every detector over `source` and return findings.
pub fn detect_all(source: &str, file: &str) -> Vec<AuditFinding> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_move::language()).expect("failed to load Move grammar");
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return vec![],
    };
    let root = tree.root_node();

    let mut findings: Vec<AuditFinding> = Vec::new();

    findings.extend(detect_entry_without_auth(source, file, root));
    findings.extend(detect_public_entry_without_signer(source, file, root));
    findings.extend(detect_borrow_global_without_exists(source, file, root));
    findings.extend(detect_hardcoded_address(source, file, root));
    findings.extend(detect_external_call_in_entry(source, file, root));
    findings.extend(detect_state_change_before_external_call(source, file, root));
    findings.extend(detect_missing_access_control(source, file, root));
    findings.extend(detect_unsafe_public_transfer(source, file, root));
    findings.extend(detect_unused_variables(source, file, root));
    findings.extend(detect_missing_doc(source, file, root));
    findings.extend(detect_capability_leak(source, file, root));
    findings.extend(detect_integer_overflow_risk(source, file, root));
    findings.extend(detect_share_object_without_auth(source, file, root));
    findings.extend(detect_division_by_zero_risk(source, file, root));
    findings.extend(detect_unchecked_vector_borrow(source, file, root));

    findings
}

// Helpers ------------------------------------------------------------------

/// Whether `file` is a test file (should skip some checks).
fn is_test_file(file: &str) -> bool {
    file.contains("/tests/") || file.contains("\\tests\\")
}

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

/// Return true if the function has at least one owned (non-reference) parameter,
/// which implies the caller must own the value (implicit authentication).
fn has_owned_parameter(func: Node, source: &str) -> bool {
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
            let actual = if param.kind() == "mut_function_parameter" {
                param.named_child(0).unwrap_or(param)
            } else {
                param
            };
            // If the parameter has no `&` in its type, it's owned.
            if let Some(ty) = child_by_field(actual, "type") {
                let t = node_text(ty, source);
                if !t.starts_with('&') {
                    return true;
                }
            }
        }
    }
    false
}

/// Return true if the function has an authenticated parameter (`signer`,
/// `&mut TxContext`, or a capability type such as `TreasuryCap`).
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
                // signer model OR object model (TxContext) OR capability-based auth
                if t == "signer"
                    || t.contains("TxContext")
                    || t.contains("TreasuryCap")
                    || t.contains("KioskOwnerCap")
                    || t.contains("TransferPolicy")
                {
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

// Detector: entry function without authentication ---------------------------

fn detect_entry_without_auth(source: &str, file: &str, root: Node) -> Vec<AuditFinding> {
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
                RuleId::EntryWithoutAuth,
                Severity::High,
                format!("entry function `{name}` lacks caller authentication"),
                "Entry functions must authenticate the caller via a `signer` \
                 parameter, a `&mut TxContext` parameter (object model), or a \
                 capability parameter such as `TreasuryCap`/`KioskOwnerCap`.",
                file,
                line,
                col,
                Some(node_text(func, source).lines().next().unwrap_or("").to_string()),
            ));
        }
    }
    out
}

// Detector: borrow_global without exists check ------------------------------

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

// Detector: hardcoded address literal ---------------------------------------

fn detect_hardcoded_address(source: &str, file: &str, root: Node) -> Vec<AuditFinding> {
    let mut out = Vec::new();
    let mut addrs = Vec::new();
    descendants_by_kind(root, "num_literal", &mut addrs);

    for addr in addrs {
        let text = node_text(addr, source);
        // Only flag hex address literals
        if !text.starts_with("0x") {
            continue;
        }
        let hex = &text[2..];
        // Skip bit masks and small constants (hex length <= 4 means value <= 0xFFFF)
        if hex.len() <= 4 {
            continue;
        }
        // Skip canonical error codes (5 hex digits starting with 0x1-0x4):
        // e.g. 0x10000, 0x20000, 0x40000 — these are error code constants,
        // not account addresses.
        if hex.len() == 5
            && (hex.starts_with('1')
                || hex.starts_with('2')
                || hex.starts_with('3')
                || hex.starts_with('4'))
        {
            continue;
        }
        // Skip test files - test constants are not real addresses
        if is_test_file(file) {
            continue;
        }
        // Skip if the literal appears inside a `const` declaration (compile-time
        // constants are not runtime addresses).
        let line_start = source[..addr.start_byte()].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_end = source[addr.start_byte()..]
            .find('\n')
            .map(|i| addr.start_byte() + i)
            .unwrap_or(source.len());
        let enclosing_line = &source[line_start..line_end];
        if enclosing_line.trim_start().starts_with("const ") {
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

// Detector: external call in entry function ---------------------------------

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
                RuleId::ExternalCall,
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

/// Check whether an entry function mutates state before making an external call.
fn detect_state_change_before_external_call(
    source: &str,
    file: &str,
    root: Node,
) -> Vec<AuditFinding> {
    let mut out = Vec::new();
    let mut funcs = Vec::new();
    descendants_by_kind(root, "function_definition", &mut funcs);

    for func in funcs {
        let mods = func_modifiers(func, source);
        if !mods.contains(&"entry") {
            continue;
        }
        // Collect all external calls and state-mutating calls with their byte ranges.
        let mut external_calls: Vec<(Node, usize)> = Vec::new();
        let mut state_changes: Vec<(Node, usize)> = Vec::new();
        collect_state_changes_and_external_calls(
            func,
            source,
            &mut state_changes,
            &mut external_calls,
        );

        // Find the earliest external call.
        let Some(first_external) = external_calls.iter().min_by_key(|(_, byte)| *byte) else {
            continue;
        };
        let first_external_byte = first_external.1;

        // Check if any state change happens before the first external call.
        let has_state_change_before =
            state_changes.iter().any(|(_, byte)| *byte < first_external_byte);
        if has_state_change_before {
            let (line, col) = line_col(first_external.0);
            let name = func_name_from_def(func, source).unwrap_or("<unknown>");
            out.push(AuditFinding::new(
                RuleId::StateChangeBeforeExternalCall,
                Severity::High,
                format!("entry function `{name}` mutates state before an external call"),
                "State changes before an external call can create reentrancy \
                 windows. Perform all state updates after external calls, or \
                 use reentrancy guards.",
                file,
                line,
                col,
                Some(
                    source[first_external.0.start_byte()..first_external.0.end_byte()]
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

/// Collect state-changing calls and external calls from a function body.
fn collect_state_changes_and_external_calls<'a>(
    node: Node<'a>,
    source: &'a str,
    state_changes: &mut Vec<(Node<'a>, usize)>,
    external_calls: &mut Vec<(Node<'a>, usize)>,
) {
    if node.kind() == "call_expression" {
        let Some(name_expr) = child_by_name(node, "name_expression") else {
            return;
        };
        let text = node_text(name_expr, source);
        let module = text.split("::").next().unwrap_or(text);
        let is_stdlib = STDLIB_MODULES.contains(&module);
        if text.contains("::") && !is_stdlib {
            external_calls.push((node, node.start_byte()));
        } else {
            // Check for state-mutating stdlib calls.
            let last_seg = text.split("::").last().unwrap_or(text);
            let is_state_change = STATE_CHANGING_METHODS.contains(&last_seg);
            if is_state_change {
                state_changes.push((node, node.start_byte()));
            }
        }
    }
    for i in 0..node.named_child_count() {
        if let Some(c) = node.named_child(i as u32) {
            collect_state_changes_and_external_calls(c, source, state_changes, external_calls);
        }
    }
}

/// Check whether public (non-entry) functions that mutate state lack access control.
fn detect_missing_access_control(source: &str, file: &str, root: Node) -> Vec<AuditFinding> {
    let mut out = Vec::new();
    let mut funcs = Vec::new();
    descendants_by_kind(root, "function_definition", &mut funcs);

    for func in funcs {
        let mods = func_modifiers(func, source);
        // Only check public functions that are not entry functions.
        if !mods.contains(&"public") || mods.contains(&"entry") {
            continue;
        }
        // Skip functions with no state changes.
        if !has_state_changing_call(func, source) {
            continue;
        }
        // Only flag functions that touch global storage (move_to/move_from/
        // borrow_global). Pure utility functions operating on `&mut` parameters
        // (e.g. bcs::peel_*, vector helpers) are not access-control risks.
        let func_text = node_text(func, source);
        let touches_global_storage = func_text.contains("move_to")
            || func_text.contains("move_from")
            || func_text.contains("borrow_global");
        if !touches_global_storage {
            continue;
        }
        // Skip functions with authenticated parameters.
        if func_has_authenticated_param(func, source) {
            continue;
        }
        let (line, col) = line_col(func);
        let name = func_name_from_def(func, source).unwrap_or("<unknown>");
        out.push(AuditFinding::new(
            RuleId::MissingAccessControl,
            Severity::High,
            format!("public function `{name}` mutates state without access control"),
            "Public functions that mutate state should authenticate the caller \
             via `signer`, `&mut TxContext`, or a capability parameter.",
            file,
            line,
            col,
            Some(node_text(func, source).lines().next().unwrap_or("").to_string()),
        ));
    }
    out
}

/// Check whether `transfer::public_transfer` is called without access control.
fn detect_unsafe_public_transfer(source: &str, file: &str, root: Node) -> Vec<AuditFinding> {
    let mut out = Vec::new();
    let mut calls = Vec::new();
    descendants_by_kind(root, "call_expression", &mut calls);

    for call in calls {
        // Skip test files - test code is allowed to call public_transfer freely.
        if is_test_file(file) {
            continue;
        }
        let Some(name_expr) = child_by_name(call, "name_expression") else {
            continue;
        };
        let text = node_text(name_expr, source);
        if text != "transfer::public_transfer" {
            continue;
        }
        let Some(func) = enclosing_function(call) else {
            continue;
        };
        let func_name = func_name_from_def(func, source).unwrap_or("");
        if func_name.starts_with("test_") {
            continue;
        }
        // Skip if the enclosing function takes an owned (non-reference) parameter:
        // the caller must own the object to transfer it, which is implicit auth.
        let has_owned_param = has_owned_parameter(func, source);
        if has_owned_param {
            continue;
        }
        // Skip if the enclosing function has authentication.
        if func_has_authenticated_param(func, source) {
            continue;
        }
        let (line, col) = line_col(call);
        let func_name = func_name_from_def(func, source).unwrap_or("<unknown>");
        out.push(AuditFinding::new(
            RuleId::UnsafePublicTransfer,
            Severity::High,
            format!("function `{func_name}` calls `public_transfer` without access control"),
            "`public_transfer` moves assets to an arbitrary address. Ensure the \
             caller is authenticated before transferring funds.",
            file,
            line,
            col,
            Some(
                source[call.start_byte()..call.end_byte()].lines().next().unwrap_or("").to_string(),
            ),
        ));
    }
    out
}

/// Return true if the function contains a state-changing call.
fn has_state_changing_call(func: Node, source: &str) -> bool {
    let mut found = false;
    collect_state_changes_only(func, source, &mut found);
    found
}

fn collect_state_changes_only<'a>(node: Node<'a>, source: &'a str, found: &mut bool) {
    if *found {
        return;
    }
    if node.kind() == "call_expression" {
        let Some(name_expr) = child_by_name(node, "name_expression") else {
            return;
        };
        let text = node_text(name_expr, source);
        let last_seg = text.split("::").last().unwrap_or(text);
        let is_state_change = STATE_CHANGING_METHODS.contains(&last_seg);
        if is_state_change {
            *found = true;
        }
    }
    for i in 0..node.named_child_count() {
        if let Some(c) = node.named_child(i as u32) {
            collect_state_changes_only(c, source, found);
        }
    }
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
        let is_stdlib = STDLIB_MODULES.contains(&module);
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
            t.starts_with("/**")
                || t.starts_with("///")
                || t.starts_with("//!")
                || t.starts_with("//")
        });
        if func_name_from_def(func, source).unwrap_or("").contains("testing") {
            continue;
        }
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
                Some(node_text(func, source).lines().next().unwrap_or("").to_string()),
            ));
        }
    }
    out
}

// Detector: capability leak (public function returning a capability) ------------

fn detect_capability_leak(source: &str, file: &str, root: Node) -> Vec<AuditFinding> {
    let mut out = Vec::new();
    let mut funcs = Vec::new();
    descendants_by_kind(root, "function_definition", &mut funcs);

    const CAPABILITY_TYPES: &[&str] = &[
        "TreasuryCap",
        "KioskOwnerCap",
        "TransferPolicy",
        "AdminCap",
        "OwnerCap",
        "MintCap",
        "BurnCap",
        "PauseCap",
    ];

    for func in funcs {
        let mods = func_modifiers(func, source);
        if !mods.contains(&"public") {
            continue;
        }
        // Skip factory functions: those that accept a witness (`T: drop`) or a
        // `&mut TxContext` are the canonical way to mint capabilities and are
        // not leaks.
        let func_text = node_text(func, source);
        let has_witness = func_text.contains(": drop)")
            || func_text.contains(": drop,")
            || func_text.contains("TxContext");
        if has_witness {
            continue;
        }
        // Check return type for capability types.
        let mut returns_cap = false;
        if let Some(ret) = child_by_field(func, "return_type") {
            let text = node_text(ret, source);
            for cap in CAPABILITY_TYPES {
                if text.contains(cap) {
                    returns_cap = true;
                    break;
                }
            }
        }
        if !returns_cap {
            continue;
        }
        let (line, col) = line_col(func);
        let name = func_name_from_def(func, source).unwrap_or("<unknown>");
        out.push(AuditFinding::new(
            RuleId::CapabilityLeak,
            Severity::High,
            format!("public function `{name}` returns a capability"),
            "Returning capabilities from public functions can leak privileged \
             access. Capabilities should be created internally and passed to \
             trusted callers, never returned freely.",
            file,
            line,
            col,
            Some(node_text(func, source).lines().next().unwrap_or("").to_string()),
        ));
    }
    out
}

// Detector: integer overflow risk (arithmetic without assert) ------------------

fn detect_integer_overflow_risk(source: &str, file: &str, root: Node) -> Vec<AuditFinding> {
    // Skip test files - test arithmetic is not a production concern.
    if is_test_file(file) {
        return Vec::new();
    }
    // Skip math/hex/crypto/royalty modules - their functions are inherently arithmetic.
    if file.contains("/math.move")
        || file.contains("\\math.move")
        || file.contains("/hex.move")
        || file.contains("\\hex.move")
        || file.contains("/crypto/")
        || file.contains("\\crypto\\")
        || file.contains("/royalty_rule.move")
        || file.contains("\\royalty_rule.move")
    {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut funcs = Vec::new();
    descendants_by_kind(root, "function_definition", &mut funcs);

    for func in funcs {
        // Skip test-only functions (check preceding text for #[test_only]).
        let before = &source[..func.start_byte()];
        let is_test_only = before.lines().rev().take(5).any(|l| l.trim().contains("#[test_only]"));
        if is_test_only {
            continue;
        }
        // Skip functions whose names indicate bounded/encoding arithmetic.
        let fname = func_name_from_def(func, source).unwrap_or("");
        if fname.contains("pow")
            || fname.contains("hex")
            || fname.contains("encode")
            || fname.contains("format")
            || fname.contains("to_string")
            || fname.contains("calculate")
        {
            continue;
        }
        // Skip math/hex modules — their functions are inherently arithmetic.
        if file.contains("/math.move")
            || file.contains("\\math.move")
            || file.contains("/hex.move")
            || file.contains("\\hex.move")
        {
            continue;
        }
        let mut arith_calls: Vec<Node> = Vec::new();
        collect_arithmetic_calls(func, source, &mut arith_calls);
        if arith_calls.is_empty() {
            continue;
        }
        for call in arith_calls {
            // Per-expression check: only flag if no assert! precedes this
            // arithmetic expression within the same function.
            let func_start = func.start_byte();
            let arith_pos = call.start_byte();
            let func_text_before_arith = &source[func_start..arith_pos];
            if func_text_before_arith.contains("assert!") {
                continue;
            }
            let (line, col) = line_col(call);
            let text = node_text(call, source);
            out.push(AuditFinding::new(
                RuleId::IntegerOverflowRisk,
                Severity::Medium,
                "arithmetic without assertion guard",
                format!(
                    "Arithmetic `{}` has no preceding `assert!` in this function. \
                     Add bounds checks to prevent overflow.",
                    text.trim()
                ),
                file,
                line,
                col,
                Some(text.trim().to_string()),
            ));
        }
    }
    out
}

/// Heuristic: should this arithmetic expression be skipped (not an overflow risk)?
fn is_skipped_integer_overflow(text: &str, trimmed: &str) -> bool {
    // Loop-counter increments (i + 1, index + 1, j + 1, etc.)
    if trimmed.ends_with("+ 1")
        || trimmed.ends_with("+1")
        || trimmed.ends_with("- 1")
        || trimmed.ends_with("-1")
    {
        return true;
    }
    // Hex-encoding / ASCII arithmetic (bounded by input digit 0-15).
    if trimmed.contains("48 +")
        || trimmed.contains("87 +")
        || trimmed.contains("+ 48")
        || trimmed.contains("+ 87")
        || trimmed.contains("(v - 10)")
        || trimmed.contains("(value - 10)")
        || trimmed.contains("as u8)")
    {
        return true;
    }
    // Loop-counter * stride patterns.
    if trimmed.contains("as u8) * 8")
        || trimmed.contains("as u8) *8")
        || trimmed.contains("(n - 1) * 4")
    {
        return true;
    }
    // Offset + i (loop counter addition).
    if trimmed == "offset + i" {
        return true;
    }
    // u64-to-u128 widening multiplications (cannot overflow u128).
    if text.contains("as u128") && text.contains('*') {
        return true;
    }
    // Decimal scaling (bounded by small decimal counts).
    if text.contains("pow_u128") || text.contains("pow_u64") || text.contains("pow_u256") {
        return true;
    }
    // Loop accumulators (bounded by collection sizes, not inputs).
    if trimmed.starts_with("total_")
        || trimmed.contains("total_bid")
        || trimmed.contains("total_ask")
    {
        return true;
    }
    // Arithmetic involving compile-time constants (uppercase identifiers).

    text.split(|c: char| !c.is_alphanumeric() && c != '_').any(|tok| {
        !tok.is_empty()
            && tok.chars().any(|c| c.is_ascii_uppercase())
            && tok.chars().all(|c| c.is_ascii_uppercase() || c == '_')
    })
}

/// Collect binary expressions involving `+` or `*` on integer types.
fn collect_arithmetic_calls<'a>(node: Node<'a>, source: &'a str, out: &mut Vec<Node<'a>>) {
    if node.kind() == "binary_expression" {
        let text = node_text(node, source);
        // Skip comparisons (==, !=, <, >, <=, >=) — these are not overflow risks.
        if text.contains("==")
            || text.contains("!=")
            || text.contains("<=")
            || text.contains(">=")
            || text.contains('<')
            || text.contains('>')
        {
            // fall through to children
        } else if text.contains('+') || text.contains('*') {
            let trimmed = text.trim();
            if !is_skipped_integer_overflow(text, trimmed) {
                out.push(node);
            }
        }
    }
    for i in 0..node.named_child_count() {
        if let Some(c) = node.named_child(i as u32) {
            collect_arithmetic_calls(c, source, out);
        }
    }
}

// Detector: share_object without authentication ----------------------------

fn detect_share_object_without_auth(source: &str, file: &str, root: Node) -> Vec<AuditFinding> {
    let mut out = Vec::new();
    let mut calls = Vec::new();
    descendants_by_kind(root, "call_expression", &mut calls);

    for call in calls {
        let Some(name_expr) = child_by_name(call, "name_expression") else {
            continue;
        };
        let text = node_text(name_expr, source);
        if text != "share_object" && !text.ends_with("::share_object") {
            continue;
        }
        let Some(func) = enclosing_function(call) else {
            continue;
        };
        let func_name = func_name_from_def(func, source).unwrap_or("");
        if func_name.starts_with("test_") {
            continue;
        }
        if func_has_authenticated_param(func, source) {
            continue;
        }
        // Skip if the function takes an owned parameter (caller owns the object).
        if has_owned_parameter(func, source) {
            continue;
        }
        let (line, col) = line_col(call);
        out.push(AuditFinding::new(
            RuleId::ShareObjectWithoutAuth,
            Severity::High,
            format!("function `{func_name}` calls `share_object` without access control"),
            "`share_object` makes an object publicly mutable. Ensure the caller \
             is authenticated before sharing.",
            file,
            line,
            col,
            Some(
                source[call.start_byte()..call.end_byte()].lines().next().unwrap_or("").to_string(),
            ),
        ));
    }
    out
}

/// Heuristic: should this division be skipped (not a zero-check risk)?
fn is_skipped_division(divisor: &str, divisor_clean: &str) -> bool {
    if divisor.contains("MAX_") {
        return true;
    }
    if divisor_clean == "2" || divisor_clean == "10" {
        return true;
    }
    // Constant divisor (uppercase identifier) — bounded by definition.
    if divisor.chars().any(|c| c.is_ascii_uppercase()) {
        return true;
    }
    false
}

// Detector: division by zero risk ------------------------------------------

fn detect_division_by_zero_risk(source: &str, file: &str, root: Node) -> Vec<AuditFinding> {
    let mut out = Vec::new();
    let mut exprs = Vec::new();
    descendants_by_kind(root, "binary_expression", &mut exprs);

    for expr in exprs {
        let text = node_text(expr, source);
        // Strip comments to avoid false positives from `//` in trailing comments.
        let text_no_comments: String =
            text.lines().map(|l| l.split("//").next().unwrap_or(l)).collect::<Vec<_>>().join("\n");
        if !text_no_comments.contains('/') {
            continue;
        }
        // Skip if inside a test function or test file.
        let Some(func) = enclosing_function(expr) else {
            continue;
        };
        let func_name = func_name_from_def(func, source).unwrap_or("");
        if func_name.starts_with("test_") {
            continue;
        }
        if is_test_file(file) {
            continue;
        }
        // Skip math module (inherently arithmetic).
        if file.contains("/math.move") || file.contains("\\math.move") {
            continue;
        }
        // Check if there's an assert! before this expression in the function.
        let func_start = func.start_byte();
        let expr_pos = expr.start_byte();
        let before = &source[func_start..expr_pos];
        if before.contains("assert!") {
            continue;
        }
        let func_text = node_text(func, source);
        if func_text.contains("assert!") {
            continue;
        }
        let parts: Vec<&str> = text_no_comments.split('/').collect();
        if parts.len() >= 2 {
            let divisor = parts[parts.len() - 1].trim();
            // Strip trailing non-digit characters (e.g., trailing `)`).
            let divisor_clean: String = divisor.chars().filter(|c| c.is_ascii_digit()).collect();
            if divisor_clean.chars().all(|c| c.is_ascii_digit()) && !divisor_clean.is_empty() {
                continue;
            }
            if is_skipped_division(divisor, &divisor_clean) {
                continue;
            }
        }
        let (line, col) = line_col(expr);
        out.push(AuditFinding::new(
            RuleId::DivisionByZeroRisk,
            Severity::Medium,
            "division without zero-check",
            "Division may abort if the divisor is zero. Add `assert!(divisor != 0)` \
             before dividing.",
            file,
            line,
            col,
            Some(text.trim().to_string()),
        ));
    }
    out
}

// Detector: unchecked vector borrow ----------------------------------------

fn detect_unchecked_vector_borrow(source: &str, file: &str, root: Node) -> Vec<AuditFinding> {
    let mut out = Vec::new();
    let mut calls = Vec::new();
    descendants_by_kind(root, "call_expression", &mut calls);

    for call in calls {
        let Some(name_expr) = child_by_name(call, "name_expression") else {
            continue;
        };
        let text = node_text(name_expr, source);
        if text != "vector::borrow" && text != "vector::borrow_mut" {
            continue;
        }
        let Some(func) = enclosing_function(call) else {
            continue;
        };
        let func_name = func_name_from_def(func, source).unwrap_or("");
        if func_name.starts_with("test_") {
            continue;
        }
        if is_test_file(file) {
            continue;
        }
        // Skip crypto module (low-level byte manipulation).
        if file.contains("/crypto/") || file.contains("\\crypto\\") {
            continue;
        }
        let func_start = func.start_byte();
        let call_pos = call.start_byte();
        let before = &source[func_start..call_pos];
        if before.contains("assert!") || before.contains("length") || before.contains('<') {
            continue;
        }
        let (line, col) = line_col(call);
        out.push(AuditFinding::new(
            RuleId::UncheckedVectorBorrow,
            Severity::Medium,
            format!("`{text}` without bounds check"),
            "Vector borrow may abort if the index is out of bounds. Add a bounds \
             check (`assert!(i < vector::length(v))`) before borrowing.",
            file,
            line,
            col,
            Some(
                source[call.start_byte()..call.end_byte()].lines().next().unwrap_or("").to_string(),
            ),
        ));
    }
    out
}

// Detector: public entry function without signer ----------------------------

fn detect_public_entry_without_signer(source: &str, file: &str, root: Node) -> Vec<AuditFinding> {
    let mut out = Vec::new();
    let mut funcs = Vec::new();
    descendants_by_kind(root, "function_definition", &mut funcs);

    for func in funcs {
        let mods = func_modifiers(func, source);
        if !mods.contains(&"public") || !mods.contains(&"entry") {
            continue;
        }
        if func_has_authenticated_param(func, source) {
            continue;
        }
        let (line, col) = line_col(func);
        let name = func_name_from_def(func, source).unwrap_or("<unknown>");
        out.push(AuditFinding::new(
            RuleId::PublicEntryWithoutSigner,
            Severity::High,
            format!("public entry function `{name}` lacks a signer"),
            "Public entry functions should authenticate the caller with a \
             `signer` parameter to prevent unauthorized invocation.",
            file,
            line,
            col,
            Some(node_text(func, source).lines().next().unwrap_or("").to_string()),
        ));
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
