use colored::Colorize;
use move_core_types::errmap::ErrorMapping;
use move_vm_test_utils::gas_schedule::zero_cost_schedule;
use move_package::BuildConfig;
use std::{path::PathBuf, process::exit};
use kari_move::{
    base::{
        build::Build, coverage::{Coverage, CoverageSummaryOptions}, disassemble::Disassemble, docgen::Docgen, errmap::Errmap, info::Info, migrate::Migrate, new::New, publish::Publish, call::Call, test::Test
    }, run_cli, sandbox, Command, Move
};

struct CommandInfo {
    name: &'static str,
    description: &'static str,
}

const COMMANDS: &[CommandInfo] = &[
    CommandInfo { name: "build", description: "Build the package" },
    CommandInfo { name: "coverage", description: "Inspect test coverage for this package. A previous test run with the `--coverage` flag must have" },
    CommandInfo { name: "", description: "previously been run" },
    CommandInfo { name: "disassemble", description: "Disassemble Move bytecode" },
    CommandInfo { name: "doc", description: "Generate documentation" },
    CommandInfo { name: "errmap", description: "Generate error map" },
    CommandInfo { name: "info", description: "Print address information" },
    CommandInfo { name: "migrate", description: "Migrate Move module" },
    CommandInfo { name: "new", description: "Create a new Move package with name `name` at `path`. If `path` is not provided the package will" },
    CommandInfo { name: "", description: "be created in the directory `name`" },
    CommandInfo { name: "test", description: "Run Move unit tests" },
    CommandInfo { name: "publish", description: "Publish Move module" },
    CommandInfo { name: "call", description: "Call a function in a Move module" },
    CommandInfo { name: "sandbox", description: "Execute sandbox commands" },
];


fn display_help(show_error: bool) {
    if show_error {
        println!("\n{}", "ERROR: Invalid command".red().bold());
    }


    println!("{}", "USAGE:".bright_yellow().bold());
    println!("kari move <command> [options]\n");

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

pub fn handle_move_command() {
    let args: Vec<String> = std::env::args().collect();
    let cost_table = zero_cost_schedule();
    let error_mapping = ErrorMapping::default();

        // Check for minimum arguments
        if args.len() <= 2 {
            display_help(false);
            return;
        }

    let move_args = Move {
        package_path: None,
        verbose: false,
        build_config: BuildConfig::default(),
    };

    let cmd = match args.get(2).map(|s| s.as_str()) {
        Some("build") => Command::Build(Build {}),
        Some("coverage") => Command::Coverage(Coverage {
            options: CoverageSummaryOptions::Summary {
                functions: false,
                output_csv: false
            }
        }),
        Some("disassemble") => Command::Disassemble(Disassemble {
            interactive: false,
            package_name: None,
            module_or_script_name: String::new(),
            debug: true
        }),
        Some("doc") => Command::Docgen(Docgen {
            section_level_start: Some(0),
            exclude_private_fun: false,
            exclude_specs: false,
            independent_specs: false,
            exclude_impl: false,
            toc_depth: Some(3),
            no_collapsed_sections: false,
            output_directory: None,
            compile_relative_to_output_dir: false,
            references_file: None,
            template: Vec::new(),
            include_dep_diagrams: false,
            include_call_diagrams: false
        }),
        Some("errmap") => Command::Errmap(Errmap {
            error_prefix: None,
            output_file: PathBuf::new()
        }),
        Some("info") => Command::Info(Info {}),
        Some("migrate") => Command::Migrate(Migrate {}),
        Some("new") => {
            match args.get(3).map(String::from) {
                Some(name) if !name.is_empty() => Command::New(New { name }),
                _ => {
                    eprintln!("Error: Project name is required. Usage: kari move new <project_name>");
                    std::process::exit(1);
                }
            }
        },
        Some("test") => Command::Test(Test {
            filter: None,
            list: false,
            num_threads: 1,
            report_statistics: None,
            check_stackless_vm: false,
            verbose_mode: false,
            compute_coverage: false,
            gas_limit: None
        }),
        Some("publish") => {
            // Default to current directory if not specified
            let module_path = if args.len() > 3 {
                PathBuf::from(args[3].clone())
            } else {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::new())
            };
            
            // Check for address parameter (--address=0x...)
            let address = args.iter().find(|arg| arg.starts_with("--address="))
                .and_then(|arg| {
                    let addr_str = arg.trim_start_matches("--address=");
                    match move_core_types::account_address::AccountAddress::from_hex_literal(addr_str) {
                        Ok(addr) => Some(addr),
                        Err(_) => None,
                    }
                });
            
            // Check for gas_budget parameter (--gas-budget=N)
            let gas_budget = args.iter().find(|arg| arg.starts_with("--gas-budget="))
                .and_then(|arg| {
                    let budget_str = arg.trim_start_matches("--gas-budget=");
                    budget_str.parse::<u64>().ok()
                }).unwrap_or(3_000_000); // Default to 3M gas units (0.003 KARI)
            
            // Check for --skip-verify flag
            let skip_verify = args.iter().any(|arg| arg == "--skip-verify");
            
            // Default to using Mona VM for blockchain deployment (use --no-mona-vm to disable)
            let use_mona_vm = !args.iter().any(|arg| arg == "--no-mona-vm");
            
            println!("Publishing Move module to {}blockchain...", 
                     if use_mona_vm { "" } else { "local sandbox (not " });
            
            Command::Publish(Publish {
                module_path,
                gas_budget,
                address,
                skip_verify,
                use_mona_vm,
            })
        },
        Some("call") => {
            // Parse both formats: --function-id=VALUE and --function-id VALUE
            let function_id = args.iter()
                .position(|arg| arg == "--function-id" || arg.starts_with("--function-id="))
                .and_then(|pos| {
                    if args[pos] == "--function-id" && args.len() > pos + 1 {
                        // Handle --function-id VALUE format
                        Some(args[pos + 1].clone())
                    } else if args[pos].starts_with("--function-id=") {
                        // Handle --function-id=VALUE format
                        Some(args[pos].trim_start_matches("--function-id=").to_string())
                    } else {
                        None
                    }
                });
                
            // Do the same for module and function if needed
            let module = args.iter()
                .position(|arg| arg == "--module" || arg.starts_with("--module="))
                .and_then(|pos| {
                    if args[pos] == "--module" && args.len() > pos + 1 {
                        Some(args[pos + 1].clone())
                    } else if args[pos].starts_with("--module=") {
                        Some(args[pos].trim_start_matches("--module=").to_string())
                    } else {
                        None
                    }
                });

            let function = args.iter()
                .position(|arg| arg == "--function" || arg.starts_with("--function="))
                .and_then(|pos| {
                    if args[pos] == "--function" && args.len() > pos + 1 {
                        Some(args[pos + 1].clone())
                    } else if args[pos].starts_with("--function=") {
                        Some(args[pos].trim_start_matches("--function=").to_string())
                    } else {
                        None
                    }
                });

            // Package is less commonly used so keep it simple
            let package = args.iter().find(|arg| arg.starts_with("--package="))
                .map(|arg| arg.trim_start_matches("--package=").to_string());
            
            // Extract function arguments - support both formats
            let mut args_values = Vec::new();
            let mut i = 0;
            while i < args.len() {
                if args[i] == "--args" && args.len() > i + 1 {
                    args_values.push(args[i + 1].clone());
                    i += 2; // Skip the value we just processed
                } else if args[i].starts_with("--args=") {
                    args_values.push(args[i].trim_start_matches("--args=").to_string());
                    i += 1;
                } else {
                    i += 1;
                }
            }

            // Extract gas budget with default of 1000
            let gas_budget = args.iter()
                .position(|arg| arg == "--gas-budget" || arg.starts_with("--gas-budget="))
                .and_then(|pos| {
                    if args[pos] == "--gas-budget" && args.len() > pos + 1 {
                        args[pos + 1].parse::<u64>().ok()
                    } else if args[pos].starts_with("--gas-budget=") {
                        args[pos].trim_start_matches("--gas-budget=").parse::<u64>().ok()
                    } else {
                        None
                    }
                }).unwrap_or(1000);

            // Optional sender address
            let sender = args.iter()
                .position(|arg| arg == "--sender" || arg.starts_with("--sender="))
                .and_then(|pos| {
                    if args[pos] == "--sender" && args.len() > pos + 1 {
                        Some(args[pos + 1].clone())
                    } else if args[pos].starts_with("--sender=") {
                        Some(args[pos].trim_start_matches("--sender=").to_string())
                    } else {
                        None
                    }
                });

            Command::Call(Call {
                function_id,
                package,
                module,
                function,
                args: args_values,
                gas_budget,
                sender,
            })
        },
        Some("sandbox") => Command::Sandbox {
            storage_dir: PathBuf::from(kari_move::DEFAULT_STORAGE_DIR),
            cmd: sandbox::cli::SandboxCommand::Clean {}
        },
        _ => {
            display_help(true);
            return;
        }
    };

    if let Err(e) = run_cli(Vec::new(), &cost_table, &error_mapping, move_args, cmd) {
        println!("\n{}: {}", "ERROR".red().bold(), e);
        exit(1);
    }
}
