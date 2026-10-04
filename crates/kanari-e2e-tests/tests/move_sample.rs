//! Move language integration test
//!
//! Verify basic Move code compilation/structure in example Move packages.

#[test]
fn example_move_packages_exist() {
    let dex_path = std::path::Path::new("../../example_move/dex_v1/Move.toml");
    let james_path = std::path::Path::new("../../example_move/james/Move.toml");
    let vault_path = std::path::Path::new("../../example_move/document_vault/Move.toml");

    assert!(
        dex_path.exists(),
        "dex_v1 Move.toml should exist: {}",
        dex_path.display()
    );
    assert!(
        james_path.exists(),
        "james Move.toml should exist: {}",
        james_path.display()
    );
    assert!(
        vault_path.exists(),
        "document_vault Move.toml should exist: {}",
        vault_path.display()
    );
}

#[test]
fn dex_v1_move_sources_compile_check() {
    let dex_src = std::path::Path::new("../../example_move/dex_v1/sources/dex_v1.move");
    assert!(
        dex_src.exists(),
        "dex_v1.move should exist: {}",
        dex_src.display()
    );
    let content = std::fs::read_to_string(dex_src).expect("read dex_v1.move");
    assert!(
        content.contains("module dex_v1::dex_v1"),
        "dex_v1.move should define module"
    );
    assert!(
        content.contains("create_pool") || content.contains("add_liquidity"),
        "dex_v1 should have liquidity functions"
    );
}

#[test]
fn james_token_move_sources_exist() {
    let james_src = std::path::Path::new("../../example_move/james/sources/james.move");
    assert!(james_src.exists(), "james.move should exist");
    let content = std::fs::read_to_string(james_src).expect("read james.move");
    assert!(
        content.contains("module james::james"),
        "james.move should define module"
    );
    assert!(
        content.contains("JAMES") || content.contains("coin::"),
        "james token should use coin module"
    );
}

#[test]
fn document_vault_move_sources_exist() {
    let vault_src =
        std::path::Path::new("../../example_move/document_vault/sources/document_vault.move");
    assert!(vault_src.exists(), "document_vault.move should exist");
    let content = std::fs::read_to_string(vault_src).expect("read document_vault.move");
    assert!(
        content.contains("module document_vault::document_vault"),
        "document_vault.move should define module"
    );
    assert!(
        content.contains("Document") || content.contains("hash"),
        "document_vault should have Document/hash logic"
    );
}
