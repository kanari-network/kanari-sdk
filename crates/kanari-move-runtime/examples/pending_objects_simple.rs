// Simple example: Using pending_objects without external modules
// This demonstrates the core API without requiring compiled Move modules

use anyhow::Result;
use kanari_move_runtime::MoveRuntime;
use move_core_types::account_address::AccountAddress;

fn main() -> Result<()> {
    println!("🚀 Simple Pending Objects Example\n");

    // 1. Create runtime (no external modules needed)
    println!("📦 Creating runtime...");
    let mut runtime = MoveRuntime::new_with_natives(vec![], false)?;
    println!("✅ Runtime created\n");

    // 2. Verify runtime starts with empty pending_objects
    println!("📊 Initial State:");
    let initial = runtime.get_pending_objects();
    println!("  Transfers: {}", initial.transfers.len());
    println!("  Freezes:   {}", initial.freezes.len());
    println!("  Shares:    {}", initial.shares.len());
    assert!(initial.is_empty(), "Expected empty pending_objects on init");
    println!("  ✅ Verified: pending_objects is empty\n");

    // 3. Manually add object operations (simulating what native functions would do)
    println!("➕ Adding object operations manually...\n");

    // Add NFT transfer
    let nft_id = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let nft_type = "0x2::nft::CoolNFT".to_string();
    let nft_data = b"NFT metadata".to_vec();
    let alice = AccountAddress::from_hex_literal("0xA11CE")?;

    runtime.add_pending_transfer(nft_id.clone(), nft_type.clone(), nft_data.clone(), alice);
    println!("  ✅ Added NFT transfer to Alice (0xA11CE)");

    // Add Token transfer
    let token_id = vec![9, 10, 11, 12, 13, 14, 15, 16];
    let token_type = "0x2::coin::Coin<0x2::kanari::KANARI>".to_string();
    let token_data = vec![100, 0, 0, 0, 0, 0, 0, 0]; // 100 tokens
    let bob = AccountAddress::from_hex_literal("0xB0B")?;

    runtime.add_pending_transfer(
        token_id.clone(),
        token_type.clone(),
        token_data.clone(),
        bob,
    );
    println!("  ✅ Added Token transfer to Bob (0xB0B)");

    // Add another NFT transfer
    let nft2_id = vec![17, 18, 19, 20, 21, 22, 23, 24];
    let nft2_type = "0x2::nft::RareNFT".to_string();
    let nft2_data = b"Rare NFT metadata".to_vec();
    let charlie = AccountAddress::from_hex_literal("0xCCC")?;

    runtime.add_pending_transfer(
        nft2_id.clone(),
        nft2_type.clone(),
        nft2_data.clone(),
        charlie,
    );
    println!("  ✅ Added Rare NFT transfer to Charlie (0xCCC)\n");

    // 4. Read pending operations (non-destructive)
    println!("📖 Reading pending operations (non-destructive):");
    let pending = runtime.get_pending_objects();
    println!("  Total operations: {}", pending.transfers.len());

    for (i, transfer) in pending.transfers.iter().enumerate() {
        println!("\n  🔄 Transfer #{}:", i + 1);
        println!("    Object ID:  {}", hex::encode(&transfer.object_id));
        println!("    Type:       {}", transfer.object_type);
        println!("    Recipient:  {}", transfer.recipient);
        println!("    Data size:  {} bytes", transfer.object_data.len());
    }
    println!();

    // 5. Verify get_pending_objects is non-destructive
    println!("🔍 Verifying non-destructive read...");
    let pending_again = runtime.get_pending_objects();
    assert_eq!(
        pending.transfers.len(),
        pending_again.transfers.len(),
        "get_pending_objects should not remove operations"
    );
    println!("  ✅ Confirmed: Operations still present after read\n");

    // 6. Demonstrate take_pending_objects (destructive)
    println!("📤 Taking pending operations (destructive)...");
    let taken = runtime.take_pending_objects();
    println!("  Took {} operations", taken.transfers.len());

    // Verify operations were cleared
    let after_take = runtime.get_pending_objects();
    assert!(
        after_take.is_empty(),
        "take_pending_objects should clear operations"
    );
    println!("  ✅ Confirmed: Operations cleared after take\n");

    // 7. Process the taken operations
    println!("⚙️  Processing taken operations:");
    for (i, transfer) in taken.transfers.iter().enumerate() {
        println!(
            "  Processing transfer #{}: {} -> {}",
            i + 1,
            transfer.object_type,
            transfer.recipient
        );
        // In real implementation, this would update blockchain state
    }
    println!();

    // 8. Add more operations after clear
    println!("➕ Adding new operations after clear...");
    runtime.add_pending_transfer(
        vec![25, 26, 27, 28],
        "0x2::token::GameToken".to_string(),
        vec![200, 0, 0, 0],
        AccountAddress::from_hex_literal("0xDDD")?,
    );
    println!("  ✅ Added new operation\n");

    // 9. Demonstrate clear_pending_objects
    println!("🧹 Clearing pending operations...");
    runtime.clear_pending_objects();

    let cleared = runtime.get_pending_objects();
    assert!(
        cleared.is_empty(),
        "clear_pending_objects should remove all"
    );
    println!("  ✅ All operations cleared\n");

    // 10. Summary
    println!("✨ Summary:");
    println!("   ✅ Created runtime with empty pending_objects");
    println!("   ✅ Added multiple object operations manually");
    println!("   ✅ Read operations without removing them (get)");
    println!("   ✅ Took operations and cleared them (take)");
    println!("   ✅ Processed operations");
    println!("   ✅ Cleared operations manually (clear)");
    println!();

    println!("🎯 Key Methods Demonstrated:");
    println!("   • add_pending_transfer() - Add operations manually");
    println!("   • get_pending_objects()  - Read without clearing");
    println!("   • take_pending_objects() - Read and clear");
    println!("   • clear_pending_objects() - Clear all operations");
    println!();

    println!("🎉 Example completed successfully!");

    Ok(())
}
