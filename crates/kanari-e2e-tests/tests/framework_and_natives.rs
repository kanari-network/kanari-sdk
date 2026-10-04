//! Move Frameworks & System Natives
//!
//! Verify:
//! 1. Framework package configs (MoveStdlib and KanariSystem) have correct addresses/deps
//! 2. System natives table is populated with required modules

use kanari_frameworks::packages_config::get_package_configs;
use kanari_system_natives::{GasParameters, all_natives};
use kanari_types::address::Address;

#[test]
fn framework_packages_defined_and_linked() {
    let configs = get_package_configs();
    assert!(configs.len() >= 2, "must have at least 2 package configs");

    let stdlib = configs
        .iter()
        .find(|c| c.name == "MoveStdlib")
        .expect("must have MoveStdlib");
    assert!(stdlib.is_stdlib());
    assert_eq!(stdlib.address, Address::STD_ADDRESS);
    assert!(stdlib.get_dependencies().is_empty());

    let system = configs
        .iter()
        .find(|c| c.name == "KanariSystem")
        .expect("must have KanariSystem");
    assert!(!system.is_stdlib());
    assert_eq!(system.address, Address::KANARI_SYSTEM_ADDRESS);
    assert_eq!(system.get_dependencies(), vec!["move-stdlib"]);
}

#[test]
fn system_natives_table_populated() {
    let system_addr = Address::kanari_system_account_address();

    let table_zeros = all_natives(system_addr, GasParameters::zeros());
    let table_prod = all_natives(system_addr, GasParameters::production());

    assert!(
        !table_zeros.is_empty(),
        "native function table must not be empty"
    );
    assert_eq!(
        table_zeros.len(),
        table_prod.len(),
        "native count must match between zeros and production"
    );

    let module_names: std::collections::HashSet<String> = table_zeros
        .iter()
        .map(|(_, mod_name, _, _)| mod_name.as_str().to_string())
        .collect();

    for required in ["base64", "ed25519", "dilithium3", "transfer", "tx_context"] {
        assert!(
            module_names.contains(required),
            "must have native module `{required}`"
        );
    }
}
