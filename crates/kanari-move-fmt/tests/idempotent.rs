//! `format_source` must be idempotent: formatting twice gives the same output.

use kanari_move_fmt::{FormatOptions, format_source};

fn fmt(s: &str) -> String {
    format_source(s, &FormatOptions::default()).unwrap()
}

fn assert_idempotent(src: &str) {
    let once = fmt(src);
    let twice = fmt(&once);
    assert_eq!(
        once, twice,
        "fmt must be idempotent\n--- src ---\n{src}\n--- once ---\n{once}\n--- twice ---\n{twice}"
    );
}

#[test]
fn idempotent_else_chain() {
    assert_idempotent(
        "module m::n {\n    fun f(c: u8): u8 {\n        if (c >= 48 && c <= 57) c - 48\n        // 0-9\n        else if (c >= 65 && c <= 70) c - 55\n        // A-F\n        else abort E\n    }\n}\n",
    );
}

#[test]
fn idempotent_grouped_use() {
    assert_idempotent(
        "module m::n {\n    use std::vector;\n    use std::ascii;\n    use std::vector;\n}\n",
    );
}

#[test]
fn idempotent_long_call() {
    assert_idempotent(
        "module m::n {\n    fun f() {\n        some_fn(arg_one, arg_two, arg_three, arg_four, arg_five, arg_six, arg_seven);\n    }\n}\n",
    );
}

#[test]
fn idempotent_pack_and_let() {
    assert_idempotent(
        "module m::n {\n    struct S has drop { x: u64 }\n    fun f() { let s = S { x: 1 }; let y = s.x; }\n}\n",
    );
}

#[test]
fn idempotent_block_comment_if() {
    assert_idempotent(
        "module m::n {\n    fun f() {\n        if (/* 0-9 */ 48 <= hex && hex < 58) { hex - 48 } else { 0 }\n    }\n}\n",
    );
}
