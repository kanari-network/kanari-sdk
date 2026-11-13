//! `while` / `for` with a non-block body need a space: `while (c) call()`, not `while (c)call()`.

use kanari_move_fmt::{FormatOptions, format_source};

#[test]
fn while_call_body_has_space() {
    let src = include_str!("../move_package/while_call_body.move");
    let out = format_source(src, &FormatOptions::default()).unwrap();
    eprintln!("{out}");
    assert!(
        out.contains("while (i < len) swap("),
        "expected space after while: {out}"
    );
    assert!(
        !out.contains(")swap("),
        "body must not be glued to condition: {out}"
    );
}

#[test]
fn while_block_body_no_double_space() {
    let src = include_str!("../move_package/while_block_body.move");
    let out = format_source(src, &FormatOptions::default()).unwrap();
    eprintln!("{out}");
    assert!(
        out.contains("while (true) {"),
        "block body keeps one space: {out}"
    );
    assert!(
        !out.contains("while (true)  {"),
        "must not double-space block: {out}"
    );
}

#[test]
fn for_loop_semicolon_form() {
    // Move `for` is C-style (tree-sitter parses it as for_statement);
    // at least the while/for spacing must not glue tokens.
    let src = include_str!("../move_package/for_loop.move");
    let out = format_source(src, &FormatOptions::default()).unwrap();
    eprintln!("{out}");
    assert!(!out.is_empty());
    assert!(!out.contains(")call("), "for body glued: {out}");
}
