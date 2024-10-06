use move_compiler::compiled_unit::CompiledUnit;
use move_compiler::shared::NumericalAddress;
use move_vm_runtime::move_vm::MoveVM;
use move_vm_test_utils::InMemoryStorage;
use move_core_types::language_storage::ModuleId;
use move_core_types::account_address::AccountAddress;
use move_vm_runtime::native_functions::NativeFunctionTable;
use std::collections::BTreeMap;
use std::path::Path;

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