use std::io;
use colored::Colorize;
use key::{generate_karix_address, list_wallet_files, load_wallet, save_wallet, set_selected_wallet,  };

use k2::blockchain::{get_balance, load_blockchain, transfer_coins};
use std::process::exit;


struct CommandInfo {
    name: &'static str,
    description: &'static str,
}

const COMMANDS: &[CommandInfo] = &[
    CommandInfo { name: "generate", description: "Generate new address" },
    CommandInfo { name: "balance", description: "Check balance" },
    CommandInfo { name: "select", description: "Select wallet" },
    CommandInfo { name: "wallet", description: "Load existing wallet" },
    CommandInfo { name: "send", description: "Send coins" },
    CommandInfo { name: "list", description: "List wallet files" },
];

fn display_help(show_error: bool) {
    if show_error {
        println!("\n{}", "ERROR: Invalid command".red().bold());
    }

    // Usage section
    println!("{}", "USAGE:".bright_yellow().bold());
    println!("  kari keytool <command> [options]\n");

    // Commands section
    println!("{}", "COMMANDS:".bright_yellow().bold());
    
    let max_name_len = COMMANDS.iter().map(|cmd| cmd.name.len()).max().unwrap_or(0);
    
    for cmd in COMMANDS {
        println!(
            "  {}{}  {}", 
            cmd.name.green().bold(),
            " ".repeat(max_name_len - cmd.name.len() + 2),
            cmd.description.bright_white()
        );
    }
    println!();
    
    exit(1);
}


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
            "balance" => {
                println!("Enter public address:");
                let mut public_address = String::new();
                match io::stdin().read_line(&mut public_address) {
                    Ok(_) => {
                        let public_address = public_address.trim().to_string();
                        
                        match load_blockchain() {
                            Ok(_) => {
                                match get_balance(&public_address) {
                                    Ok(balance) => {
                                        println!("Balance for {}: {}", 
                                            public_address.green(), 
                                            balance.to_string().green()
                                        );
                                    },
                                    Err(e) => {
                                        println!("{}: {}", "Error getting balance".red(), e);
                                    }
                                }
                            },
                            Err(e) => {
                                println!("{}: {}", "Error loading blockchain".red(), e);
                            }
                        }
                    },
                    Err(e) => {
                        println!("{}: {}", "Error reading input".red(), e);
                    }
                }
                return None;
            },

            "select" => {
                match list_wallet_files() {
                    Ok(wallets) => {
                        if wallets.is_empty() {
                            println!("{}", "No wallets found!".red());
                            return None;
                        }
        
                        println!("\nAvailable wallets:");
                        for (i, (wallet, _)) in wallets.iter().enumerate() {
                            println!("{}. {}", i + 1, wallet.trim_end_matches(".toml"));
                        }
        
                        println!("\nEnter wallet number to select:");
                        let mut input = String::new();
                        let _ = io::stdin().read_line(&mut input);
                        
                        if let Ok(index) = input.trim().parse::<usize>() {
                            if index > 0 && index <= wallets.len() {
                                let selected = wallets[index - 1].0.trim_end_matches(".toml");
                                if let Err(e) = set_selected_wallet(selected) {
                                    println!("Error setting wallet: {}", e);
                                } else {
                                    println!("Selected wallet: {}", selected.green());
                                }
                            }
                        }
                    },
                    Err(e) => println!("Error listing wallets: {}", e),
                }
                None
            },

            "wallet" => { // String comparison for "wallet"
                println!("Enter public address to load:");
                let mut public_address = String::new();
                io::stdin().read_line(&mut public_address).unwrap();
                let public_address = public_address.trim().to_string();
            
                if let Some(wallet_data) = load_wallet(&public_address) {
                    println!("Wallet loaded:");
                    println!("Address: {}", wallet_data.address.green());
                    println!("Private Key: {}", wallet_data.private_key.green());
                    println!("Seed Phrase: {}", wallet_data.seed_phrase.green());
                    return Some(public_address);
                } else {
                    println!("Wallet not found for address: {}", public_address.red());
                    return None; // Return None to indicate no address to be used further
                }
            },
            
            "send" => {
                // Get sender
                println!("Enter sender address:");
                let mut sender = String::new();
                let _ = io::stdin().read_line(&mut sender);
                let sender = sender.trim().to_string();
            
                // Get receiver
                println!("Enter receiver address:");
                let mut receiver = String::new();
                let _ = io::stdin().read_line(&mut receiver);
                let receiver = receiver.trim().to_string();
            
                // Get amount
                println!("Enter amount to send:");
                let mut amount = String::new();
                let _ = io::stdin().read_line(&mut amount);
                let amount: u64 = match amount.trim().parse() {
                    Ok(num) => num,
                    Err(_) => {
                        println!("{}", "Invalid amount".red());
                        return None;
                    }
                };
            
                // Execute transfer
                match transfer_coins(sender.clone(), receiver.clone(), amount) {
                    Ok(_) => {
                        println!("{}", "Transaction successful!".green());
                        println!("Sent {} tokens from {} to {}", 
                            amount.to_string().green(),
                            sender.green(),
                            receiver.green()
                        );
                    },
                    Err(e) => {
                        println!("{}: {}", "Transaction failed".red(), e);
                    }
                }
                return None;
            },

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
                display_help(true);
                return None;
            },
        }
    } else {
        display_help(false);
        return None;
    }
}
