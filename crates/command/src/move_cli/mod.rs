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
    

    if args.len() <= 2 {
        println!("{}", "Usage: move <command> [options]".red());
        println!("Available commands:");
        println!("  {} - Build the package", "build".green());
        println!("  {} - Inspect test coverage", "coverage".green());
        println!("  {} - Disassemble Move bytecode", "disassemble".green());
        println!("  {} - Generate documentation", "docgen".green());
        println!("  {} - Generate error map", "errmap".green());
        println!("  {} - Print address information", "info".green());
        println!("  {} - Create new Move package", "new".green());
        println!("  {} - Run Move unit tests", "test".green());
        println!("  {} - Execute sandbox commands", "sandbox".green());
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
        // "coverage" => Command::Coverage(Coverage {
        //     options: vec![
        //         CoverageSummaryOptions::ModuleSummary,
        //         CoverageSummaryOptions::FunctionSummary,
        //         CoverageSummaryOptions::BytecodeSummary { module_name: None }
        //     ]
        // }),
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
