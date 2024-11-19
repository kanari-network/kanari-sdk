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
                    BALANCES.as_ref().unwrap().lock().unwrap().get(&public_address).cloned().unwrap_or(0)
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
                // Debug print args
                println!("Args received: {:?}", args);
                
                if args.len() != 6 {
                    println!("Usage: send <from_address> <to_address> <amount>");
                    return None;
                }
            
                let sender = args[3].clone();
                let receiver = args[4].clone();
                let amount = match args[5].parse::<u64>() {
                    Ok(value) => value,
                    Err(_) => {
                        println!("Invalid amount: must be a positive number");
                        return None;
                    }
                };
            
                // Debug print balance before transaction
                let sender_balance = unsafe {
                    BALANCES.as_ref()
                        .and_then(|b| b.lock().ok())
                        .map(|b| *b.get(&sender).unwrap_or(&0))
                        .unwrap_or(0)
                };
                println!("Sender balance before transfer: {}", sender_balance);
                println!("Attempting transfer of {} coins plus gas cost", amount);
            
                match send_coins(sender.clone(), receiver, amount) {
                    Some(transaction) => {
                        println!("Transaction successful!");
                        println!("Transaction details:");
                        println!("  From: {}", transaction.sender);
                        println!("  To: {}", transaction.receiver);
                        println!("  Amount: {}", transaction.amount);
                        println!("  Gas cost: {}", transaction.gas_cost);
                        println!("  Timestamp: {}", transaction.timestamp);
                        
                        // Debug print final balance
                        let final_balance = unsafe {
                            BALANCES.as_ref()
                                .and_then(|b| b.lock().ok())
                                .map(|b| *b.get(&sender).unwrap_or(&0))
                                .unwrap_or(0)
                        };
                        println!("Sender balance after transfer: {}", final_balance);
                        Some(sender)
                    }
                    None => {
                        println!("Transaction failed - insufficient funds or internal error");
                        None
                    }
                }
            }
            "list" => { // String comparison for "list"}
                match list_wallet_files() {
                    Ok(wallets) => {
                        println!("Wallet files:");
                        for wallet in wallets {
                            println!("{}", wallet.green());
                        }
                    },
                    Err(e) => {
                        println!("Failed to list wallet files: {}", e);
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
