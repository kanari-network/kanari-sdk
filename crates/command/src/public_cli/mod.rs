use std::process::exit;
use std::path::Path;
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
                if args.len() != 3 {
                    return Some("Usage: mona-storage upload <file_path>".to_string());
                }
            
                // Get file path from correct argument index
                let file_path = Path::new(&args[2]);
                
                // Validate file exists
                if !file_path.exists() {
                    return Some(format!("Error: File '{}' not found", args[2]));
                }
            
                let filename = file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unnamed")
                    .to_string();
            
                // Upload file
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
            "download" => Some("download".to_string()),

            "delete" => Some("delete".to_string()),

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


