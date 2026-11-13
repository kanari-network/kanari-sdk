//! Statement block `{ ... };` inside a function must indent exactly one
//! level (8 spaces), not 9 — the block's own leading ` ` previously added
//! one extra space.

use kanari_move_fmt::{FormatOptions, format_source};

#[test]
fn block_item_brace_indent_is_single_level() {
    let src = include_str!("../move_package/block_indent.move");
    let out = format_source(src, &FormatOptions::default()).unwrap();
    eprintln!("{out}");
    // The `{` of the inner block must be at 8 spaces (fn body level).
    let brace = out.lines().find(|l| l.trim() == "{").unwrap();
    let indent = brace.len() - brace.trim_start().len();
    assert_eq!(indent, 8, "inner block brace must be 8 spaces: '{brace}'");
    // Inner statements at 12.
    let stmt = out
        .lines()
        .find(|l| l.trim().starts_with("let value"))
        .unwrap();
    assert_eq!(stmt.len() - stmt.trim_start().len(), 12);
    // `};` at 8.
    let close = out.lines().find(|l| l.trim() == "};").unwrap();
    assert_eq!(close.len() - close.trim_start().len(), 8);
}
