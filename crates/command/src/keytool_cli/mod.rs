use colored::Colorize;
use key::{
    generate_karix_address, import_from_private_key, import_from_seed_phrase, list_wallet_files,
    load_wallet, save_wallet, set_selected_wallet,
};
use std::io::{self, Write};

use panorama::blockchain::{get_balance, load_blockchain_with_retry};
use panorama::simulation::process_transfer;
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
                                if (mnemonic_length != 12 && mnemonic_length != 24) {
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
                                    // Format the balance in a human-readable way
                                    let formatted_balance = format_balance(balance);
                                    println!(
                                        "Balance for {}: {} Kari",
                                        public_address.green(),
                                        formatted_balance.green()
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

            "transfer" => {
                // Load blockchain first
                match load_blockchain_with_retry() {
                    Ok(_) => {
                        // Get sender address (current wallet)
                        let wallets = match list_wallet_files() {
                            Ok(w) => w,
                            Err(e) => {
                                println!("{}", format!("Error listing wallets: {}", e).red());
                                return None;
                            }
                        };
                        
                        // Find selected wallet
                        let selected_wallet = wallets.iter()
                            .find(|(_, is_selected)| *is_selected)
                            .map(|(name, _)| name.trim_end_matches(".enc").to_string());
                        
                        let sender_address = match selected_wallet {
                            Some(addr) => addr,
                            None => {
                                // Try to select wallet interactively if none is selected
                                println!("{}", "No wallet selected. Please select a wallet first.".yellow());
                                
                                if wallets.is_empty() {
                                    println!("{}", "No wallets found!".red());
                                    return None;
                                }
                                
                                println!("\nAvailable wallets:");
                                for (i, (wallet, _)) in wallets.iter().enumerate() {
                                    let wallet_name = wallet.trim_end_matches(".enc");
                                    println!("{}. {}", i + 1, wallet_name);
                                }
                                
                                println!("\nEnter wallet number to use (or press Enter to cancel):");
                                let mut input = String::new();
                                match io::stdin().read_line(&mut input) {
                                    Ok(_) => {
                                        if input.trim().is_empty() {
                                            println!("Transfer cancelled.");
                                            return None;
                                        }
                                        
                                        match input.trim().parse::<usize>() {
                                            Ok(index) if index > 0 && index <= wallets.len() => {
                                                let selected = wallets[index - 1].0.trim_end_matches(".enc");
                                                match set_selected_wallet(selected) {
                                                    Ok(_) => {
                                                        println!("Using wallet: {}", selected.green());
                                                        selected.to_string()
                                                    }
                                                    Err(e) => {
                                                        println!("{}", format!("Error setting wallet: {}", e).red());
                                                        return None;
                                                    }
                                                }
                                            }
                                            _ => {
                                                println!("{}", format!("Invalid selection. Please enter a number between 1 and {}", wallets.len()).red());
                                                return None;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("{}", format!("Error reading input: {}", e).red());
                                        return None;
                                    }
                                }
                            }
                        };
                        
                        // Check sender balance
                        let balance = match get_balance(&sender_address) {
                            Ok(b) => b,
                            Err(e) => {
                                println!("{}", format!("Error checking balance: {}", e).red());
                                return None;
                            }
                        };
                        
                        let formatted_balance = format_balance(balance);
                        println!("Your balance: {} KARI", formatted_balance.green());
                        
                        // Get recipient address
                        println!("Enter recipient address:");
                        let mut recipient = String::new();
                        match io::stdin().read_line(&mut recipient) {
                            Ok(_) => {},
                            Err(e) => {
                                println!("{}", format!("Error reading input: {}", e).red());
                                return None;
                            }
                        };
                        let recipient = recipient.trim();
                        
                        // Get amount to send
                        println!("Enter amount to send (in KARI):");
                        let mut amount_str = String::new();
                        match io::stdin().read_line(&mut amount_str) {
                            Ok(_) => {},
                            Err(e) => {
                                println!("{}", format!("Error reading input: {}", e).red());
                                return None;
                            }
                        };
                        
                        // Parse amount and convert from KARI to KA units
                        let amount_kari = match amount_str.trim().parse::<f64>() {
                            Ok(a) => a,
                            Err(e) => {
                                println!("{}", format!("Invalid amount: {}", e).red());
                                return None;
                            }
                        };
                        
                        const KA_PER_KARI: u64 = 1_000_000_000;
                        let amount_ka = (amount_kari * KA_PER_KARI as f64) as u64;
                        
                        // Confirm the transfer
                        println!("\nTransaction details:");
                        println!("  From: {}", sender_address.green());
                        println!("  To:   {}", recipient.green());
                        println!("  Amount: {} KARI ({} KA)", amount_str.trim(), amount_ka);
                        
                        println!("\nConfirm transfer? (y/n)");
                        let mut confirm = String::new();
                        match io::stdin().read_line(&mut confirm) {
                            Ok(_) => {},
                            Err(e) => {
                                println!("{}", format!("Error reading input: {}", e).red());
                                return None;
                            }
                        };
                        
                        if !confirm.trim().eq_ignore_ascii_case("y") {
                            println!("Transfer cancelled.");
                            return None;
                        }
                        
                        // Unlock wallet to verify ownership
                        println!("Enter wallet password:");
                        let password = prompt_password(false);
                        match load_wallet(&sender_address, &password) {
                            Ok(_wallet) => {
                                // Create a channel for transaction processing
                                let (tx, _rx) = tokio::sync::mpsc::channel::<String>(100);
                                
                                // Process the transfer
                                match process_transfer(&sender_address, recipient, amount_ka, &tx) {
                                    Ok(_) => {
                                        println!("{}", "Transfer initiated successfully!".green());
                                        println!("Transaction will be included in the next block.");
                                        return Some(sender_address);
                                    },
                                    Err(e) => {
                                        println!("{}", format!("Transfer failed: {}", e).red());
                                        return None;
                                    }
                                }
                            },
                            Err(e) => {
                                println!("{}", format!("Failed to unlock wallet: {}", e).red());
                                return None;
                            }
                        }
                    },
                    Err(e) => {
                        println!("{}", format!("Failed to load blockchain: {}", e).red());
                        return None;
                    }
                }
            },

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
                            let wallet_display = wallet_name.trim_end_matches(".enc");
                            if is_selected {
                                println!("➤ {} {}", wallet_display.green().bold(), "[SELECTED]".green().bold());
                            } else {
                                println!("  {}", wallet_display);
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

// Add this function to format KA balance into human-readable Kari format
fn format_balance(ka_balance: u64) -> String {
    // KA is 10^-9 of a Kari token
    const KA_PER_KARI: u64 = 1_000_000_000;
    
    // Separate whole Kari and fractional part (KA)
    let whole_kari = ka_balance / KA_PER_KARI;
    let fractional_ka = ka_balance % KA_PER_KARI;
    
    // Format whole part with thousands separators
    let whole_part_formatted = format!("{}", whole_kari)
        .chars()
        .rev()
        .collect::<Vec<_>>()
        .chunks(3)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(",")
        .chars()
        .rev()
        .collect::<String>();
    
    // Format fractional part with leading zeros if needed
    let fractional_part_formatted = format!("{:09}", fractional_ka);
    
    format!("{}.{}", whole_part_formatted, fractional_part_formatted)
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
