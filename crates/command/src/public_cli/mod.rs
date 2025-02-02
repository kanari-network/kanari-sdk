use std::process::exit;
use std::path::{Path, PathBuf};

use mona_storage::file_storage::FileStorage;
use colored::Colorize;

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
        name: "search",
        description: "Search for files in storage",
    },

];

fn display_help(show_error: bool) {
    if show_error {
        println!("\n{}", "ERROR: Invalid command".red().bold());
    }

    // Usage section
    println!("{}", "USAGE:".bright_yellow().bold());
    println!("kari public <command> [options]\n");

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
pub fn handle_public_command() -> Option<String> {
    // Collect command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check if any arguments were provided
    if args.len() > 2 {
        // Collect command line arguments
        let command = &args[2];
        // Use string comparison in the match statement
        match command.as_str() {
            "upload" => {
                if args.len() != 4 {  // Changed from 3 to 4 since command is at index 2
                    return Some("Usage: kari public upload <file_path>".to_string());
                }
            
                // Initialize storage first
                if let Err(e) = FileStorage::init_storage() {
                    return Some(format!("Failed to initialize storage: {}", e));
                }
            
                // Get file path from correct argument index (3 instead of 2)
                let file_path = Path::new(&args[3]);
                
                // Debug print
                println!("Attempting to upload file: {}", file_path.display());
                println!("Storage path: {}", get_storage_path().display());
                
                if !file_path.exists() {
                    return Some(format!("Error: File '{}' not found", file_path.display()));
                }
            
                let filename = file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unnamed")
                    .to_string();
            
                match FileStorage::upload(file_path, filename) {
                    Ok(storage) => Some(format!(
                        "File uploaded successfully!\nID: {}\nLocation: {}\nSize: {} bytes\nType: {}",
                        storage.id,
                        storage.path.display(),
                        storage.metadata.size,
                        storage.metadata.content_type
                    )),
                    Err(e) => Some(format!("Upload failed: {}", e))
                }
            },

            "search" => {

            },

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

fn get_storage_path() -> PathBuf {
    let home_dir = dirs::home_dir().expect("Could not find home directory");
    home_dir.join(".kari").join("storage")
}
