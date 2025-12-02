use anyhow::Result;
use kanari_types::module_registry::{ModuleCallBuilder, ModuleRegistry};

fn main() -> Result<()> {
    println!("=== Kanari Module Registry Demo ===\n");

    // 1. List all available modules
    println!("📦 Available Modules:");
    for module in ModuleRegistry::all_modules() {
        println!("  - {}", module);
    }
    println!();

    // 2. Get detailed info for each module
    println!("📋 Module Information:\n");
    for info in ModuleRegistry::all_modules_info() {
        println!("{}", info.display());
        println!();
    }

    // 3. Check if specific modules exist
    println!("✅ Module Existence Checks:");
    println!(
        "  kanari exists: {}",
        ModuleRegistry::module_exists("kanari")
    );
    println!("  coin exists: {}", ModuleRegistry::module_exists("coin"));
    println!(
        "  invalid exists: {}",
        ModuleRegistry::module_exists("invalid")
    );
    println!();

    // 4. Check if specific functions exist
    println!("🔍 Function Existence Checks:");
    println!(
        "  kanari::transfer exists: {}",
        ModuleRegistry::function_exists("kanari", "transfer")
    );
    println!(
        "  coin::mint exists: {}",
        ModuleRegistry::function_exists("coin", "mint")
    );
    println!(
        "  coin::invalid exists: {}",
        ModuleRegistry::function_exists("coin", "invalid")
    );
    println!();

    // 5. Get function identifiers
    println!("🎯 Function Identifiers:");
    if let Some(id) = ModuleRegistry::get_function_identifier("kanari", "transfer") {
        println!("  {}", id);
    }
    if let Some(id) = ModuleRegistry::get_function_identifier("coin", "mint") {
        println!("  {}", id);
    }
    if let Some(id) = ModuleRegistry::get_function_identifier("transfer", "execute") {
        println!("  {}", id);
    }
    println!();

    // 6. Get module IDs (for Move VM calls)
    println!("🆔 Module IDs:");
    for module_name in ModuleRegistry::all_modules() {
        let module_id = ModuleRegistry::get_module_id(module_name)?;
        println!(
            "  {} -> {}::{}",
            module_name,
            module_id.address(),
            module_id.name()
        );
    }
    println!();

    // 7. Use ModuleCallBuilder for validation
    println!("🔨 Module Call Builder Examples:");

    // Valid call
    let builder = ModuleCallBuilder::new("kanari").function("transfer");
    match builder.validate() {
        Ok(_) => {
            let identifier = builder.build_identifier()?;
            println!("  ✅ Valid call: {}", identifier);
        }
        Err(e) => println!("  ❌ Error: {}", e),
    }

    // Invalid module
    let builder = ModuleCallBuilder::new("invalid_module").function("test");
    match builder.validate() {
        Ok(_) => println!("  ✅ Valid call"),
        Err(e) => println!("  ❌ Invalid module: {}", e),
    }

    // Invalid function
    let builder = ModuleCallBuilder::new("kanari").function("invalid_function");
    match builder.validate() {
        Ok(_) => println!("  ✅ Valid call"),
        Err(e) => println!("  ❌ Invalid function: {}", e),
    }
    println!();

    // 8. Create function map
    println!("🗺️  Function Map:");
    let function_map = ModuleRegistry::create_function_map();
    for (module, functions) in function_map.iter().take(2) {
        println!("  {} has {} functions:", module, functions.len());
        for func in functions.iter().take(3) {
            println!("    - {}", func);
        }
        if functions.len() > 3 {
            println!("    ... and {} more", functions.len() - 3);
        }
    }
    println!();

    // 9. Get specific module info
    println!("📊 Specific Module Info:");
    if let Some(info) = ModuleRegistry::get_module_info("coin") {
        println!("  Module: {}", info.name);
        println!("  Address: {}", info.address);
        println!("  Function count: {}", info.function_count);
        println!("  Functions:");
        for func in &info.functions {
            println!("    - {}", func);
        }
    }

    Ok(())
}
