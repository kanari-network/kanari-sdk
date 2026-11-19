// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use kanari_types::address::Address;
use move_compiler::editions::Edition;

/// Parse the `edition` field from a `Move.toml`, defaulting to legacy.
pub fn parse_move_edition(content: &str) -> Edition {
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("edition") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                let value = rest.trim().trim_matches('"').trim_matches('\'');
                return match value {
                    "2024" => Edition::E2024,
                    "2024.beta" => Edition::E2024_BETA,
                    "2024.alpha" => Edition::E2024_ALPHA,
                    "2024.migration" => Edition::E2024_MIGRATION,
                    "development" => Edition::DEVELOPMENT,
                    _ => Edition::LEGACY,
                };
            }
        }
    }
    Edition::LEGACY
}

/// Package configuration
#[derive(Debug, Clone)]
pub struct PackageConfig {
    pub name: &'static str,
    pub directory: &'static str,
    pub address: &'static str,
    pub address_name: &'static str,
    /// Framework package directories this package depends on (besides itself).
    pub dependencies: &'static [&'static str],
}

impl PackageConfig {
    /// Check if this package is stdlib
    pub fn is_stdlib(&self) -> bool {
        self.address == Address::STD_ADDRESS
    }

    /// Get dependencies for this package
    pub fn get_dependencies(&self) -> Vec<&'static str> {
        if self.is_stdlib() { Vec::new() } else { self.dependencies.to_vec() }
    }
}

/// All framework packages
const PACKAGES: &[PackageConfig] = &[
    PackageConfig {
        name: "MoveStdlib",
        directory: "move-stdlib",
        address: Address::STD_ADDRESS,
        address_name: "std",
        dependencies: &[],
    },
    PackageConfig {
        name: "KanariSystem",
        directory: "kanari-system",
        address: Address::KANARI_SYSTEM_ADDRESS,
        address_name: "kanari_system",
        dependencies: &["move-stdlib"],
    },
    PackageConfig {
        name: "KanariGovernance",
        directory: "kanari-governance",
        address: Address::KANARI_GOVERNANCE_ADDRESS,
        address_name: "kanari_governance",
        dependencies: &["move-stdlib", "kanari-system"],
    },
    // Add new packages here.
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
        assert!(configs.len() >= 3);

        let stdlib = configs.iter().find(|p| p.name == "MoveStdlib").unwrap();
        assert_eq!(stdlib.directory, "move-stdlib");
        assert_eq!(stdlib.address, Address::STD_ADDRESS);

        let system = configs.iter().find(|p| p.name == "KanariSystem").unwrap();
        assert_eq!(system.directory, "kanari-system");
        assert_eq!(system.address, Address::KANARI_SYSTEM_ADDRESS);

        let governance = configs.iter().find(|p| p.name == "KanariGovernance").unwrap();
        assert_eq!(governance.directory, "kanari-governance");
        assert_eq!(governance.address, Address::KANARI_GOVERNANCE_ADDRESS);
        assert!(governance.get_dependencies().contains(&"kanari-system"));
    }

    #[test]
    fn parse_move_edition_defaults_to_legacy() {
        assert_eq!(parse_move_edition(""), Edition::LEGACY);
        assert_eq!(parse_move_edition("[package]\nname = \"X\"\n"), Edition::LEGACY);
        assert_eq!(parse_move_edition("edition = \"legacy\""), Edition::LEGACY);
        assert_eq!(parse_move_edition("edition = \"unknown\""), Edition::LEGACY);
    }

    #[test]
    fn parse_move_edition_recognizes_2024_variants() {
        assert_eq!(parse_move_edition("edition = \"2024\""), Edition::E2024);
        assert_eq!(parse_move_edition("edition = \"2024.beta\""), Edition::E2024_BETA);
        assert_eq!(parse_move_edition("edition = \"2024.alpha\""), Edition::E2024_ALPHA);
        assert_eq!(parse_move_edition("edition = \"2024.migration\""), Edition::E2024_MIGRATION);
        assert_eq!(parse_move_edition("edition = \"development\""), Edition::DEVELOPMENT);
    }

    #[test]
    fn parse_move_edition_handles_whitespace_and_quotes() {
        assert_eq!(parse_move_edition("  edition   =   \"2024\"  "), Edition::E2024);
        assert_eq!(parse_move_edition("edition='2024'"), Edition::E2024);
    }
}
