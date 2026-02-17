use kanari_move_runtime::move_runtime::MoveRuntime;

#[test]
fn publish_module_upgrade_preserves_storage() {
    let runtime = MoveRuntime::new_with_kanari_natives().expect("init runtime");
    let modules = runtime.list_modules();
    assert!(!modules.is_empty(), "expected at least one system module");

    // Find a module which has persisted bytes in the runtime state
    let maybe = modules
        .into_iter()
        .find_map(|m| runtime.get_module_bytes(&m).map(|b| (m, b)));
    let (module_id, module_bytes) = match maybe {
        Some(pair) => pair,
        None => panic!("no preloaded module had persisted bytes in state"),
    };

    runtime
        .publish_module(module_bytes.clone(), *module_id.address(), None, None)
        .expect("publish (upgrade) should succeed");

    // Ensure storage contains the module bytes after publish
    let stored = runtime
        .get_module_bytes(&module_id)
        .expect("module bytes present after publish");
    assert_eq!(stored, module_bytes);

    // Confirm module_id still present in listed modules
    let listed = runtime.list_modules();
    assert!(listed.iter().any(|m| m == &module_id));

    runtime
        .publish_module(module_bytes.clone(), *module_id.address(), None, None)
        .expect("second publish should succeed");

    let stored2 = runtime
        .get_module_bytes(&module_id)
        .expect("module bytes present after second publish");
    assert_eq!(stored2, module_bytes);
}
