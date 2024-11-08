use move_compiler::compiled_unit::CompiledUnit;
use move_compiler::shared::NumericalAddress;
use move_vm_runtime::move_vm::MoveVM;
use move_vm_test_utils::InMemoryStorage;
use move_core_types::language_storage::ModuleId;
use move_core_types::account_address::AccountAddress;
use move_vm_runtime::native_functions::NativeFunctionTable;
use std::collections::BTreeMap;
use std::path::Path;
use std::io::Write;
use std::fs;

pub fn compile_move_script(script_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(script_path);
    let (compiled_units, _) = move_compiler::Compiler::from_files(
        None,
        vec![path.to_str().unwrap().to_string()],
        vec![],
        BTreeMap::<String, NumericalAddress>::new(),
    ).build()?;
    for unit in compiled_units {
        println!("Compiled unit: {:?}", unit);
    }
    Ok(())
}
// move_vm_runtime::move_vm::MoveVM
pub fn execute_move_script(
    script_path: &str,
    sender: AccountAddress,
    natives: NativeFunctionTable,
) -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let vm = MoveVM::new(natives)?;
    let path = Path::new(script_path);
    let (compiled_units, _) = move_compiler::Compiler::from_files(
        None,
        vec![path.to_str().unwrap().to_string()],
        vec![],
        BTreeMap::<String, NumericalAddress>::new(),
    ).build()?;
    for unit in compiled_units {

    }
    Ok(())
}


pub fn create_move_project(name: &str) -> std::io::Result<()> {
    let project_dir = Path::new(name);
    let sources_dir = project_dir.join("sources");
    let tests_dir = project_dir.join("tests");

    // Create directories
    fs::create_dir_all(&sources_dir)?;
    fs::create_dir_all(&tests_dir)?;

    // Create a sample Move file
    let move_file_path = sources_dir.join(format!("{}.move", name));
    let mut move_file = fs::File::create(move_file_path)?;
    writeln!(move_file, "// Sample Move file for {}", name)?;

    // Create Move.toml file
    let move_toml_path = project_dir.join("Move.toml");
    let mut move_toml_file = fs::File::create(move_toml_path)?;
    writeln!(move_toml_file, r#"[package]
        name = "kanari_network"
        version = "0.0.1"
        license = "Academic Free License v3.0"
        authors = ["James Atomc (co-founder@kanari.network)"]
        published-at = "0xedb4864f8021cb6191028e0389312f058104bca6b68667789177db5f98ebae19"
        edition = "2024.beta"

        [dependencies]
        Sui = {{ git = "https://github.com/MystenLabs/sui.git", subdir = "crates/sui-framework/packages/sui-framework", rev = "framework/testnet" }}
        MoveStdlib = {{ git = "https://github.com/MystenLabs/sui.git", subdir = "crates/sui-framework/packages/move-stdlib", rev = "framework/testnet" }}
        DeepBook = {{ git = "https://github.com/MystenLabs/sui.git", subdir = "crates/sui-framework/packages/deepbook", rev = "framework/testnet" }}

        [addresses]
        kanari_network = "0x0"
        std = "0x1"
        sui = "0x2"
        deepbook = "0xdee9"
"#)?;

    println!("Move project '{}' created successfully.", name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_core_types::account_address::AccountAddress;
    use move_vm_runtime::native_functions::NativeFunctionTable;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_compile_move_script() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_script.move");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "script {{ module 0x42::test {{
    struct Example has copy, drop {{ i: u64 }}

    use std::debug;
    friend 0x42::another_test;

    const ONE: u64 = 1;

    public fun print(x: u64) {{
        let sum = x + ONE;
        let example = Example {{ i: sum }};
        debug::print(&sum)
    }}
}}
 }}").unwrap();

        let result = compile_move_script(file_path.to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_move_script() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_script.move");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "script {{ module 0x42::test {{
    struct Example has copy, drop {{ i: u64 }}

    use std::debug;
    friend 0x42::another_test;

    const ONE: u64 = 1;

    public fun print(x: u64) {{
        let sum = x + ONE;
        let example = Example {{ i: sum }};
        debug::print(&sum)
    }}
}}
 }}").unwrap();

        let sender = AccountAddress::random();
        let natives = NativeFunctionTable::new();
        let result = execute_move_script(file_path.to_str().unwrap(), sender, natives);
        assert!(result.is_ok());
    }
}