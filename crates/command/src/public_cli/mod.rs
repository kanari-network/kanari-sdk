use std::process::exit;

use colored::Colorize;


struct CommandInfo {
    name: &'static str,
    description: &'static str,
}

const COMMANDS: &[CommandInfo] = &[
    CommandInfo { name: "updown", description: "up" },
    CommandInfo { name: "balance", description: "Check balance" },
    CommandInfo { name: "select", description: "Select wallet" },
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


pub fn handle_public_command() -> Option<String> {
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
                return Some("generate".to_string());
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