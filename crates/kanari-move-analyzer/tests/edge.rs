//! Edge-case tests for code_action helpers.

use kanari_move_analyzer::code_action::{
    code_lenses, extract_unresolved_name, insert_import_edit, module_used_as_alias,
    scan_package_members_and_modules,
};
use lsp_types::TextDocumentIdentifier;

#[test]
fn extract_empty_and_unrelated() {
    assert_eq!(extract_unresolved_name(""), None);
    assert_eq!(extract_unresolved_name("Unbound"), None);
    assert_eq!(extract_unresolved_name("Unbound type ''"), None);
    assert_eq!(extract_unresolved_name("Unbound type '123bad'"), None);
}

#[test]
fn extract_prefers_first_pattern() {
    // Message with two patterns — first match wins.
    assert_eq!(
        extract_unresolved_name("Unbound variable 'idx' and Could not resolve the name 'UID'"),
        Some("idx".to_string())
    );
}

#[test]
fn module_used_ignores_comment_use_and_module_lines() {
    // Comment mentioning tx_context:: should NOT count as used.
    assert!(!module_used_as_alias(
        "// tx_context::sender in comment\nuse kanari_system::tx_context;\nmodule x::m {}",
        "tx_context"
    ));
    assert!(module_used_as_alias(
        "module x::m { fun f() { tx_context::sender(); } }",
        "tx_context"
    ));
    // Prefix collision: `coin2::` should not count as `coin::`
    assert!(!module_used_as_alias("fun f() { coin2::value(); }", "coin"));
}

#[test]
fn insert_import_no_module_returns_none() {
    let lines = vec!["pub fun f() {}"];
    assert!(insert_import_edit(&lines, "x::y").is_none());
}

#[test]
fn insert_import_indent_preserved() {
    let lines = vec![
        "module x::m {",
        "    use std::bcs;",
        "    public fun f() {}",
        "}",
    ];
    let edit = insert_import_edit(&lines, "kanari_system::transfer").unwrap();
    assert!(
        edit.new_text.starts_with("    use"),
        "indent: {:?}",
        edit.new_text
    );
    assert_eq!(
        edit.range.start.line, 2,
        "should be after last use (line 2)"
    );
}

#[test]
fn scan_inline_struct_and_fun() {
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("pkg");
    std::fs::create_dir_all(pkg.join("sources")).unwrap();
    std::fs::write(pkg.join("Move.toml"), "[package]\nname=\"x\"\n").unwrap();
    std::fs::write(
        pkg.join("sources").join("m.move"),
        "module kanari_system::tx_context { struct TxContext has drop {} public fun sender() {} }\n",
    ).unwrap();
    let (members, modules) = scan_package_members_and_modules(&pkg);
    assert!(members.contains_key("TxContext"), "members: {members:?}");
    assert!(members.contains_key("sender"));
    assert!(modules.contains_key("tx_context"));
}

#[test]
fn scan_empty_package() {
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("pkg");
    std::fs::create_dir_all(pkg.join("sources")).unwrap();
    std::fs::write(pkg.join("Move.toml"), "[package]\nname=\"x\"\n").unwrap();
    let (members, modules) = scan_package_members_and_modules(&pkg);
    assert!(members.is_empty());
    assert!(modules.is_empty());
}

#[test]
fn code_lenses_no_fun_name_skipped() {
    let params = lsp_types::CodeLensParams {
        text_document: TextDocumentIdentifier {
            uri: "file:///x.move".parse().unwrap(),
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };
    let text = "#[test]\nstruct Foo {}\n";
    assert_eq!(code_lenses(&params, text).len(), 1, "only file lens");
}

#[test]
fn format_empty_is_noop_exists_in_lib() {
    // smoke: format_document exists and is callable (we cannot run `kanari`
    // in unit tests, but empty text must be a no-op without spawning).
    let params = lsp_types::DocumentFormattingParams {
        text_document: TextDocumentIdentifier {
            uri: "file:///x.move".parse().unwrap(),
        },
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
    assert!(kanari_move_analyzer::code_action::format_document(&params, "").is_empty());
}

#[test]
fn edition_missing_is_legacy_no_prelude() {
    use kanari_move_analyzer::edition::{MoveEdition, is_prelude};
    assert!(!is_prelude("vector", &MoveEdition::Legacy));
    assert!(is_prelude("vector", &MoveEdition::Edition2024));
}
