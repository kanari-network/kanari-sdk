use anyhow::Result;
use move_core_types::account_address::AccountAddress;
use kanari_types::address::Address as KanariAddress;

/// Load move-stdlib and kanari-system modules as methods on `MoveRuntime`
impl super::MoveRuntime {
    /// Load move-stdlib modules (0x1::*)
    pub fn load_move_stdlib(&mut self) -> Result<()> {
        // Determine stdlib path. Allow override via MOVE_STDLIB_PATH env var.
        let modules_dir = if let Ok(path_str) = std::env::var("MOVE_STDLIB_PATH") {
            std::path::PathBuf::from(path_str)
        } else {
            // Try two candidate locations to avoid duplicated `crates/crates` when cwd is already the `crates` folder.
            let cwd = std::env::current_dir().unwrap_or_default();
            let candidate1 = cwd
                .join("crates")
                .join("kanari-frameworks")
                .join("packages")
                .join("move-stdlib")
                .join("build")
                .join("MoveStdlib")
                .join("bytecode_modules");

            if candidate1.exists() {
                candidate1
            } else if let Some(parent) = cwd.parent() {
                let candidate2 = parent
                    .join("crates")
                    .join("kanari-frameworks")
                    .join("packages")
                    .join("move-stdlib")
                    .join("build")
                    .join("MoveStdlib")
                    .join("bytecode_modules");
                candidate2
            } else {
                candidate1
            }
        };

        println!("✓ Looking for Move stdlib modules at: {:?}", modules_dir);

        if !modules_dir.exists() {
            eprintln!(
                "Warning: Move stdlib modules not found at {:?}",
                modules_dir
            );
            eprintln!("Standard library will not be pre-loaded.");
            return Ok(());
        }

        // Load stdlib modules in dependency order
        let std_addr = AccountAddress::from_hex_literal(KanariAddress::STD_ADDRESS)?;
        let module_order = vec![
            "vector.mv",
            "error.mv",
            "address.mv",
            "signer.mv",
            "option.mv",
            "fixed_point32.mv",
            "ascii.mv",
            "string.mv",
            "hash.mv",
            "bcs.mv",
            "bit_vector.mv",
            "type_name.mv",
        ];

        let mut count = 0;
        for module_file in module_order {
            let module_path = modules_dir.join(module_file);
            if let Ok(module_bytes) = std::fs::read(&module_path) {
                match self.publish_module(module_bytes, std_addr, None) {
                    Ok(_) => count += 1,
                    Err(e) => {
                        // Silently skip already loaded modules
                        if !e.to_string().contains("already exists") {
                            eprintln!("Warning: Failed to load {}: {}", module_file, e);
                        }
                    }
                }
            }
        }

        println!("✓ Loaded {} move-stdlib modules (0x1::*)", count);
        Ok(())
    }

    /// Load Kanari system modules (0x2::*)
    pub fn load_kanari_system(&mut self) -> Result<()> {
        // Path to pre-compiled Kanari system modules
        // Determine kanari system framework path (allow override via KANARI_FRAMEWORK_PATH).
        let modules_dir = if let Ok(path_str) = std::env::var("KANARI_FRAMEWORK_PATH") {
            std::path::PathBuf::from(path_str)
        } else {
            let cwd = std::env::current_dir().unwrap_or_default();
            let candidate1 = cwd
                .join("crates")
                .join("kanari-frameworks")
                .join("packages")
                .join("kanari-system")
                .join("build")
                .join("KanariSystem")
                .join("bytecode_modules");

            if candidate1.exists() {
                candidate1
            } else if let Some(parent) = cwd.parent() {
                parent
                    .join("crates")
                    .join("kanari-frameworks")
                    .join("packages")
                    .join("kanari-system")
                    .join("build")
                    .join("KanariSystem")
                    .join("bytecode_modules")
            } else {
                candidate1
            }
        };

        println!("✓ Looking for Kanari system modules at: {:?}", modules_dir);

        if !modules_dir.exists() {
            eprintln!(
                "Warning: Kanari system modules not found at {:?}",
                modules_dir
            );
            eprintln!(
                "System modules will not be pre-loaded. You may need to publish them manually."
            );
            eprintln!();
            eprintln!("To fix this:");
            eprintln!("  cd crates/kanari-frameworks");
            eprintln!("  sui move build -p packages/kanari-system");
            return Ok(());
        }

        // List of system modules to load in dependency order
        // Load system modules in dependency order. Note: some modules (coin) depend on transfer,
        // so transfer must be published before coin.
        let module_files = vec![
            "tx_context.mv",
            "object.mv",
            "url.mv",
            "balance.mv",
            "transfer.mv",
            // Deny-list must be published before coin which references it
            "deny_list.mv",
            "coin.mv",
            "kanari.mv",
            // Crypto modules (these are wrappers for native functions)
            "ecdsa_k1.mv",
            "ecdsa_r1.mv",
            "ed25519.mv",
        ];

        let system_addr = AccountAddress::from_hex_literal(KanariAddress::KANARI_SYSTEM_ADDRESS)?;
        let mut count = 0;

        for module_file in module_files {
            let module_path = modules_dir.join(module_file);

            if let Ok(module_bytes) = std::fs::read(&module_path) {
                // Publish module silently (no gas accounting for system modules)
                match self.publish_module(module_bytes.clone(), system_addr, None) {
                    Ok(_) => count += 1,
                    Err(e) => {
                        // Silently skip already loaded modules
                        if !e.to_string().contains("already exists") {
                            eprintln!("Warning: Failed to load {}: {}", module_file, e);
                        }
                    }
                }
            } else {
                eprintln!("Warning: Module file not found: {:?}", module_path);
            }
        }

        println!("✓ Loaded {} kanari-system modules (0x2::*)", count);
        Ok(())
    }
}
