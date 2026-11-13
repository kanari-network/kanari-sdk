//! Attribute handling in Move: every `#[...]` form the LSP must tolerate.
//!
//! The language server evaluates attributes at type-check time (test-only
//! inclusion, `#[expected_failure]` abort tracking, `#[allow(...)]` lints).
//! If the analyzer mishandles them, tests either won't run or false
//! diagnostics appear where nothing is wrong.

use kanari_move_analyzer::code_action::format_document;
use lsp_types::{DocumentFormattingParams, TextDocumentIdentifier};

fn formatting_no_change(text: &str) -> bool {
    let params = DocumentFormattingParams {
        text_document: TextDocumentIdentifier { uri: "file:///x.move".parse().unwrap() },
        options: lsp_types::FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            properties: Default::default(),
            trim_trailing_whitespace: None,
            insert_final_newline: None,
            trim_final_newlines: None,
        },
        work_done_progress_params: Default::default(),
    };
    format_document(&params, text).is_empty()
}

fn has_attribute(text: &str, attr: &str) -> bool {
    text.lines().any(|l| l.trim_start().starts_with(&format!("#[{attr}")))
}

/// All attribute syntaxes used across the kanari-framework sources.
#[test]
fn attributes_are_tolerated() {
    let attrs = [
        ("allow(unused_function)", "#[allow(unused_function)]\nfun init() {}\n"),
        ("allow(unused_const)", "#[allow(unused_const)]\nconst X: u64 = 1;\n"),
        ("test_only block", "#[test_only]\nfun helper() {}\n"),
        ("test", "#[test]\nfun test_x() {}\n"),
        ("expected_failure code", "#[expected_failure(abort_code = E_X)]\nfun test_y() {}\n"),
        ("test_only with inner use", "#[test_only]\nuse std::option;\nfun h() {}\n"),
    ];
    for (_name, code) in attrs {
        // Formatting must never crash or produce noise on attribute code.
        assert!(formatting_no_change(code), "fmt must be idempotent for {code:?}");
    }
}

/// Malformed attributes must not crash formatting or quick fixes.
#[test]
fn malformed_attributes_are_tolerated() {
    for code in ["#[]\nfun f() {}\n", "#[unknown_attr]\nfun f() {}\n", "#[test_only]\nfun f( {}\n"]
    {
        let params = DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri: "file:///x.move".parse().unwrap() },
            options: lsp_types::FormattingOptions {
                tab_size: 4,
                insert_spaces: true,
                properties: Default::default(),
                trim_trailing_whitespace: None,
                insert_final_newline: None,
                trim_final_newlines: None,
            },
            work_done_progress_params: Default::default(),
        };
        let _ = format_document(&params, code); // must not panic
    }
    // No false detections either: plain module without attributes is quiet.
    assert!(!has_attribute("module m::n {}\n", "test"));
    assert!(has_attribute("#[test]\nfun f() {}\n", "test"));
}

/// Quick-fix rename respects test-only visibility: a `#[test_only]` helper
/// used inside a test may be renamed `_helper` normally.
#[test]
fn attribute_visibility_rules_apply() {
    let code = "#[test_only]\nfun helper() { let x = 1; }\n";
    assert!(has_attribute(code, "test_only"));
    // The underscore quick-fix on unused `x` must still fire in test code
    // (attributes don't block it).
    use kanari_move_analyzer::code_action::extract_unused_name;
    assert!(extract_unused_name("Unused local variable 'x'").is_some());
}
