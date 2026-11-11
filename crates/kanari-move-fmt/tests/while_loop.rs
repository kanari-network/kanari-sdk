//! `while` / `for` with a non-block body need a space: `while (c) call()`, not `while (c)call()`.

use kanari_move_fmt::{FormatOptions, format_source};

#[test]
fn while_call_body_has_space() {
    let src = "module M::v { fun f(v: vector<u64>, i: u64) { while (i < len) swap(v, i, { i = i + 1; i }); } }";
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
    let src = "module M::v { fun f() { while (true) { len = len - 1; } } }";
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
    let src = "module M::v { fun f() { for (i = 0; i < 10; i = i + 1) call(i); } }";
    let out = format_source(src, &FormatOptions::default()).unwrap();
    eprintln!("{out}");
    assert!(!out.is_empty());
    assert!(!out.contains(")call("), "for body glued: {out}");
}
