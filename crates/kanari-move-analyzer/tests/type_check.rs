//! Integration tests proving the compiler (and therefore the editor, which
//! surfaces compiler diagnostics) rejects wrong-typed Move code.
//!
//! These compile real Move sources through `move-compiler` (the same
//! compilation the LSP runs) instead of hand-written message strings.
//! If any of these start compiling cleanly, the editor would silently
//! stop warning about wrong types — that is the regression these guard.

use kanari_move_analyzer::diagnostics::is_kanari_false_positive_with_file;
use move_compiler::command_line::compiler::move_check_for_errors;
use move_compiler::diagnostics::{Diagnostics, report_diagnostics_to_buffer};
use move_compiler::editions::{Edition, Flavor};
use move_compiler::shared::{Flags, NumericalAddress, PackageConfig, PackagePaths};
use move_compiler::{Compiler, PASS_PARSER};
use std::collections::BTreeMap;

fn testing_addresses() -> BTreeMap<String, NumericalAddress> {
    vec![
        ("std", "0x1"),
        ("sui", "0x2"),
        ("M", "0x40"),
        ("A", "0x41"),
        ("B", "0x42"),
        ("K", "0x19"),
        ("a", "0x44"),
        ("b", "0x45"),
        ("k", "0x19"),
    ]
    .into_iter()
    .map(|(name, addr)| (name.to_string(), NumericalAddress::parse_str(addr).unwrap()))
    .collect()
}

/// Compile `source` as a single-file package. Returns diagnostics.
fn compile_source(source: &str) -> (String, Diagnostics) {
    compile_source_with_flags(source, Flags::empty().set_sources_shadow_deps(true))
}

/// Same as [`compile_source`] but with explicit compiler flags.
fn compile_source_with_flags(source: &str, flags: Flags) -> (String, Diagnostics) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.move");
    std::fs::write(&path, source).unwrap();
    let path_str = path.to_string_lossy().replace('\\', "/");

    let targets = vec![PackagePaths {
        name: None,
        paths: vec![path_str],
        named_address_map: testing_addresses(),
    }];
    let config = PackageConfig {
        edition: Edition::E2024,
        flavor: Flavor::default(),
        ..PackageConfig::default()
    };
    let compiler = Compiler::from_package_paths(None, targets, vec![])
        .unwrap()
        .set_flags(flags)
        .set_default_config(config);
    let (files, res) = compiler.run::<PASS_PARSER>().unwrap();
    let diags = move_check_for_errors(res);
    let rendered = if diags.is_empty() {
        String::new()
    } else {
        String::from_utf8(report_diagnostics_to_buffer(&files, diags.clone(), false)).unwrap()
    };
    let _ = dir.close();
    (rendered, diags)
}

fn diag_count(source: &str) -> (String, usize) {
    let (rendered, diags) = compile_source(source);
    (rendered, diags.len())
}

// ---------------------------------------------------------------------------
// valid code must compile cleanly
// ---------------------------------------------------------------------------

#[test]
fn valid_code_compiles_clean() {
    let src = "module M::f {\n    public fun add(a: u64, b: u64): u64 { a + b }\n}\n";
    let (rendered, n) = diag_count(src);
    assert_eq!(n, 0, "valid code must compile cleanly: {rendered}");
}

#[test]
fn valid_struct_and_function_compile_clean() {
    // Move 2024 requires `public` structs (no internal visibility).
    let src = "module M::f {\n    public struct S has drop { x: u64 }\n    public fun make(): S { S { x: 1 } }\n}\n";
    let (rendered, n) = diag_count(src);
    assert_eq!(n, 0, "valid struct code must compile cleanly: {rendered}");
}

// ---------------------------------------------------------------------------
// wrong types must produce diagnostics
// ---------------------------------------------------------------------------

#[test]
fn wrong_operand_type_errors() {
    // u64 + bool is incompatible
    let src = "module M::f {\n    public fun add(a: u64, b: u64): u64 { a + b + true }\n}\n";
    let (rendered, n) = diag_count(src);
    assert!(n > 0, "u64+bool must warn: {rendered}");
    assert!(
        rendered.contains("incompatible") || rendered.contains("Expected"),
        "diagnostic should explain the type mismatch: {rendered}"
    );
}

#[test]
fn wrong_assignment_type_errors() {
    // string literal where u64 expected
    let src = "module M::f {\n    public fun f() {\n        let x: u64 = \"hello\";\n    }\n}\n";
    let (rendered, n) = diag_count(src);
    assert!(n > 0, "u64 = string must warn: {rendered}");
}

#[test]
fn wrong_return_type_errors() {
    let src = "module M::f {\n    public fun f(): u64 { true }\n}\n";
    let (rendered, n) = diag_count(src);
    assert!(n > 0, "returning bool as u64 must warn: {rendered}");
}

#[test]
fn wrong_argument_type_errors() {
    let src =
        "module M::f {\n    public fun take(x: u64) {}\n    public fun f() { take(true); }\n}\n";
    let (rendered, n) = diag_count(src);
    assert!(n > 0, "bool arg as u64 must warn: {rendered}");
}

#[test]
fn wrong_struct_field_type_errors() {
    let src = "module M::f {\n    public struct S has drop { x: u64 }\n    public fun make(): S { S { x: true } }\n}\n";
    let (rendered, n) = diag_count(src);
    assert!(n > 0, "bool field as u64 must warn: {rendered}");
}

#[test]
fn unbound_type_errors() {
    // TxContext without import/definition must warn (proves missing-type
    // warnings are not swallowed)
    let src = "module M::f {\n    public fun f(ctx: &mut TxContext) {}\n}\n";
    let (rendered, n) = diag_count(src);
    assert!(n > 0, "unbound TxContext must warn: {rendered}");
}

#[test]
fn wrong_generic_arg_errors() {
    // providing bool where u64 expected in vector
    let src = "module M::f {\n    public fun f() {\n        let v: vector<u64> = vector::empty();\n        vector::push(&mut v, true);\n    }\n}\n";
    // Note: std library not linked (deps=[]), so this may warn about
    // unresolved `vector` instead — either way it MUST warn, not compile
    // cleanly. The point is: wrong-typed code never passes silently.
    let (rendered, n) = diag_count(src);
    assert!(n > 0, "wrong generic arg must warn: {rendered}");
}

#[test]
fn unused_private_function_warns() {
    // `fun init` (private, never called) must surface W09008 — a WARNING —
    // exactly like `kanari move build` does. Under Sui flavor the compiler
    // exempts `init`; under Kanari (Core) it must warn unless suppressed.
    let src = "module M::f {\n    fun unused_helper() {}\n}\n";
    let (rendered, n) = diag_count(src);
    assert!(n > 0, "unused private function must warn: {rendered}");
    assert!(
        rendered.contains("never called"),
        "expected 'never called' unused warning, got: {rendered}"
    );
}

#[test]
fn unused_init_function_warns_under_core_flavor() {
    // Direct analog of the user's `fun init(witness: M, ctx: &mut TxContext)`:
    // Core flavor (Kanari) must NOT exempt init from the unused-function
    // lint, so the editor and `kanari move build` agree.
    let src = "module M::f {\n    struct M has drop {}\n    fun init(witness: M, ctx: &mut TxContext) {}\n}\n";
    let (rendered, n) = diag_count(src);
    assert!(n > 0, "unused init must warn under Kanari flavor: {rendered}");
    assert!(
        rendered.contains("never called"),
        "expected 'never called' unused warning on init, got: {rendered}"
    );
}

#[test]
fn unused_warning_survives_testing_flags() {
    // The LSP compiles packages with `test_mode: true` (Flags::testing()).
    // If testing mode silenced the unused-function lint, the editor would
    // never show `fun init` as a warning while `kanari move build` does.
    let src = "module M::f {\n    fun unused_helper() {}\n}\n";
    let (rendered, diags) = compile_source_with_flags(src, Flags::testing());
    assert!(!diags.is_empty(), "unused-function warning must survive test mode: {rendered}");
    assert!(
        rendered.contains("never called"),
        "expected 'never called' unused warning under test mode, got: {rendered}"
    );
}

// ---------------------------------------------------------------------------
// filter must not hide real type errors, but must hide TxContext entry FP
// ---------------------------------------------------------------------------

#[test]
fn filter_does_not_hide_wrong_types() {
    // For each wrong-typed source, the flagged lines must not be filtered.
    let cases = [
        "module M::f {\n    public fun add(a: u64, b: u64): u64 { a + b + true }\n}\n",
        "module M::f {\n    public fun f() {\n        let x: u64 = \"hello\";\n    }\n}\n",
        "module M::f {\n    public fun f(): u64 { true }\n}\n",
        "module M::f {\n    public fun take(x: u64) {}\n    public fun f() { take(true); }\n}\n",
    ];
    for src in cases {
        let (rendered, n) = diag_count(src);
        assert!(n > 0, "case must warn: {rendered}");
        for line in 0..src.lines().count() {
            assert!(
                !is_kanari_false_positive_with_file(
                    "Invalid 'entry' parameter type for parameter 'foo'",
                    src,
                    line
                ),
                "entry filter must not hide wrong types (line {line})"
            );
        }
    }
}

#[test]
fn filter_still_hides_entry_tx_context_fp() {
    // Contrast: the Sui-checker entry diagnostic for a TxContext param IS
    // filtered, for any param name.
    for name in ["ctx", "_ctx", "my_ctx", "woof", "sender"] {
        let code =
            format!("module M::n {{\n    public entry fun f({name}: &mut TxContext) {{}}\n}}\n");
        let msg = format!("Invalid 'entry' parameter type for parameter '{name}'");
        assert!(
            is_kanari_false_positive_with_file(&msg, &code, 1),
            "entry TxContext ({name}) must be filtered"
        );
    }
}
