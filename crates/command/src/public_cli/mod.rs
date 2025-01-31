use std::{path::PathBuf, process::exit};

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
pub async fn handle_public_command() -> Option<String> {
    // Collect command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check if any arguments were provided
    if args.len() > 2 {
        // Collect command line arguments
        let command = &args[2];
        // Use string comparison in the match statement
        match command.as_str() {
            "upload" => {
                let cmd = UploadCommand::parse();
                match cmd.execute().await {
                    Ok(_) => Some("upload".to_string()),
                    Err(e) => {
                        eprintln!("Upload failed: {}", e);
                        Some("upload".to_string())
                    }
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



use clap::Parser;
use mona_storage::FileStorage;
use std::path::Path;

#[derive(Parser, Debug)]
pub struct UploadCommand {
    /// File path to upload
    #[clap(name = "FILE")]
    pub file: PathBuf,

    /// Custom filename (optional)
    #[clap(short, long)]
    pub name: Option<String>,
}

impl UploadCommand {
    pub async fn execute(self) -> anyhow::Result<()> {
        // Initialize storage directories
        mona_storage::FileStorage::init_storage();

        // Rest of upload logic
        let file_path = Path::new(&self.file).canonicalize()?;
        
        println!("Uploading file: {}", file_path.display());

        let storage = FileStorage::new()?;

        let filename = self.name.unwrap_or_else(|| {
            file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

        println!("Using filename: {}", filename);

        match FileStorage::upload(&file_path, filename).await {
            Ok(file) => {
                println!("Upload successful!");
                println!("Stored at: {}", file.path.display());
                println!("File size: {} bytes", file.metadata.size);
                Ok(())
            }
            Err(e) => {
                eprintln!("Upload failed: {}", e);
                Err(anyhow::anyhow!(e))
            }
        }
    }
}