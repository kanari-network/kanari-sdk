//! Edge-case tests for code_action helpers.

use kanari_move_analyzer::code_action::{
    code_lenses, extract_unresolved_name, insert_import_edit, module_used_as_alias,
    scan_package_members_and_modules,
};
use lsp_types::TextDocumentIdentifier;

#[test]
fn extract_ignores_non_matches() {
    assert_eq!(extract_unresolved_name("some random error"), None);
    assert_eq!(extract_unresolved_name(""), None);
    assert_eq!(extract_unresolved_name("Unbound"), None);
    // earliest position wins
    assert_eq!(
        extract_unresolved_name("Unbound variable 'idx' and Could not resolve the name 'UID'"),
        Some("idx".to_string())
    );
    // duplicate pattern order — first occurrence in message wins
    assert_eq!(
        extract_unresolved_name("Could not resolve the name 'UID' then Unbound type 'TxContext'"),
        Some("UID".to_string())
    );
}

#[test]
fn module_used_ignores_comment_and_use_and_module_lines() {
    assert!(!module_used_as_alias(
        "// tx_context::sender in comment\nuse kanari_system::tx_context;\nmodule x::m {}",
        "tx_context"
    ));
    assert!(module_used_as_alias("fun f() { tx_context::sender(); }", "tx_context"));
    assert!(!module_used_as_alias("module tx_context::m { fun f() {} }", "tx_context"));
    // prefix collision: coin2:: must not count as coin::
    assert!(!module_used_as_alias("fun f() { coin2::value(); }", "coin"));
}

#[test]
fn insert_import_no_module_returns_none() {
    let lines = vec!["pub fun f() {}"];
    assert!(insert_import_edit(&lines, "x::y").is_none());
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
fn code_lenses_no_fun_name_skipped() {
    let params = lsp_types::CodeLensParams {
        text_document: TextDocumentIdentifier { uri: "file:///x.move".parse().unwrap() },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };
    let text = "#[test]\nstruct Foo {}\n";
    assert_eq!(code_lenses(&params, text).len(), 1, "only file lens");
}
