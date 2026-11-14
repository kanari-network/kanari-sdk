//! Tests for Move 2024 support in kanari-move-analyzer.
//! Run: cargo test -p kanari-move-analyzer

use kanari_move_analyzer::code_action::{
    code_lenses, extract_unresolved_name, format_document, insert_import_edit,
    module_used_as_alias, on_code_action_request,
};
use kanari_move_analyzer::edition::{MoveEdition, parse_edition};
use lsp_types::{
    CodeActionContext, CodeActionParams, Diagnostic, DiagnosticSeverity, Position, Range,
    TextDocumentIdentifier, Uri, WorkDoneProgressParams,
};

fn uri_for(path: &std::path::Path) -> Uri {
    format!("file://{}", path.to_string_lossy().replace('\\', "/")).parse().unwrap()
}

fn params_for(file: &std::path::Path, diags: Vec<Diagnostic>) -> CodeActionParams {
    CodeActionParams {
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: Default::default(),
        text_document: TextDocumentIdentifier { uri: uri_for(file) },
        range: Range::new(Position::new(0, 0), Position::new(0, 0)),
        context: CodeActionContext { diagnostics: diags, only: None, trigger_kind: None },
    }
}

fn diag(msg: &str) -> Diagnostic {
    Diagnostic {
        range: Range::new(Position::new(0, 0), Position::new(0, 0)),
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        source: Some("move-analyzer".to_string()),
        message: msg.to_string(),
        related_information: None,
        tags: None,
        code_description: None,
        data: None,
    }
}

// ---------------------------------------------------------------------------
// extract_unresolved_name

#[test]
fn extracts_unbound_type() {
    assert_eq!(
        extract_unresolved_name("Unbound type 'TxContext' in current scope"),
        Some("TxContext".to_string())
    );
}

#[test]
fn extracts_unbound_module_alias() {
    assert_eq!(
        extract_unresolved_name("Unbound module alias 'transfer'"),
        Some("transfer".to_string())
    );
    assert_eq!(
        extract_unresolved_name("Unbound module alias 'tx_context'"),
        Some("tx_context".to_string())
    );
}

#[test]
fn extracts_could_not_resolve() {
    assert_eq!(
        extract_unresolved_name("Could not resolve the name 'UID'"),
        Some("UID".to_string())
    );
}

#[test]
fn extracts_unbound_variable() {
    assert_eq!(extract_unresolved_name("Unbound variable 'idx'"), Some("idx".to_string()));
}

#[test]
fn no_name_for_other_messages() {
    assert_eq!(extract_unresolved_name("some random error"), None);
}

// ---------------------------------------------------------------------------
// Edition 2024 prelude suppression

#[test]
fn candidates_suppressed_for_prelude_in_2024() {
    // In 2024, `vector`/`option` come from the prelude — the TS
    // provider used to stay quiet for them; the Rust one must too.
    // (candidates_for is edition-agnostic; the suppression happens in
    // on_code_action_request via edition::is_prelude.)
    assert_eq!(parse_edition("edition = \"2024\""), MoveEdition::Edition2024);
    assert_eq!(parse_edition("edition = \"legacy\""), MoveEdition::Legacy);
    // Empty (no edition line) → Legacy by convention.
    assert_eq!(parse_edition("[package]\n"), MoveEdition::Legacy);
}

#[test]
fn no_quick_fix_for_vector_in_2024_package() {
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("pkg");
    std::fs::create_dir_all(pkg.join("sources")).unwrap();
    std::fs::write(pkg.join("Move.toml"), "[package]\nname=\"x\"\nedition=\"2024\"\n").unwrap();
    let file = pkg.join("sources").join("mod.move");
    std::fs::write(
        &file,
        "module x::m { public fun f(): u64 { vector::length(&vector::empty()) } }",
    )
    .unwrap();
    let actions = on_code_action_request(&params_for(
        &file,
        vec![diag("Could not resolve the name 'vector'")],
    ));
    assert!(actions.is_empty(), "2024 prelude should not offer import for `vector`: {actions:?}");
}

#[test]
fn merged_self_import_for_txcontext_in_2024() {
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("pkg");
    std::fs::create_dir_all(pkg.join("sources")).unwrap();
    std::fs::write(
        pkg.join("Move.toml"),
        "[package]\nname=\"x\"\nedition=\"2024\"\n[dependencies]\nKanariSystem = { local = \"./kanari-system\" }\n[addresses]\nx=\"0x1\"\nkanari_system=\"0x2\"\n",
    )
    .unwrap();
    let dep = pkg.join("kanari-system");
    std::fs::create_dir_all(dep.join("sources")).unwrap();
    std::fs::write(
        dep.join("sources").join("tx_context.move"),
        "module kanari_system::tx_context { public struct TxContext {} }\n",
    )
    .unwrap();
    // own sources need to see membership - scan picks up via local deps
    let file = pkg.join("sources").join("mod.move");
    std::fs::write(
        &file,
        "module x::m {\n    public fun f(ctx: &mut TxContext) { tx_context::sender(ctx); }\n}\n",
    )
    .unwrap();
    let actions = on_code_action_request(&params_for(
        &file,
        vec![diag("Unbound type 'TxContext' in current scope")],
    ));
    assert_eq!(actions.len(), 1, "{actions:?}");
    assert!(actions[0].title.contains("{Self, TxContext}"), "{}", actions[0].title);
    assert!(actions[0].title.contains("tx_context"), "{}", actions[0].title);
}

// ---------------------------------------------------------------------------
// module_used_as_alias + insert_import_edit

#[test]
fn module_used_as_alias_ignores_use_lines() {
    let text = "use kanari_system::object::{Self, UID};\nfun f() { object::new(ctx); }\n";
    assert!(module_used_as_alias(text, "object"));
    let text2 = "use kanari_system::object::{Self, UID};\nfun f() {}\n";
    assert!(!module_used_as_alias(text2, "object"));
}

#[test]
fn insert_import_edit_after_last_use() {
    let lines = vec![
        "module x::m {",
        "    use std::bcs;",
        "    use kanari_system::event;",
        "    public fun f() {}",
        "}",
    ];
    let edit = insert_import_edit(&lines, "kanari_system::object::{Self, UID}").unwrap();
    assert_eq!(edit.range.start.line, 3);
    assert!(edit.new_text.contains("use kanari_system::object::{Self, UID};"));
}

#[test]
fn insert_import_edit_after_module_when_no_use() {
    let lines = vec!["module x::m {", "    public fun f() {}", "}"];
    let edit = insert_import_edit(&lines, "kanari_system::transfer").unwrap();
    assert_eq!(edit.range.start.line, 1);
}

// ---------------------------------------------------------------------------
// candidates_for

#[test]
fn scan_finds_dep_member_and_module() {
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("pkg");
    std::fs::create_dir_all(pkg.join("sources")).unwrap();
    std::fs::write(
        pkg.join("Move.toml"),
        "[package]\nname=\"x\"\n[dependencies]\nKanariSystem = { local = \"./kanari-system\" }\n",
    )
    .unwrap();
    let dep = pkg.join("kanari-system");
    std::fs::create_dir_all(dep.join("sources")).unwrap();
    std::fs::write(
        dep.join("sources").join("transfer.move"),
        "module kanari_system::transfer { public fun public_transfer() {} }\n",
    )
    .unwrap();
    std::fs::write(
        dep.join("sources").join("tx_context.move"),
        "module kanari_system::tx_context { public struct TxContext {} }\n",
    )
    .unwrap();
    let (members, modules) = kanari_move_analyzer::code_action::scan_all_members_and_modules(&pkg);
    assert!(members.contains_key("TxContext"));
    assert!(modules.contains_key("transfer"));
}

// ---------------------------------------------------------------------------
// Formatting + CodeLens

#[test]
fn format_document_empty_is_noop() {
    let params = lsp_types::DocumentFormattingParams {
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
    assert!(format_document(&params, "").is_empty());
    assert!(format_document(&params, "   \n").is_empty());
}

#[test]
fn code_lenses_finds_tests() {
    let params = lsp_types::CodeLensParams {
        text_document: TextDocumentIdentifier { uri: "file:///x.move".parse().unwrap() },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };
    let text = "#[test]\npublic fun test_add() {}\n#[test, expected_failure]\nfun test_sub() {}\n";
    let lenses = code_lenses(&params, text);
    // 1 file lens + 2 per-test lenses
    assert_eq!(lenses.len(), 3);
    assert!(lenses[1].command.as_ref().unwrap().title.contains("Run Test"));
}

#[test]
fn code_lenses_no_tests_only_file_lens() {
    let params = lsp_types::CodeLensParams {
        text_document: TextDocumentIdentifier { uri: "file:///x.move".parse().unwrap() },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };
    assert_eq!(code_lenses(&params, "public fun f() {}\n").len(), 1);
}

// ---------------------------------------------------------------------------
// Legacy vs 2024: same diagnostic, different suggestion behavior

#[test]
fn scan_package_members_finds_own_lib_fun() {
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("pkg");
    std::fs::create_dir_all(pkg.join("sources")).unwrap();
    std::fs::write(
        pkg.join("Move.toml"),
        "[package]\nname=\"deepbook\"\nedition=\"2024\"\n[addresses]\ndeepbook=\"0x123\"\n",
    )
    .unwrap();
    std::fs::write(
        pkg.join("sources").join("deepbook.move"),
        "module deepbook::deepbook {\n    public fun create_order_book_with_decimals<Base, Quote>(fee_bps: u64) {}\n    public fun other_fun() {}\n}\n",
    )
    .unwrap();
    let root = pkg.clone();
    let map = kanari_move_analyzer::code_action::scan_package_members(&root);
    assert_eq!(
        map.get("create_order_book_with_decimals"),
        Some(&("deepbook".to_string(), "deepbook".to_string()))
    );
    assert!(map.contains_key("other_fun"));
    // And code action offers the import for a file in the same package
    std::fs::write(
        pkg.join("sources").join("registry.move"),
        "module deepbook::registry {\n    public fun get() { create_order_book_with_decimals<u64, u64>(1); }\n}\n",
    )
    .unwrap();
    let actions = on_code_action_request(&params_for(
        &pkg.join("sources").join("registry.move"),
        vec![diag("Unbound function 'create_order_book_with_decimals' in current scope")],
    ));
    assert_eq!(actions.len(), 1, "{actions:?}");
    assert!(actions[0].title.contains("create_order_book_with_decimals"), "{}", actions[0].title);
}

#[test]
fn grouped_same_module_import_is_single_line() {
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("pkg");
    std::fs::create_dir_all(pkg.join("sources")).unwrap();
    std::fs::write(
        pkg.join("Move.toml"),
        "[package]\nname=\"deepbook\"\nedition=\"2024\"\n[addresses]\ndeepbook=\"0x123\"\n",
    )
    .unwrap();
    std::fs::write(
        pkg.join("sources").join("deepbook.move"),
        "module deepbook::deepbook {\n    public fun foo() {}\n    public fun bar() {}\n    public struct MyStruct {}\n}\n",
    )
    .unwrap();
    std::fs::write(
        pkg.join("sources").join("registry.move"),
        "module deepbook::registry {\n    public fun get() { foo(); bar(); MyStruct {}; }\n}\n",
    )
    .unwrap();
    let actions = on_code_action_request(&params_for(
        &pkg.join("sources").join("registry.move"),
        vec![diag("Unbound function 'foo'"), diag("Unbound function 'bar'")],
    ));
    // Same module → single merged action, not two lines.
    assert_eq!(actions.len(), 1, "{actions:?}");
    let title = &actions[0].title;
    assert!(title.contains("foo") && title.contains("bar"), "{title}");
}

#[test]
fn grouped_same_module_no_pollution_with_other_modules() {
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("pkg");
    std::fs::create_dir_all(pkg.join("sources")).unwrap();
    std::fs::write(
        pkg.join("Move.toml"),
        "[package]\nname=\"x\"\nedition=\"2024\"\n[dependencies]\nKanariSystem = { local = \"./ks\" }\n",
    )
    .unwrap();
    let ks = pkg.join("ks");
    std::fs::create_dir_all(ks.join("sources")).unwrap();
    std::fs::write(
        ks.join("sources").join("table.move"),
        "module kanari_system::table { public struct Table {} }\n",
    )
    .unwrap();
    std::fs::write(
        ks.join("sources").join("tx_context.move"),
        "module kanari_system::tx_context { public struct TxContext {} public fun sender() {} }\n",
    )
    .unwrap();
    // registry file has a table import already
    std::fs::write(
        pkg.join("sources").join("reg.move"),
        "module x::reg {\n    use kanari_system::table::{Self, Table};\n    use kanari_system::tx_context::{Self, TxContext};\n    public fun get() { tx_context::sender(); }\n}\n",
    ).unwrap();
    let actions = on_code_action_request(&params_for(
        &pkg.join("sources").join("reg.move"),
        vec![diag("Unbound type 'TxContext' in current scope")],
    ));
    // TxContext must stay in tx_context group, NOT be merged into table or vice versa.
    for a in &actions {
        assert!(!a.title.contains("table::"), "polluted: {}", a.title);
    }
}

#[test]
fn legacy_package_offers_vector_import() {
    // No fallback: std only appears if it exists on disk as a local dep.
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("pkg");
    std::fs::create_dir_all(pkg.join("sources")).unwrap();
    std::fs::write(
        pkg.join("Move.toml"),
        "[package]\nname=\"x\"\nedition=\"legacy\"\n[dependencies]\nStd = { local = \"./std\" }\n",
    )
    .unwrap();
    let std_dep = pkg.join("std");
    std::fs::create_dir_all(std_dep.join("sources")).unwrap();
    std::fs::write(
        std_dep.join("sources").join("vector.move"),
        "module std::vector { public fun empty() {} }\n",
    )
    .unwrap();
    let file = pkg.join("sources").join("mod.move");
    std::fs::write(&file, "module x::m { public fun f() { vector::empty<u64>(); } }").unwrap();
    let actions = on_code_action_request(&params_for(
        &file,
        vec![diag("Could not resolve the name 'vector'")],
    ));
    // In legacy there is no prelude — import is offered.
    assert_eq!(actions.len(), 1, "{actions:?}");
    assert!(actions[0].title.contains("std::vector"), "{}", actions[0].title);
}
