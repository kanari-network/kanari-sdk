// Copyright (c) Kanari Network
// SPDX-License-Identifier: Apache-2.0

use kanari_types::address::Address;

/// Package configuration
#[derive(Debug, Clone)]
pub struct PackageConfig {
    pub name: &'static str,
    pub directory: &'static str,
    pub address: &'static str,
    pub address_name: &'static str,
}

impl PackageConfig {
    /// Check if this package is stdlib
    pub fn is_stdlib(&self) -> bool {
        self.address == Address::STD_ADDRESS
    }

    /// Get dependencies for this package
    pub fn get_dependencies(&self) -> Vec<&'static str> {
        if self.is_stdlib() {
            Vec::new()
        } else {
            vec!["move-stdlib"]
        }
    }
}

/// All framework packages
const PACKAGES: &[PackageConfig] = &[
    PackageConfig {
        name: "MoveStdlib",
        directory: "move-stdlib",
        address: Address::STD_ADDRESS,
        address_name: "std",
    },
    PackageConfig {
        name: "KanariSystem",
        directory: "kanari-system",
        address: Address::KANARI_SYSTEM_ADDRESS,
        address_name: "kanari_system",
    },
    // เพิ่ม packages ใหม่ที่นี่:
    // PackageConfig { name: "MyPackage", directory: "my-package", address: "0x3", address_name: "my_package" },
];

pub fn get_package_configs() -> Vec<PackageConfig> {
    PACKAGES.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_configs() {
        let configs = get_package_configs();
        assert!(configs.len() >= 2);

        let stdlib = configs.iter().find(|p| p.name == "MoveStdlib").unwrap();
        assert_eq!(stdlib.directory, "move-stdlib");
        assert_eq!(stdlib.address, Address::STD_ADDRESS);

        let system = configs.iter().find(|p| p.name == "KanariSystem").unwrap();
        assert_eq!(system.directory, "kanari-system");
        assert_eq!(system.address, Address::KANARI_SYSTEM_ADDRESS);
    }
}
