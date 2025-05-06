use colored::Colorize;
use move_core_types::errmap::ErrorMapping;
use move_vm_test_utils::gas_schedule::zero_cost_schedule;
use move_package::BuildConfig;
use std::{path::PathBuf, process::exit};
use kari_move::{
    base::{
        build::Build, coverage::{Coverage, CoverageSummaryOptions}, disassemble::Disassemble, 
        docgen::Docgen, errmap::Errmap, info::Info, migrate::Migrate, new::New, 
        publish::Publish, call::Call, test::Test
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
    CommandInfo { name: "publish", description: "Publish Move module to blockchain network" },
    CommandInfo { name: "call", description: "Call a function in a Move module on the blockchain" },
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
            let module_path = if args.len() > 3 && !args[3].starts_with("--") {
                PathBuf::from(args[3].clone())
            } else {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::new())
            };
            
            // Check for gas_budget parameter (--gas-budget=N)
            let gas_budget = args.iter().find(|arg| arg.starts_with("--gas-budget"))
                .and_then(|arg| {
                    let budget_str = arg.trim_start_matches("--gas-budget=");
                    budget_str.parse::<u64>().ok()
                }).unwrap_or(3_000_000); // Default to 3M gas units (0.003 KARI)
            
            // Check for --skip-verify flag
            let skip_verify = args.iter().any(|arg| arg == "--skip-verify");
            
            // Check for address parameter (--address 0x...)
            // If not provided, use the first address from the config file or default to 0x1
            let address = args.iter().find(|arg| arg.starts_with("--address="))
                .and_then(|arg| {
                    let addr_str = arg.trim_start_matches("--address=");
                    // Try parsing with 0x prefix if not present
                    let addr_with_prefix = if !addr_str.starts_with("0x") {
                        format!("0x{}", addr_str)
                    } else {
                        addr_str.to_string()
                    };
                    
                    use move_core_types::account_address::AccountAddress;
                    AccountAddress::from_hex_literal(&addr_with_prefix)
                        .or_else(|_| AccountAddress::from_hex(addr_str))
                        .ok()
                });
            
            // Check for password parameter
            let password = args.iter().find(|arg| arg.starts_with("--password="))
                .map(|arg| arg.trim_start_matches("--password=").to_string());
            
            Command::Publish(Publish {
                module_path,
                gas_budget,
                skip_verify,
                address,
                password,
            })
        },
        Some("call") => {
            // สร้างฟังก์ชันช่วยในการดึงค่าพารามิเตอร์ทั้งสองรูปแบบ
            fn get_param_value(args: &[String], name: &str) -> Option<String> {
                // รูปแบบ --name=value
                if let Some(arg) = args.iter().find(|arg| arg.starts_with(&format!("{}=", name))) {
                    return Some(arg.trim_start_matches(&format!("{}=", name)).to_string());
                }

                // รูปแบบ --name value
                for i in 0..args.len().saturating_sub(1) {
                    if args[i] == name {
                        return Some(args[i + 1].clone());
                    }
                }

                None
            }

            // ดึงค่า module_id
            let module_id = match get_param_value(&args, "--module-id") {
                Some(value) => value,
                None => {
                    eprintln!("Error: --module-id parameter is required");
                    std::process::exit(1);
                }
            };

            // ดึงค่า function
            let function = match get_param_value(&args, "--function") {
                Some(value) => value,
                None => {
                    eprintln!("Error: --function parameter is required");
                    std::process::exit(1);
                }
            };

            // ดึงค่า args (อาร์กิวเมนต์)
            let mut args_list = Vec::new();
            
            // ตรวจสอบ args ในรูปแบบ --args=val1,val2
            if let Some(args_str) = get_param_value(&args, "--args") {
                args_list = args_str.split(',')
                    .map(|s| s.to_string())
                    .collect();
            }

            // ดึงค่า gas_budget
            let gas_budget = get_param_value(&args, "--gas-budget")
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(1_000_000);

            // ดึงค่า address
            let address = get_param_value(&args, "--address")
                .and_then(|addr_str| {
                    let addr_with_prefix = if !addr_str.starts_with("0x") {
                        format!("0x{}", addr_str)
                    } else {
                        addr_str
                    };
                    
                    use move_core_types::account_address::AccountAddress;
                    AccountAddress::from_hex_literal(&addr_with_prefix)
                        .or_else(|_| AccountAddress::from_hex(&addr_with_prefix.trim_start_matches("0x")))
                        .ok()
                });

            Command::Call(Call {
                module_id,
                function,
                args: args_list,
                gas_budget,
                address,
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