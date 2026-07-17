use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn watch_package_sources(path: &Path) {
    let entries = fs::read_dir(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some("build") {
            continue;
        }
        if path.is_dir() {
            watch_package_sources(&path);
        } else if matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("move" | "toml" | "lock")
        ) {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

fn compile_modules(package: &Path, output: &Path) -> Vec<PathBuf> {
    fs::create_dir_all(output)
        .unwrap_or_else(|error| panic!("cannot create {}: {error}", output.display()));

    // Keep Move compiler output inside Cargo's OUT_DIR. A clean checkout must not
    // depend on ignored package/build directories, and build scripts must not
    // mutate the source tree (especially during cross compilation).
    let install_dir = output.join("compiled-package");
    let config = move_package::BuildConfig {
        install_dir: Some(install_dir),
        ..Default::default()
    };
    let compiled = config
        .compile_package(package, &mut std::io::sink())
        .unwrap_or_else(|error| panic!("cannot compile {}: {error}", package.display()));

    let mut paths = Vec::new();
    for module in compiled.root_modules() {
        let module_name = module.unit.module.self_id().name().to_string();
        let path = output.join(format!("{module_name}.mv"));
        let mut bytes = Vec::new();
        module
            .unit
            .module
            .serialize(&mut bytes)
            .unwrap_or_else(|error| panic!("cannot serialize {module_name}: {error}"));
        fs::write(&path, bytes)
            .unwrap_or_else(|error| panic!("cannot write {}: {error}", path.display()));
        paths.push(path);
    }

    assert!(
        !paths.is_empty(),
        "Move package {} produced no root modules",
        package.display()
    );
    paths.sort();
    paths
}

fn emit_modules(out: &mut String, constant: &str, paths: &[PathBuf]) {
    out.push_str(&format!(
        "pub(crate) const {constant}: &[(&str, &[u8])] = &[\n"
    ));
    for path in paths {
        let name = path
            .file_name()
            .expect("compiled module path must have a file name")
            .to_string_lossy();
        out.push_str(&format!(
            "    ({name:?}, include_bytes!({path:?})),\n",
            name = name,
            path = path.display(),
        ));
    }
    out.push_str("];\n");
}

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let frameworks = root.join("crates/kanari-frameworks/packages");
    let stdlib = frameworks.join("move-stdlib");
    let system = frameworks.join("kanari-system");
    watch_package_sources(&stdlib);
    watch_package_sources(&system);

    let output = PathBuf::from(env::var("OUT_DIR").unwrap());
    let stdlib_modules = compile_modules(&stdlib, &output.join("move-stdlib"));
    let system_modules = compile_modules(&system, &output.join("kanari-system"));

    let mut generated = String::new();
    emit_modules(
        &mut generated,
        "EMBEDDED_MOVE_STDLIB_MODULES",
        &stdlib_modules,
    );
    emit_modules(
        &mut generated,
        "EMBEDDED_KANARI_SYSTEM_MODULES",
        &system_modules,
    );
    fs::write(output.join("embedded_framework_modules.rs"), generated)
        .expect("cannot write embedded framework module manifest");
}
