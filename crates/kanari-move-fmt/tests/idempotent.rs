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
    assert_idempotent(include_str!("../move_package/else_chain.move"));
}

#[test]
fn idempotent_grouped_use() {
    assert_idempotent(include_str!("../move_package/grouped_use.move"));
}

#[test]
fn idempotent_long_call() {
    assert_idempotent(include_str!("../move_package/long_call.move"));
}

#[test]
fn idempotent_pack_and_let() {
    assert_idempotent(include_str!("../move_package/pack_and_let.move"));
}

#[test]
fn idempotent_block_comment_if() {
    assert_idempotent(include_str!("../move_package/block_comment_if.move"));
}
