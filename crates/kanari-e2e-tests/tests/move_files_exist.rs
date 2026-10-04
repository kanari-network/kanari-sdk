//! Move source files organization test

#[test]
fn move_source_files_exist_in_sources_folder() {
    let sources_dir = std::path::Path::new("move_sources");
    assert!(
        sources_dir.exists() && sources_dir.is_dir(),
        "move_sources directory should exist"
    );

    let files = [
        "move_e2e_basic.move",
        "move_e2e_comprehensive.move",
        "move_e2e_math.move",
        "move_e2e_events.move",
        "move_e2e_access.move",
    ];

    for f in &files {
        let path = sources_dir.join(f);
        assert!(
            path.exists(),
            "Move source should exist: {}",
            path.display()
        );
    }
}

#[test]
fn move_modules_have_correct_names() {
    let basic = std::fs::read_to_string("move_sources/move_e2e_basic.move").unwrap();
    assert!(basic.contains("module kanari_e2e_tests::move_e2e_basic"));

    let comp = std::fs::read_to_string("move_sources/move_e2e_comprehensive.move").unwrap();
    assert!(comp.contains("module kanari_e2e_tests::move_e2e_comprehensive"));

    let math = std::fs::read_to_string("move_sources/move_e2e_math.move").unwrap();
    assert!(math.contains("module kanari_e2e_tests::move_e2e_math"));

    let events = std::fs::read_to_string("move_sources/move_e2e_events.move").unwrap();
    assert!(events.contains("module kanari_e2e_tests::move_e2e_events"));

    let access = std::fs::read_to_string("move_sources/move_e2e_access.move").unwrap();
    assert!(access.contains("module kanari_e2e_tests::move_e2e_access"));
}
