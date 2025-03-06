use anyhow::Result;
use clap::{Command, Arg};
use mona_move::MoveModuleHandler;
use move_core_types::account_address::AccountAddress;
use std::fs;

fn main() -> Result<()> {
    let matches = Command::new("MonaMove")
        .version("0.1.0")
        .about("Move module compiler and executor for MonaVM")
        .subcommand(
            Command::new("compile")
                .about("Compile a Move module")
                .arg(
                    Arg::new("SOURCE")
                        .help("Path to Move source file")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("address")
                        .help("Publishing account address")
                        .short('a')
                        .long("address")
                        .value_parser(clap::value_parser!(String))
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("execute")
                .about("Execute a Move function")
                .arg(
                    Arg::new("MODULE_ID")
                        .help("Module ID (hex)")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("function")
                        .help("Function name to execute")
                        .short('f')
                        .long("function")
                        .value_parser(clap::value_parser!(String))
                        .required(true),
                )
                .arg(
                    Arg::new("args")
                        .help("Function arguments (comma separated)")
                        .short('p')
                        .long("args")
                        .value_parser(clap::value_parser!(String))
                        .default_value(""),
                )
                .arg(
                    Arg::new("sender")
                        .help("Sender address")
                        .short('s')
                        .long("sender")
                        .value_parser(clap::value_parser!(String))
                        .required(true),
                ),
        )
        .get_matches();

    let mut handler = MoveModuleHandler::new();

    if let Some(matches) = matches.subcommand_matches("compile") {
        let source_path = matches.get_one::<String>("SOURCE").unwrap();
        let address_str = matches.get_one::<String>("address").unwrap();
        
        let address = AccountAddress::from_hex_literal(address_str)
            .unwrap_or_else(|_| panic!("Invalid address format: {}", address_str));
        
        let source = fs::read_to_string(source_path)
            .unwrap_or_else(|_| panic!("Failed to read source file: {}", source_path));
        
        let module_id = handler.compile_and_upload(&source, address)?;
        println!("Module compiled and uploaded successfully!");
        println!("Module ID: 0x{}", hex::encode(&module_id));
        
    } else if let Some(matches) = matches.subcommand_matches("execute") {
        let module_id_hex = matches.get_one::<String>("MODULE_ID").unwrap();
        let function_name = matches.get_one::<String>("function").unwrap();
        let args_str = matches.get_one::<String>("args").unwrap();
        let sender_str = matches.get_one::<String>("sender").unwrap();
        
        let module_id = hex::decode(module_id_hex.trim_start_matches("0x"))
            .unwrap_or_else(|_| panic!("Invalid module ID format: {}", module_id_hex));
            
        let sender = AccountAddress::from_hex_literal(sender_str)
            .unwrap_or_else(|_| panic!("Invalid address format: {}", sender_str));
            
        let args: Vec<Vec<u8>> = if args_str.is_empty() {
            Vec::new()
        } else {
            args_str.split(',')
                .map(|arg| arg.trim().as_bytes().to_vec())
                .collect()
        };
            
        let status = handler.execute_function(module_id, function_name, args, sender)?;
        println!("Function executed with status: {:?}", status);
    }

    Ok(())
}