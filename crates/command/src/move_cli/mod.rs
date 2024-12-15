use colored::Colorize;
use kari_move::create_move_project;
use std::process::exit;

pub fn handle_move_command() -> Option<String> {
    // Collect command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check if any arguments were provided
    if args.len() > 2 {
        // Collect command line arguments
        let command = &args[2];
        // Use string comparison in the match statement
        match command.as_str() {
            "build" => {
                println!("Building package...");
                return None;
            }
            "coverage" => {
                // String comparison for "coverage"
                println!("Inspecting test coverage...");
                return None;
            }
            "disassemble" => {
                // String comparison for "disassemble"
                println!("Disassembling bytecode...");
                return None;
            }
            "docgen" => {
                // String comparison for "docgen"
                println!("Generating documentation...");
                return None;
            }
            "errmap" => {
                // String comparison for "errmap"
                println!("Generating error map...");
                return None;
            }
            "info" => {
                // String comparison for "info"
                println!("Printing address information...");
                return None;
            }
            "new" => {
                if let Some(project_name) = args.get(2) {
                    // Validate project name
                    if !is_valid_project_name(project_name) {
                        eprintln!("❌ Invalid project name: {}", project_name);
                        eprintln!("Project name must:");
                        eprintln!("- Start with a letter");
                        eprintln!("- Contain only letters, numbers, or underscores");
                        return Some(1.to_string());
                    }

                    // Create project directory
                    match create_move_project(project_name) {
                        Ok(_) => {
                            println!("✨ Created new Move project: {}", project_name);
                            println!("\nNext steps:");
                            println!("  cd {}", project_name);
                            println!("  kari move build");
                            return Some(0.to_string());
                        }
                        Err(e) => {
                            eprintln!("❌ Failed to create project: {}", e);
                            return Some(1.to_string());
                        }
                    }
                } else {
                    eprintln!("Usage: kari move new <project-name>");
                    eprintln!("Example: kari move new my_move_project");
                    return Some(1.to_string());
                }
            }

            "prove" => {
                // String comparison for "prove"
                println!("Running Move prover...");
                return None;
            }
            "publish" => {
                // String comparison for "publish"
                println!("Publishing account...");
                return None;
            }
            "run" => {
                // String comparison for "run"
                println!("Running Move function...");
                return None;
            }
            "test" => {
                // String comparison for "test"
                println!("Running Move unit tests...");
                return None;
            }
            "help" => {
                // String comparison for "help"
                println!("Printing help...");
                return None;
            }
            _ => {
                println!("{}", "Invalid command".red());
                println!("Usage: kari move [OPTIONS] <COMMAND>");
                println!("Commands:");
                println!("  {} - Build the package at `path`. If no path is provided defaults to current directory", "build".green());
                println!("  {} - Inspect test coverage for this package. A previous test run with the `--coverage` flag must have previously been run", "coverage".green());
                println!(
                    "  {} - Disassemble the Move bytecode pointed to",
                    "disassemble".green()
                );
                println!(
                    "  {} - Generate javadoc style documentation for Move packages",
                    "docgen".green()
                );
                println!("  {} - Generate error map for the package and its dependencies at `path` for use by the Move explanation tool", "errmap".green());
                println!("  {} - Print address information", "info".green());
                println!(
                    "  {} - CLI frontend for the Move compiler and VM",
                    "new".green()
                );
                println!("  {} - Run the Move Prover on the package at `path`. If no path is provided defaults to current directory. Use `.. prove .. -- <options>` to pass on options to the prover", "prove".green());
                println!(
                    "  {} - Common options for interacting with an account for a validator",
                    "publish".green()
                );
                println!("  {} - Run a Move function", "run".green());
                println!("  {} - Run Move unit tests in this package", "test".green());
                println!(
                    "  {} - Print this message or the help of the given subcommand(s)",
                    "help".green()
                );
                exit(1); // Exit with an error code
            }
        }
    } else {
        println!("{}", "Invalid command".red());
        println!("Usage: kari move [OPTIONS] <COMMAND>");
        println!("Commands:");
        println!("  {} - Build the package at `path`. If no path is provided defaults to current directory", "build".green());
        println!("  {} - Inspect test coverage for this package. A previous test run with the `--coverage` flag must have previously been run", "coverage".green());
        println!(
            "  {} - Disassemble the Move bytecode pointed to",
            "disassemble".green()
        );
        println!(
            "  {} - Generate javadoc style documentation for Move packages",
            "docgen".green()
        );
        println!("  {} - Generate error map for the package and its dependencies at `path` for use by the Move explanation tool", "errmap".green());
        println!("  {} - Print address information", "info".green());
        println!(
            "  {} - CLI frontend for the Move compiler and VM",
            "new".green()
        );
        println!("  {} - Run the Move Prover on the package at `path`. If no path is provided defaults to current directory. Use `.. prove .. -- <options>` to pass on options to the prover", "prove".green());
        println!(
            "  {} - Common options for interacting with an account for a validator",
            "publish".green()
        );
        println!("  {} - Run a Move function", "run".green());
        println!("  {} - Run Move unit tests in this package", "test".green());
        println!(
            "  {} - Print this message or the help of the given subcommand(s)",
            "help".green()
        );
        exit(1); // Exit with an error code
    }
}

// Add helper function for name validation
fn is_valid_project_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() {
        return false;
    }

    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
