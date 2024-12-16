use colored::Colorize;
use move_core_types::errmap::ErrorMapping;
use move_vm_test_utils::gas_schedule::zero_cost_schedule;
use move_package::BuildConfig;
use std::{path::PathBuf, process::exit};
use kari_move::{
    base::{
        build::Build,
        coverage::{Coverage, CoverageSummaryOptions},
        disassemble::Disassemble,
        docgen::Docgen,
        errmap::Errmap,
        info::Info,
        migrate::Migrate,
        new::New,
        test::Test,
    }, run_cli, sandbox, Command, Move
};

pub fn handle_move_command() {
    let args: Vec<String> = std::env::args().collect();
    let cost_table = zero_cost_schedule();
    let error_mapping = ErrorMapping::default();
    

    struct CommandInfo {
        name: &'static str,
        description: &'static str,
    }
    
    const COMMANDS: &[CommandInfo] = &[
        CommandInfo { name: "build", description: "Build the package" },
        CommandInfo { name: "coverage", description: "Inspect test coverage" },
        CommandInfo { name: "disassemble", description: "Disassemble Move bytecode" },
        CommandInfo { name: "docgen", description: "Generate documentation" },
        CommandInfo { name: "errmap", description: "Generate error map" },
        CommandInfo { name: "info", description: "Print address information" },
        CommandInfo { name: "new", description: "Create new Move package" },
        CommandInfo { name: "test", description: "Run Move unit tests" },
        CommandInfo { name: "sandbox", description: "Execute sandbox commands" },
    ];
    
    if args.len() <= 2 {
        println!("\n{}", "Usage: move <command> [options]".bright_white().bold());
        println!("\n{}\n", "Available commands:".bright_white());
        
        let max_name_len = COMMANDS.iter().map(|cmd| cmd.name.len()).max().unwrap_or(0);
        
        for cmd in COMMANDS {
            println!(
                "  {}{}  {}", 
                cmd.name.green(),
                " ".repeat(max_name_len - cmd.name.len() + 2),
                cmd.description.bright_white()
            );
        }
        println!();
        exit(1);
    }

    let command = &args[2];
    let move_args = Move {
        package_path: None,
        verbose: false,
        build_config: BuildConfig::default(),
    };

    let cmd = match command.as_str() {
        "build" => Command::Build(Build {}),
        "coverage" => Command::Coverage(Coverage {
            options: CoverageSummaryOptions::Summary {
                functions: false,
                output_csv: false
            }
        }),
        "disassemble" => Command::Disassemble(Disassemble {
            interactive: false,
            package_name: None,
            module_or_script_name: String::new(),
            debug: false
        }),
        "docgen" => Command::Docgen(Docgen {
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
        "errmap" => Command::Errmap(Errmap {
            error_prefix: None,
            output_file: PathBuf::new()
        }),
        "info" => Command::Info(Info {}),
        "migrate" => Command::Migrate(Migrate {}),
        "new" => Command::New(New {
            name: args.get(3).map(String::from).unwrap_or_default()
        }),
        "test" => Command::Test(Test {
            filter: None,
            list: false,
            num_threads: 1,
            report_statistics: None,
            check_stackless_vm: false,
            verbose_mode: false,
            compute_coverage: false,
            gas_limit: None
        }),
        "sandbox" => Command::Sandbox {
            storage_dir: PathBuf::from(kari_move::DEFAULT_STORAGE_DIR),
            cmd: sandbox::cli::SandboxCommand::Clean {}
        },
        _ => {
            println!("{}", "Invalid command".red());
            exit(1);
        }
    };

    if let Err(e) = run_cli(Vec::new(), &cost_table, &error_mapping, move_args, cmd) {
        println!("{}: {}", "Error".red(), e);
        exit(1);
    }
}
