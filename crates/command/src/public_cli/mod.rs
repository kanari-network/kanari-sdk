use std::{path::PathBuf, process::exit};

use colored::Colorize;

use std::sync::Once;
use tokio::runtime::Runtime;
use mona_storage::FileStorage;

struct CommandInfo {
    name: &'static str,
    description: &'static str,
}

const COMMANDS: &[CommandInfo] = &[
    CommandInfo { name: "upload", description: "Upload a file to storage" },
    CommandInfo { name: "download", description: "Download a file from storage" },
    CommandInfo { name: "delete", description: "Delete a file from storage" },
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
                if args.len() < 4 {
                    println!("Usage: upload <file_path>");
                    return None;
                }
                handle_upload(&args[3])
            },
            // "download" => {
            //     if args.len() < 4 {
            //         println!("Usage: download <file_id>");
            //         return None;
            //     }
            //     handle_download(&args[3])
            // },
            "delete" => {
                if args.len() < 4 {
                    println!("Usage: delete <file_id>");
                    return None; 
                }
                handle_delete(&args[3])
            },
            _ => {
                display_help(true);
                None
            },
        }
    } else {
        display_help(false);
        return None;
    }
}


static INIT: Once = Once::new();
static mut RUNTIME: Option<Runtime> = None;

pub fn init() {
    INIT.call_once(|| {
        let runtime = Runtime::new().unwrap();
        unsafe {
            RUNTIME = Some(runtime);
        }
    });
}

fn get_runtime() -> &'static Runtime {
    unsafe { RUNTIME.as_ref().unwrap() }
}


fn handle_upload(file_path: &str) -> Option<String> {
    let storage = match FileStorage::new(PathBuf::from("./storage")) {
        Ok(storage) => storage,
        Err(e) => {
            println!("Failed to initialize storage: {}", e);
            return None;
        }
    };

    match get_runtime().block_on(async {
        storage.upload_file(PathBuf::from(file_path)).await
    }) {
        Ok(file_id) => {
            println!("File uploaded successfully. ID: {}", file_id);
            Some("upload".to_string())
        },
        Err(e) => {
            println!("Upload failed: {}", e);
            None
        }
    }
}

fn handle_delete(file_id: &str) -> Option<String> {
    let storage = match FileStorage::new(PathBuf::from("./storage")) {
        Ok(storage) => storage,
        Err(e) => {
            println!("Failed to initialize storage: {}", e);
            return None;
        }
    };

    get_runtime().block_on(async {
        match storage.delete_file(file_id).await {
            Ok(_) => {
                println!("File deleted successfully");
                Some("delete".to_string())
            },
            Err(e) => {
                println!("Delete failed: {}", e);
                None
            }
        }
    })
}

// fn handle_download(file_id: &str) -> Option<String> {
//     let rt = tokio::runtime::Runtime::new().unwrap();
//     let storage = mona_storage::FileStorage::FileStorage::new(PathBuf::from("./storage")).unwrap();
    
//     rt.block_on(async {
//         match storage.get_file(file_id).await {
//             Ok(path) => {
//                 println!("File downloaded to: {}", path.display());
//                 Some("download".to_string())
//             },
//             Err(e) => {
//                 println!("Download failed: {}", e);
//                 None
//             }
//         }
//     })
// }
