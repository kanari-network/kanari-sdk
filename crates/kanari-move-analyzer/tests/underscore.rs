//! Quick-fix: rename unused param/var to `_{name}`.

use kanari_move_analyzer::code_action::{extract_unused_name, rename_to_underscored};
use lsp_types::{Position, Range};

fn rng(line: u32, s: u32, e: u32) -> Range {
    Range::new(Position::new(line, s), Position::new(line, e))
}

#[test]
fn extract_param() {
    assert_eq!(
        extract_unused_name(
            "Unused parameter 'ctx'. Consider removing or prefixing with an underscore: '_ctx'"
        ),
        Some("ctx".to_string())
    );
}

#[test]
fn extract_local_variable() {
    assert_eq!(
        extract_unused_name(
            "Unused local variable 'x'. Consider removing or prefixing with an underscore: '_x'"
        ),
        Some("x".to_string())
    );
    assert_eq!(
        extract_unused_name(
            "Unused variable 'balance'. Consider removing or prefixing with an underscore: '_balance'"
        ),
        Some("balance".to_string())
    );
}

#[test]
fn extract_ignores_already_underscored_and_type_params() {
    assert_eq!(
        extract_unused_name(
            "Unused parameter '_ctx'. Consider removing or prefixing with an underscore: '__ctx'"
        ),
        None,
        "already underscored must not produce an action"
    );
    assert_eq!(
        extract_unused_name("Unused type parameter 'T'. Consider declaring it as phantom"),
        None
    );
    assert_eq!(extract_unused_name("Unbound type 'TxContext'"), None);
}

#[test]
fn extract_word_boundary() {
    // `NotUnused` must not match
    assert_eq!(extract_unused_name("NotUnused parameter 'x'"), None);
}

#[test]
fn rename_exact_range() {
    let lines = vec!["    fun f(ctx: &mut TxContext) {"];
    // column of `ctx` = 10
    let edits = rename_to_underscored(&lines, rng(0, 10, 13), "ctx").unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "_ctx");
    assert_eq!(edits[0].range.start.character, 10);
    assert_eq!(edits[0].range.end.character, 13);
}

#[test]
fn rename_zero_length_range_inserts_underscore() {
    let lines = vec!["    fun f(ctx: u8) {"];
    let edits = rename_to_underscored(&lines, rng(0, 10, 10), "ctx").unwrap();
    assert_eq!(edits[0].new_text, "_");
    assert_eq!(edits[0].range.start.character, 10);
}

#[test]
fn rename_fallback_finds_token_in_line() {
    let lines = vec!["    fun f(ctx: u8) {"];
    // wrong column (points at `f`) — fallback searches line
    let edits = rename_to_underscored(&lines, rng(0, 8, 9), "ctx").unwrap();
    assert_eq!(edits[0].new_text, "_ctx");
    // actual `ctx` at col 10
    assert_eq!(edits[0].range.start.character, 10);
}

#[test]
fn rename_skips_already_underscored_and_missing() {
    let lines = vec!["    fun f(_ctx: u8) {"];
    assert!(rename_to_underscored(&lines, rng(0, 10, 14), "_ctx").is_none());
    let lines2 = vec!["    fun f(a: u8) {"];
    assert!(rename_to_underscored(&lines2, rng(0, 10, 11), "ctx").is_none());
    // out of bounds line
    assert!(rename_to_underscored(&lines2, rng(99, 0, 0), "ctx").is_none());
}
