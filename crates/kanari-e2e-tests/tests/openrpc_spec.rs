//! ระบบ OpenRPC spec: สัญญา API ต้องครบ + ตรงกับไฟล์ที่เช็กอินไว้

use kanari_open_rpc::{OPENRPC_VERSION, Project, method, param, result, schema_string};
use kanari_open_rpc_spec_builder::{Action, build_kanari_rpc_spec, run_action};
use kanari_rpc_api::methods;

#[test]
fn spec_contains_core_methods() {
    let spec = build_kanari_rpc_spec();
    assert_eq!(spec.openrpc, OPENRPC_VERSION);

    let names: Vec<&str> = spec.methods.iter().map(|m| m.name).collect();
    for required in [
        methods::GET_OWNER,
        methods::GET_STATS,
        methods::HEALTH,
        methods::VIEW_FUNCTION,
        methods::GET_OBJECT,
    ] {
        assert!(names.contains(&required), "spec ต้องมี method `{required}`");
    }
}

#[test]
fn recorded_spec_check_passes() {
    // เทียบกับ schemas/openrpc.json ที่เช็กอินไว้ (ข้ามถ้าไฟล์ยังไม่มี)
    let path = kanari_open_rpc_spec_builder::spec_file();
    if !path.exists() {
        eprintln!("SKIP: recorded spec ยังไม่มีที่ {}", path.display());
        return;
    }
    run_action(Action::Test).expect("recorded spec ต้องตรงกับ code");
}

#[test]
fn project_builder_roundtrips_through_json() {
    let mut project = Project::new(
        "0.0.0-test",
        "e2e",
        "e2e test project",
        "Kanari",
        "https://kanarinetwork.site",
        "test@kanarinetwork.site",
        "Apache-2.0",
        "https://example.com/LICENSE",
    );
    project.add_method(method(
        "kanari_e2ePing",
        "ping",
        None,
        vec![param("input", "echo input", true, schema_string())],
        result("output", "echo output", schema_string()),
        &["e2e"],
    ));

    let value = serde_json::to_value(&project).unwrap();
    assert_eq!(value["openrpc"], OPENRPC_VERSION);
    assert_eq!(value["methods"][0]["name"], "kanari_e2ePing");
}

#[test]
fn recorded_spec_file_is_valid_json_when_present() {
    let path = kanari_open_rpc_spec_builder::spec_file();
    if !path.exists() {
        eprintln!("SKIP: recorded spec ยังไม่มีที่ {}", path.display());
        return;
    }
    let recorded = kanari_open_rpc_spec::read_recorded_spec().unwrap();
    assert_eq!(recorded["openrpc"], OPENRPC_VERSION);
}
