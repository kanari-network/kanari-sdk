use std::{path::PathBuf, process::exit};

use colored::Colorize;

use mona_storage::FileStorage;
use tokio::fs;

struct CommandInfo {
    name: &'static str,
    description: &'static str,
}

const COMMANDS: &[CommandInfo] = &[
    CommandInfo {
        name: "upload",
        description: "Upload a file to storage",
    },
    CommandInfo {
        name: "download",
        description: "Download a file from storage",
    },
    CommandInfo {
        name: "delete",
        description: "Delete a file from storage",
    },
];

fn display_help(show_error: bool) {
    if show_error {
        println!("\n{}", "ERROR: Invalid command".red().bold());
    }

    // Usage section
    println!("{}", "USAGE:".bright_yellow().bold());
    println!("  kari public <command> [options]\n");

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

// Handle public commands
pub  fn handle_public_command() -> Option<String> {
    // Collect command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check if any arguments were provided
    if args.len() > 2 {
        // Collect command line arguments
        let command = &args[2];
        // Use string comparison in the match statement
        match command.as_str() {
            "upload" => {
                Some("upload".to_string())
            }

            _ => {
                display_help(true);
                None
            }
        }
    } else {
        display_help(false);
        return None;
    }
}


