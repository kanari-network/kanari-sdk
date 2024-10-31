use colored::Colorize;
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
            },
            "coverage" => { // String comparison for "coverage"
                println!("Inspecting test coverage...");
                return None;
            },
            "disassemble" => { // String comparison for "disassemble"
                println!("Disassembling bytecode...");
                return None;
            },
            "docgen" => { // String comparison for "docgen"
                println!("Generating documentation...");
                return None;
            },
            "errmap" => { // String comparison for "errmap"
                println!("Generating error map...");
                return None;
            },
            "info" => { // String comparison for "info"
                println!("Printing address information...");
                return None;
            },
            "new" => { // String comparison for "new"
                println!("Enter project name:");
                let mut project_name = String::new();
                std::io::stdin().read_line(&mut project_name).unwrap();
                let project_name = project_name.trim();

                match create_move_project(project_name) {
                    Ok(_) => println!("Move project '{}' created successfully.", project_name),
                    Err(e) => eprintln!("Failed to create Move project: {}", e),
                }
                return None;
            },
            "prove" => { // String comparison for "prove"
                println!("Running Move prover...");
                return None;
            },
            "publish" => { // String comparison for "publish"
                println!("Publishing account...");
                return None;
            },
            "run" => { // String comparison for "run"
                println!("Running Move function...");
                return None;
            },
            "test" => { // String comparison for "test"
                println!("Running Move unit tests...");
                return None;
            },
            "help" => { // String comparison for "help"
                println!("Printing help...");
                return None;
            },
            _ => {
                println!("{}", "Invalid command".red());
                println!("Usage: kari move [OPTIONS] <COMMAND>");
                println!("Commands:");
                println!("  {} - Build the package at `path`. If no path is provided defaults to current directory", "build".green());
                println!("  {} - Inspect test coverage for this package. A previous test run with the `--coverage` flag must have previously been run", "coverage".green());
                println!("  {} - Disassemble the Move bytecode pointed to", "disassemble".green());
                println!("  {} - Generate javadoc style documentation for Move packages", "docgen".green());
                println!("  {} - Generate error map for the package and its dependencies at `path` for use by the Move explanation tool", "errmap".green());
                println!("  {} - Print address information", "info".green());
                println!("  {} - CLI frontend for the Move compiler and VM", "new".green());
                println!("  {} - Run the Move Prover on the package at `path`. If no path is provided defaults to current directory. Use `.. prove .. -- <options>` to pass on options to the prover", "prove".green());
                println!("  {} - Common options for interacting with an account for a validator", "publish".green());
                println!("  {} - Run a Move function", "run".green());
                println!("  {} - Run Move unit tests in this package", "test".green());
                println!("  {} - Print this message or the help of the given subcommand(s)", "help".green());
                exit(1); // Exit with an error code
        
            },
        }
    } else {
        println!("{}", "Invalid command".red());
        println!("Usage: kari move [OPTIONS] <COMMAND>");
        println!("Commands:");
        println!("  {} - Build the package at `path`. If no path is provided defaults to current directory", "build".green());
        println!("  {} - Inspect test coverage for this package. A previous test run with the `--coverage` flag must have previously been run", "coverage".green());
        println!("  {} - Disassemble the Move bytecode pointed to", "disassemble".green());
        println!("  {} - Generate javadoc style documentation for Move packages", "docgen".green());
        println!("  {} - Generate error map for the package and its dependencies at `path` for use by the Move explanation tool", "errmap".green());
        println!("  {} - Print address information", "info".green());
        println!("  {} - CLI frontend for the Move compiler and VM", "new".green());
        println!("  {} - Run the Move Prover on the package at `path`. If no path is provided defaults to current directory. Use `.. prove .. -- <options>` to pass on options to the prover", "prove".green());
        println!("  {} - Common options for interacting with an account for a validator", "publish".green());
        println!("  {} - Run a Move function", "run".green());
        println!("  {} - Run Move unit tests in this package", "test".green());
        println!("  {} - Print this message or the help of the given subcommand(s)", "help".green());
        exit(1); // Exit with an error code
    }
}
