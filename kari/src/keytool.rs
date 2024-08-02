use std::io;
use colored::Colorize;
use kari_evm::EvmVm;

use crate::blockchain::{BALANCES, load_blockchain};
use crate::wallet::{generate_karix_address, load_wallet, save_wallet, send_coins};

pub fn handle_keytool_command() -> Option<String> {
    println!("Enter command");
    println!("1: Generate new address");
    println!("2: Check balance");
    println!("3: Load existing wallet");
    println!("4: Send coins");
    println!("5: Deploy Solidity contract");
    let mut command_str = String::new();
    io::stdin().read_line(&mut command_str).unwrap();
    let command: usize = command_str.trim().parse().expect("Invalid input");

    match command {
        1 => {
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

            Some(public_address)
        },
        2 => {
            println!("Enter public address:");
            let mut public_address = String::new();
            io::stdin().read_line(&mut public_address).unwrap();
            public_address = public_address.trim().to_string();

            load_blockchain();

            let balance = unsafe {
                BALANCES.as_ref().unwrap().lock().unwrap().get(&public_address).cloned().unwrap_or(0)
            };

            println!("Balance for {}: {}", public_address.green(), balance.to_string().green());
            None
        },
        3 => {
            println!("Enter public address to load:");
            let mut public_address = String::new();
            io::stdin().read_line(&mut public_address).unwrap();
            public_address = public_address.trim().to_string();

            if let Some(wallet_data) = load_wallet(&public_address) {
                println!("Wallet loaded:");
                println!("Address: {}", wallet_data["address"].as_str().unwrap().green());
                println!("Private Key: {}", wallet_data["private_key"].as_str().unwrap().green());
                println!("Seed Phrase: {}", wallet_data["seed_phrase"].as_str().unwrap().green());
                Some(public_address)
            } else {
                println!("Wallet not found for address: {}", public_address.red());
                None
            }
        },
        4 => {
            println!("Enter sender's public address:");
            let mut sender_address = String::new();
            io::stdin().read_line(&mut sender_address).unwrap();
            let sender_address = sender_address.trim().to_string();

            println!("Enter receiver's public address:");
            let mut receiver_address = String::new();
            io::stdin().read_line(&mut receiver_address).unwrap();
            let receiver_address = receiver_address.trim().to_string();

            println!("Enter amount to send:");
            let mut amount_str = String::new();
            io::stdin().read_line(&mut amount_str).unwrap();
            let amount: u64 = amount_str.trim().parse().expect("Invalid input for amount");

            if send_coins(sender_address, receiver_address, amount) {
                println!("Transaction successful.");
            } else {
                println!("Transaction failed.");
            }
            None
        },
        5 => {
            println!("Enter Solidity code:");
            let mut solidity_code = String::new();
            io::stdin().read_line(&mut solidity_code).unwrap();

            let vm = EvmVm::new();
            match vm.compile_solidity(&solidity_code) {
                Ok(bytecode) => {
                    match vm.deploy_contract(&bytecode) {
                        Ok(contract_address) => {
                            println!("Contract deployed at address: {}", contract_address.green());
                        },
                        Err(e) => {
                            println!("Deployment failed: {}", e.red());
                        }
                    }
                },
                Err(e) => {
                    println!("Compilation failed: {}", e.red());
                }
            }
            None
        },
        _ => {
            println!("{}", "Invalid command".red());
            None
        },
    }
}