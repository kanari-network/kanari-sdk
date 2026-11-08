//! Spacing regressions: double spaces before `{`, glued tokens, etc.

use kanari_move_fmt::{FormatOptions, format_source};

fn fmt(s: &str) -> String {
    format_source(s, &FormatOptions::default()).unwrap()
}

#[test]
fn else_block_single_space() {
    let out = fmt("module M::n { public fun f() { if (a > b) { x } else { y } } }");
    eprintln!("{out}");
    assert!(
        out.contains("} else {"),
        "expected single-space else block: {out}"
    );
    assert!(
        !out.contains("else  {"),
        "double space before block brace: {out}"
    );
}

#[test]
fn else_expr_single_space() {
    // Bare expr else arm needs exactly `else ` too.
    let out = fmt("module M::n { public fun f(c: u8): u8 { if (c > 48) c - 48 else c } }");
    eprintln!("{out}");
    assert!(out.contains("else c"), "else expr spacing: {out}");
    assert!(!out.contains("else  c"), "double space: {out}");
}

#[test]
fn else_if_chain_no_extra_space() {
    let src = "module M::n { fun f(c: u8): u8 { if (c >= 48 && c <= 57) c - 48\n // 0-9\n else if (c >= 65) c - 55\n else abort E } }";
    let out = fmt(src);
    eprintln!("{out}");
    assert!(out.contains("else if"), "else-if chain: {out}");
    assert!(!out.contains("else  if"), "double space: {out}");
}

#[test]
fn comment_before_else_still_single_space() {
    // `//` eats line — else must start fresh, block gives ` {`.
    let src = "module M::n { fun f(c: u8): u8 { if (c > 1) c // hi\n else { 2 } } }";
    let out = fmt(src);
    eprintln!("{out}");
    assert!(out.contains("else {"), "comment else block: {out}");
    assert!(
        !out.contains("else  {"),
        "double space after comment: {out}"
    );
}

#[test]
fn core_spacing_sanity() {
    let cases = [
        ("&mut u64", "a: &mut u64"),
        ("let (a, b) = (1, 2);", "let (a, b) = (1, 2);"),
        ("has key, store", "has key, store"),
        ("return 1", "return 1"),
        ("assert!(true, 1);", "assert!(true, 1);"),
    ];
    let srcs = [
        "module M::n { public fun f(a: &mut u64): u64 { *a + 1 } }",
        "module M::n { public fun f() { let (a, b) = (1, 2); } }",
        "module M::n { public struct S has key, store { x: u64 } }",
        "module M::n { public fun f(): u64 { return 1 } }",
        "module M::n { public fun f() { assert!(true, 1); } }",
    ];
    for (src, (want, _)) in srcs.iter().zip(cases.iter()) {
        let out = fmt(src);
        assert!(out.contains(want), "expected '{want}' in:\n{out}");
    }
}
