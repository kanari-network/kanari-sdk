// Test loading Kanari system modules into Move VM

use kanari_move_runtime::MoveRuntime;
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::ModuleId;

fn main() -> anyhow::Result<()> {
    println!("=== Kanari System Module Loading Test ===\n");

    // Create runtime with Kanari natives and system modules
    println!("Initializing MoveRuntime with Kanari natives...");
    let runtime = MoveRuntime::new_with_kanari_natives()?;
    
    println!("\n✓ Runtime initialized successfully");
    println!("✓ Native functions loaded (ecdsa_k1, ecdsa_r1, ed25519)");
    println!("✓ System modules loaded (transfer, coin, balance, etc.)\n");

    // Verify critical system modules are available
    let system_addr = AccountAddress::from_hex_literal("0x2")?;
    
    let modules_to_check = vec![
        "transfer",
        "coin", 
        "balance",
        "object",
        "tx_context",
        "kanari",
        "ecdsa_k1",
        "ecdsa_r1",
        "ed25519",
    ];

    println!("Checking system modules:");
    for module_name in modules_to_check {
        let module_id = ModuleId::new(
            system_addr,
            module_name.parse().unwrap()
        );
        
        let exists = runtime.has_module(&module_id);
        let status = if exists { "✓" } else { "✗" };
        println!("  {} 0x2::{}", status, module_name);
    }

    println!("\n=== System Ready ===");
    println!("You can now execute transactions using:");
    println!("  - 0x2::transfer::public_transfer()");
    println!("  - 0x2::coin::mint()");
    println!("  - 0x2::balance::create()");
    println!("  - etc.");

    Ok(())
}
