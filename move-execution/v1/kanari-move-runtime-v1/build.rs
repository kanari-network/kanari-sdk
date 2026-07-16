use std::env;
use std::fs;
use std::path::PathBuf;

fn emit_modules(out: &mut String, constant: &str, directory: &PathBuf) {
    let mut paths = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()))
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("mv"))
        .collect::<Vec<_>>();
    paths.sort();
    out.push_str(&format!(
        "pub(crate) const {constant}: &[(&str, &[u8])] = &[\n"
    ));
    for path in paths {
        let name = path.file_name().unwrap().to_string_lossy();
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
    let stdlib = frameworks.join("move-stdlib/build/MoveStdlib/bytecode_modules");
    let system = frameworks.join("kanari-system/build/KanariSystem/bytecode_modules");
    println!("cargo:rerun-if-changed={}", stdlib.display());
    println!("cargo:rerun-if-changed={}", system.display());

    let mut generated = String::new();
    emit_modules(&mut generated, "EMBEDDED_MOVE_STDLIB_MODULES", &stdlib);
    emit_modules(&mut generated, "EMBEDDED_KANARI_SYSTEM_MODULES", &system);
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("embedded_framework_modules.rs"),
        generated,
    )
    .unwrap();
}
