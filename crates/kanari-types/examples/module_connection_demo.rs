//! Example: Demonstrating connection between Rust and Move modules
//!
//! ตัวอย่างแสดงการเชื่อมต่อระหว่าง Rust types และ Move modules
//!
//! Run with: cargo run --example module_connection_demo

use kanari_types::stdlib::*;
use kanari_types::*;

fn main() {
    println!("🚀 Kanari Types - Move Module Connection Demo");
    println!("{}", "=".repeat(60));

    // 1. Show all Kanari System modules
    println!("\n📦 Kanari System Modules (0x2::<module>):");
    print_module_info("balance", &balance::BalanceModule::get_module_id());
    print_module_info("coin", &coin::CoinModule::get_module_id());
    print_module_info("kanari", &kanari::KanariModule::get_module_id());
    print_module_info("transfer", &transfer::TransferModule::get_module_id());
    print_module_info("tx_context", &tx_context::TxContextModule::get_module_id());

    // 2. Show all Move Stdlib modules
    println!("\n📚 Move Standard Library Modules (0x1::<module>):");
    print_module_info("ascii", &ascii::AsciiModule::get_module_id());
    print_module_info("error", &error::ErrorModule::get_module_id());
    print_module_info("option", &option::OptionModule::get_module_id());
    print_module_info("signer", &signer::SignerModule::get_module_id());
    print_module_info("string", &string::StringModule::get_module_id());
    print_module_info("vector", &vector::VectorModule::get_module_id());

    // 3. Demonstrate Balance module
    println!("\n💰 Balance Module Demo:");
    let mut balance = balance::BalanceRecord::new(1000);
    println!("   Initial balance: {}", balance.value);
    balance.increase(500).unwrap();
    println!("   After increase(500): {}", balance.value);
    balance.decrease(300).unwrap();
    println!("   After decrease(300): {}", balance.value);

    let balance_fns = balance::BalanceModule::function_names();
    println!(
        "   Available functions: zero, {}, {}, {}, ...",
        balance_fns.increase, balance_fns.decrease, balance_fns.split
    );

    // 4. Demonstrate Coin module
    println!("\n🪙 Coin Module Demo:");
    let coin = coin::CoinRecord::new(5000);
    println!("   Coin value: {}", coin.value());

    let metadata = coin::CurrencyMetadata::new(
        b"KANARI".to_vec(),
        b"Kanari Coin".to_vec(),
        b"Native token of Kanari blockchain".to_vec(),
        9u8,
        None,
    );
    println!("   Currency: {}", metadata.symbol_str().unwrap());
    println!("   Name: {}", metadata.name_str().unwrap());

    let coin_fns = coin::CoinModule::function_names();
    // `increase_supply` has moved from `coin` to the `balance` module.
    let balance_fns = balance::BalanceModule::function_names();
    println!(
        "   Available functions: {}, {} (supply helpers in balance: {}), ...",
        coin_fns.create_currency, coin_fns.treasury_into_supply, balance_fns.increase_supply
    );

    // 5. Demonstrate Kanari module
    println!("\n🦅 Kanari Module Demo:");
    println!(
        "   MIST_PER_KANARI: {}",
        kanari::KanariModule::MIST_PER_KANARI
    );
    println!(
        "   TOTAL_SUPPLY_KANARI: {}",
        kanari::KanariModule::TOTAL_SUPPLY_KANARI
    );

    let one_kanari = kanari::KanariModule::kanari_to_mist(1);
    println!("   1 KANARI = {} MIST", one_kanari);

    let formatted = kanari::KanariModule::format_kanari(1_500_000_000);
    println!("   1,500,000,000 MIST = {}", formatted);

    // 6. Demonstrate Transfer module
    println!("\n💸 Transfer Module Demo:");
    let transfer = transfer::TransferRecord::from_hex_literals("0x1", "0x2", 1000).unwrap();
    println!("   From: {}", transfer.from);
    println!("   To: {}", transfer.to);
    println!("   Amount: {}", transfer.amount);

    let transfer_fns = transfer::TransferModule::function_names();
    println!(
        "   Available functions: {}, {}, {}, ...",
        transfer_fns.create_transfer, transfer_fns.get_amount, transfer_fns.get_from
    );

    // 7. Demonstrate TxContext module
    println!("\n📝 Transaction Context Module Demo:");
    let tx_hash = vec![1, 2, 3, 4];
    let tx_ctx = tx_context::TxContextRecord::new(
        "0xabcd".to_string(),
        tx_hash.clone(),
        100,
        1_650_000_000,
        50,
    );
    println!("   Sender: {}", tx_ctx.sender());
    println!("   Epoch: {}", tx_ctx.epoch());
    println!("   Tx Hash: {:?}", tx_ctx.tx_hash());
    println!("   Epoch Timestamp (ms): {}", tx_ctx.epoch_timestamp_ms());
    println!("   IDs Created: {}", tx_ctx.ids_created());

    // 8. Demonstrate stdlib modules
    println!("\n📖 Move Stdlib Modules Demo:");

    // ASCII
    let ascii_str = ascii::AsciiString::from_str("Hello").unwrap();
    println!(
        "   ASCII String: '{}' (length: {})",
        ascii_str.to_string().unwrap(),
        ascii_str.length()
    );

    // String (UTF-8)
    let utf8_str = string::Utf8String::from_str("สวัสดี KANARI! 🦅");
    println!(
        "   UTF-8 String: '{}' (bytes: {})",
        utf8_str.to_string().unwrap(),
        utf8_str.length()
    );

    // Option
    let some_value = option::OptionValue::some(42);
    println!("   Option::Some(42): is_some={}", some_value.is_some());

    let none_value: option::OptionValue<i32> = option::OptionValue::none();
    println!("   Option::None: is_none={}", none_value.is_none());

    // Signer
    let signer = signer::SignerRef::new("0x123".to_string());
    println!("   Signer address: {}", signer.address());

    // Error categories
    println!("   Error categories:");
    println!(
        "      - INVALID_ARGUMENT: {}",
        error::ErrorModule::INVALID_ARGUMENT
    );
    println!("      - NOT_FOUND: {}", error::ErrorModule::NOT_FOUND);
    println!(
        "      - PERMISSION_DENIED: {}",
        error::ErrorModule::PERMISSION_DENIED
    );

    // 9. Show complete workflow
    println!("\n🔄 Complete Workflow Example:");
    println!("   1. Create balance: 10,000 MIST");
    let mut workflow_balance = balance::BalanceRecord::new(10_000);

    println!("   2. Create transfer: 1,000 MIST from 0x1 to 0x2");
    let workflow_transfer =
        transfer::TransferRecord::from_hex_literals("0x1", "0x2", 1_000).unwrap();

    println!("   3. Decrease balance by transfer amount");
    workflow_balance.decrease(workflow_transfer.amount).unwrap();

    println!(
        "   4. Final balance: {} MIST ({})",
        workflow_balance.value,
        kanari::KanariModule::format_kanari(workflow_balance.value as u64)
    );

    println!("\n✅ All modules connected successfully!");
    println!("{}", "=".repeat(60));
    println!("\n💡 Key Points:");
    println!("   • Kanari system modules: 0x2::balance, 0x2::coin, etc.");
    println!("   • Move stdlib modules: 0x1::ascii, 0x1::option, etc.");
    println!("   • All modules have ModuleId for Move VM integration");
    println!("   • Function names match Move module functions");
    println!("   • Data structures are serializable (BCS/JSON)");
    println!("   • Ready for Move VM execution! 🚀");
}

fn print_module_info(
    name: &str,
    result: &anyhow::Result<move_core_types::language_storage::ModuleId>,
) {
    match result {
        Ok(module_id) => {
            println!(
                "   ✅ {} -> {}::{}",
                name,
                module_id.address(),
                module_id.name()
            );
        }
        Err(e) => {
            println!("   ❌ {} -> Error: {}", name, e);
        }
    }
}
