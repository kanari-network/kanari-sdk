use colored::Colorize;
use key::{
    generate_karix_address, import_from_private_key, import_from_seed_phrase, list_wallet_files,
    load_wallet, save_wallet, set_selected_wallet,
};
use std::io::{self, Write};

use k2::blockchain::{get_balance, load_blockchain_with_retry};
use rpassword::read_password;
use std::process::exit;

struct CommandInfo {
    name: &'static str,
    description: &'static str,
}

const COMMANDS: &[CommandInfo] = &[
    CommandInfo {
        name: "generate",
        description: "Generate new address",
    },
    CommandInfo {
        name: "balance",
        description: "Check balance",
    },
    CommandInfo {
        name: "transfer",
        description: "Transfer coins to another address",
    },
    CommandInfo {
        name: "select",
        description: "Select wallet",
    },
    CommandInfo {
        name: "wallet",
        description: "Load existing wallet",
    },
    CommandInfo {
        name: "list",
        description: "List wallet files",
    },
    CommandInfo {
        name: "import",
        description: "Import from seed phrase",
    },
    CommandInfo {
        name: "privatekey",
        description: "Import from private key",
    },
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
                match io::stdin().read_line(&mut mnemonic_length_str) {
                    Ok(_) => {
                        match mnemonic_length_str.trim().parse::<usize>() {
                            Ok(mnemonic_length) => {
                                if mnemonic_length != 12 && mnemonic_length != 24 {
                                    println!(
                                        "{}",
                                        "Invalid mnemonic length. Must be 12 or 24.".red()
                                    );
                                    return None;
                                }

                                let (private_key, public_address, seed_phrase) =
                                    generate_karix_address(mnemonic_length);
                                println!("New address generated:");
                                println!("Private Key: {}", private_key.green());
                                println!("Public Address: {}", public_address.green());
                                println!("Seed Phrase: {}", seed_phrase.green());

                                let password = prompt_password(true);
                                // Convert public_address to Address type
                                match public_address.parse() {
                                    Ok(address) => {
                                        match save_wallet(
                                            &address,
                                            &private_key,
                                            &seed_phrase,
                                            &password,
                                        ) {
                                            Ok(_) => {
                                                println!("Wallet saved successfully!");
                                                return Some(public_address);
                                            }
                                            Err(e) => {
                                                println!(
                                                    "{}",
                                                    format!("Failed to save wallet: {}", e).red()
                                                );
                                                return None;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!(
                                            "{}",
                                            format!("Failed to parse public address: {}", e).red()
                                        );
                                        return None;
                                    }
                                }
                            }
                            Err(_) => {
                                println!("{}", "Invalid input - please enter 12 or 24".red());
                                return None;
                            }
                        }
                    }
                    Err(e) => {
                        println!("{}", format!("Failed to read input: {}", e).red());
                        return None;
                    }
                }
            }

            "balance" => {
                println!("Enter public address:");
                let mut public_address = String::new();
                match io::stdin().read_line(&mut public_address) {
                    Ok(_) => {
                        let public_address = public_address.trim().to_string();
                        
                        // Use a more reliable approach
                        match load_blockchain_with_retry() {
                            Ok(_) => match get_balance(&public_address) {
                                Ok(balance) => {
                                    println!(
                                        "Balance for {}: {}",
                                        public_address.green(),
                                        balance.to_string().green()
                                    );
                                }
                                Err(e) => {
                                    println!("{}: {}", "Error getting balance".red(), e);
                                }
                            },
                            Err(e) => {
                                println!("{}: {}", "Error loading blockchain".red(), e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("{}: {}", "Error reading input".red(), e);
                    }
                }
                return None;
            }

            "select" => match list_wallet_files() {
                Ok(wallets) => {
                    if wallets.is_empty() {
                        println!("{}", "No wallets found!".red());
                        return None;
                    }

                    println!("\nAvailable wallets:");
                    for (i, (wallet, is_selected)) in wallets.iter().enumerate() {
                        let wallet_name = wallet.trim_end_matches(".enc");
                        if *is_selected {
                            println!("{}. {} {}", i + 1, wallet_name, "(current)".green());
                        } else {
                            println!("{}. {}", i + 1, wallet_name);
                        }
                    }

                    println!("\nEnter wallet number to select (or press Enter to cancel):");
                    let mut input = String::new();
                    match io::stdin().read_line(&mut input) {
                        Ok(_) => {
                            if input.trim().is_empty() {
                                return None;
                            }

                            match input.trim().parse::<usize>() {
                                Ok(index) if index > 0 && index <= wallets.len() => {
                                    let selected = wallets[index - 1].0.trim_end_matches(".enc");
                                    match set_selected_wallet(selected) {
                                        Ok(_) => {
                                            println!(
                                                "{}",
                                                format!("Selected wallet: {}", selected).green()
                                            );
                                            Some(selected.to_string())
                                        }
                                        Err(e) => {
                                            println!(
                                                "{}",
                                                format!("Error setting wallet: {}", e).red()
                                            );
                                            None
                                        }
                                    }
                                }
                                _ => {
                                    println!("{}", format!("Invalid selection. Please enter a number between 1 and {}", wallets.len()).red());
                                    None
                                }
                            }
                        }
                        Err(e) => {
                            println!("{}", format!("Error reading input: {}", e).red());
                            None
                        }
                    }
                }
                Err(e) => {
                    println!("{}", format!("Error listing wallets: {}", e).red());
                    None
                }
            },

            "wallet" => {
                println!("Enter public address to load:");
                let mut public_address = String::new();
                match io::stdin().read_line(&mut public_address) {
                    Ok(_) => {
                        let public_address = public_address.trim().to_string();
                        let password = prompt_password(false);

                        match load_wallet(&public_address, &password) {
                            Ok(wallet_data) => {
                                println!("Wallet loaded:");
                                // Convert Address to String before applying green()
                                println!("Address: {}", wallet_data.address.to_string().green());
                                println!("Private Key: {}", wallet_data.private_key.green());
                                println!("Seed Phrase: {}", wallet_data.seed_phrase.green());
                                return Some(public_address);
                            }
                            Err(e) => {
                                println!("{}", format!("Failed to load wallet: {}", e).red());
                                return None;
                            }
                        }
                    }
                    Err(e) => {
                        println!("{}", format!("Failed to read input: {}", e).red());
                        return None;
                    }
                }
            }

            "list" => {
                match list_wallet_files() {
                    Ok(wallets) => {
                        println!("\nAvailable Wallets:");
                        println!("------------------");
                        for (wallet_name, is_selected) in wallets {
                            let status_symbol = if is_selected { "✓ " } else { "  " };
                            let wallet_display = wallet_name.trim_end_matches(".enc");
                            if is_selected {
                                println!("{}{}", status_symbol, wallet_display.green().bold());
                            } else {
                                println!("{}{}", status_symbol, wallet_display);
                            }
                        }
                        println!("------------------");
                    }
                    Err(e) => {
                        println!(
                            "{}Failed to list wallet files: {}",
                            "ERROR: ".red().bold(),
                            e
                        );
                    }
                }
                return None;
            }

            "import" => {
                println!("Enter seed phrase:");
                let mut phrase = String::new();
                io::stdin()
                    .read_line(&mut phrase)
                    .expect("Failed to read line");

                match import_from_seed_phrase(phrase.trim()) {
                    Ok((private_key, _, public_address)) => {
                        let password = prompt_password(true);

                        match public_address.parse() {
                            Ok(address) => {
                                match save_wallet(&address, &private_key, phrase.trim(), &password)
                                {
                                    Ok(_) => match set_selected_wallet(&public_address) {
                                        Ok(_) => {
                                            println!(
                                                "Imported wallet with address: {}",
                                                public_address
                                            );
                                            return Some(public_address);
                                        }
                                        Err(e) => {
                                            println!(
                                                "{}",
                                                format!("Failed to set selected wallet: {}", e)
                                                    .red()
                                            );
                                            return None;
                                        }
                                    },
                                    Err(e) => {
                                        println!(
                                            "{}",
                                            format!("Failed to save wallet: {}", e).red()
                                        );
                                        return None;
                                    }
                                }
                            }
                            Err(e) => {
                                println!(
                                    "{}",
                                    format!("Failed to parse public address: {}", e).red()
                                );
                                return None;
                            }
                        }
                    }
                    Err(e) => {
                        println!("{}", format!("Failed to import seed phrase: {}", e).red());
                        return None;
                    }
                }
            }

            "privatekey" => {
                println!("Enter private key:");
                let mut private_key = String::new();
                io::stdin()
                    .read_line(&mut private_key)
                    .expect("Failed to read line");

                match import_from_private_key(private_key.trim()) {
                    Ok((private_key, _, public_address)) => {
                        let password = prompt_password(true);
                        // Convert public_address to Address type
                        match public_address.parse() {
                            Ok(address) => {
                                match save_wallet(&address, &private_key, "", &password) {
                                    Ok(_) => match set_selected_wallet(&public_address) {
                                        Ok(_) => {
                                            println!(
                                                "Imported wallet with address: {}",
                                                public_address
                                            );
                                            return Some(public_address);
                                        }
                                        Err(e) => {
                                            println!(
                                                "{}",
                                                format!("Failed to set selected wallet: {}", e)
                                                    .red()
                                            );
                                            return None;
                                        }
                                    },
                                    Err(e) => {
                                        println!(
                                            "{}",
                                            format!("Failed to save wallet: {}", e).red()
                                        );
                                        return None;
                                    }
                                }
                            }
                            Err(e) => {
                                println!(
                                    "{}",
                                    format!("Failed to parse public address: {}", e).red()
                                );
                                return None;
                            }
                        }
                    }
                    Err(e) => {
                        println!("{}", format!("Failed to import private key: {}", e).red());
                        return None;
                    }
                }
            }
            _ => {
                display_help(true);
                return None;
            }
        }
    } else {
        display_help(false);
        return None;
    }
}

fn prompt_password(confirm: bool) -> String {
    print!("Enter password for wallet: ");
    io::stdout().flush().unwrap();

    let password = match read_password() {
        Ok(pwd) => pwd,
        Err(e) => {
            eprintln!("Error reading password: {}. Falling back to standard input.", e);
            // Fallback to standard input when secure input fails
            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("Failed to read input");
            input.trim().to_string()
        }
    };

    if confirm {
        print!("Confirm password: ");
        io::stdout().flush().unwrap();
        
        let confirm = match read_password() {
            Ok(pwd) => pwd,
            Err(e) => {
                eprintln!("Error reading password: {}. Falling back to standard input.", e);
                let mut input = String::new();
                io::stdin().read_line(&mut input).expect("Failed to read input");
                input.trim().to_string()
            }
        };

        if password != confirm {
            println!("{}", "Passwords do not match!".red());
            return prompt_password(true);
        }
    }
    password
}
