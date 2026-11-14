//! Spacing regressions: double spaces before `{`, glued tokens, etc.

use kanari_move_fmt::{FormatOptions, format_source};

fn fmt(s: &str) -> String {
    format_source(s, &FormatOptions::default()).unwrap()
}

#[test]
fn else_block_single_space() {
    let out = fmt(include_str!("../move_package/spacing_else_block.move"));
    eprintln!("{out}");
    assert!(out.contains("} else {"), "expected single-space else block: {out}");
    assert!(!out.contains("else  {"), "double space before block brace: {out}");
}

#[test]
fn else_expr_single_space() {
    // Bare expr else arm needs exactly `else ` too.
    let out = fmt(include_str!("../move_package/spacing_else_expr.move"));
    eprintln!("{out}");
    assert!(out.contains("else c"), "else expr spacing: {out}");
    assert!(!out.contains("else  c"), "double space: {out}");
}

#[test]
fn else_if_chain_no_extra_space() {
    let src = include_str!("../move_package/spacing_else_if_chain.move");
    let out = fmt(src);
    eprintln!("{out}");
    assert!(out.contains("else if"), "else-if chain: {out}");
    assert!(!out.contains("else  if"), "double space: {out}");
}

#[test]
fn comment_before_else_still_single_space() {
    // `//` eats line — else must start fresh, block gives ` {`.
    let src = include_str!("../move_package/spacing_comment_else.move");
    let out = fmt(src);
    eprintln!("{out}");
    assert!(out.contains("else {"), "comment else block: {out}");
    assert!(!out.contains("else  {"), "double space after comment: {out}");
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
        include_str!("../move_package/core_spacing_1.move"),
        include_str!("../move_package/core_spacing_2.move"),
        include_str!("../move_package/core_spacing_3.move"),
        include_str!("../move_package/core_spacing_4.move"),
        include_str!("../move_package/core_spacing_5.move"),
    ];
    for (src, (want, _)) in srcs.iter().zip(cases.iter()) {
        let out = fmt(src);
        assert!(out.contains(want), "expected '{want}' in:\n{out}");
    }
}
