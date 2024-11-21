use std::io;
use colored::Colorize;
use key::{generate_karix_address, list_wallet_files, load_wallet, save_wallet, send_coins, };
use k2::blockchain::{load_blockchain, BALANCES};
use std::process::exit;


pub fn handle_keytool_command() -> Option<String> {
    // Collect command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check if any arguments were provided
    if args.len() > 2 {
        // Collect command line arguments
        let command = &args[2];
        // Use string comparison in the match statement
        match command.as_str() {
            "generate" => { 
                println!("Enter mnemonic length (12 or 24):");
                let mut mnemonic_length_str = String::new();
                io::stdin().read_line(&mut mnemonic_length_str).unwrap();
                let mnemonic_length: usize = mnemonic_length_str.trim().parse().expect("Invalid input");

                let (private_key, public_address, seed_phrase) = generate_karix_address(mnemonic_length);
                println!("New address generated:");
                println!("Private Key: {}", private_key.green());
                println!("Public Address: {}", public_address.green());
                println!("Seed Phrase: {}", seed_phrase.green());

                save_wallet(&public_address, &private_key, &seed_phrase);

                return Some(public_address);
            },
            "balance" => { // String comparison for "balance"
                println!("Enter public address:");
                let mut public_address = String::new();
                io::stdin().read_line(&mut public_address).unwrap();
                let public_address = public_address.trim().to_string();

                load_blockchain();

                let balance = unsafe {
                    if let Some(balances) = BALANCES.as_ref() {
                        balances.lock().unwrap().get(&public_address).cloned().unwrap_or(0)
                    } else {
                        println!("Balances data is not available.");
                        return None;
                    }
                };

                println!("Balance for {}: {}", public_address.green(), balance.to_string().green());
                return None; // Return None to indicate no address to be used further
            },
            "wallet" => { // String comparison for "wallet"
                println!("Enter public address to load:");
                let mut public_address = String::new();
                io::stdin().read_line(&mut public_address).unwrap();
                let public_address = public_address.trim().to_string();

                if let Some(wallet_data) = load_wallet(&public_address) {
                    println!("Wallet loaded:");
                    println!("Address: {}", wallet_data["address"].as_str().unwrap().green());
                    println!("Private Key: {}", wallet_data["private_key"].as_str().unwrap().green());
                    println!("Seed Phrase: {}", wallet_data["seed_phrase"].as_str().unwrap().green());
                    return Some(public_address);
                } else {
                    println!("Wallet not found for address: {}", public_address.red());
                    return None; // Return None to indicate no address to be used further
                }
            },
            "send" => {
                println!("Enter sender public address:");
                let mut sender_address = String::new();
                io::stdin().read_line(&mut sender_address).unwrap();
                let sender_address = sender_address.trim().to_string();

                println!("Enter receiver public address:");
                let mut receiver_address = String::new();
                io::stdin().read_line(&mut receiver_address).unwrap();
                let receiver_address = receiver_address.trim().to_string();

                println!("Enter amount to send:");
                let mut amount_str = String::new();
                io::stdin().read_line(&mut amount_str).unwrap();
                let amount: u64 = amount_str.trim().parse().expect("Invalid input");

                load_blockchain();

                let result = send_coins(&sender_address, &receiver_address, amount);
                match result {
                    Ok(tx_hash) => {
                        println!("Transaction successful. Tx Hash: {}", tx_hash.green());
                    },
                    Err(e) => {
                        println!("Transaction failed: {}", e.red());
                    }
                }
                return None; // Return None to indicate no address to be used further
            }
            "list" => {
                match list_wallet_files() {
                    Ok(wallets) => {
                        println!("\nAvailable Wallets:");
                        println!("------------------");
                        for (wallet_name, is_selected) in wallets {
                            let status_symbol = if is_selected { "✓ " } else { "  " };
                            let wallet_display = wallet_name.trim_end_matches(".json");
                            if is_selected {
                                println!("{}{}", status_symbol, wallet_display.green().bold());
                            } else {
                                println!("{}{}", status_symbol, wallet_display);
                            }
                        }
                        println!("------------------");
                    },
                    Err(e) => {
                        println!("{}Failed to list wallet files: {}", "ERROR: ".red().bold(), e);
                    }
                }
                return None;
            },
            _ => {
                println!("{}", "Invalid command".red());
                println!("Usage: kari keytool <command> [options]");
                println!("Commands:");
                println!("  {} - Generate new address", "generate".green());
                println!("  {} - Check balance", "balance".green());
                println!("  {} - Load existing wallet", "wallet".green());
                println!("  {} - Send coins", "send".green());
                println!("  {} - List wallet files", "list".green());
                exit(1); // Exit with an error code
        
            },
        }
    } else {
        // No command provided, print usage
        println!("Usage: kari keytool <command> [options]");
        println!("Commands:");
        println!("  {} - Generate new address", "generate".green());
        println!("  {} - Check balance", "balance".green());
        println!("  {} - Load existing wallet", "wallet".green());
        println!("  {} - Send coins", "send".green());
        println!("  {} - List wallet files", "list".green());
        exit(1); // Exit with an error code
    }
}
