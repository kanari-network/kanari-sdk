// Move 2024 edition support, ported from TS pure.ts hasPrelude().
// In 2024, `vector`, `option`, `Option` are provided by default (prelude),
// so `use std::vector;` etc. are redundant and must not be suggested.

/// Modules the 2024 prelude provides without any `use`.
const PRELUDE_MODULES: &[&str] = &["vector", "option"];
const PRELUDE_MEMBERS: &[&str] = &["Option"];

#[derive(Debug, Clone, PartialEq)]
pub enum MoveEdition {
    Legacy,
    Edition2024,
    Unknown,
}

impl MoveEdition {
    /// True when the 2024 prelude (`vector`, `option`, `Option`) is active.
    /// Missing `edition` → Legacy (no prelude).
    pub fn has_prelude(&self) -> bool {
        matches!(self, MoveEdition::Edition2024)
    }
    pub fn is_2024(&self) -> bool {
        matches!(self, MoveEdition::Edition2024)
    }
}

/// Parse `edition = "..."` from raw Move.toml text.
/// A leading UTF-8 BOM (`\u{FEFF}`) is skipped (Windows editors add it).
pub fn parse_edition(text: &str) -> MoveEdition {
    for line in text.lines() {
        let t = line.trim_start_matches('\u{FEFF}').trim();
        if let Some(rest) = t.strip_prefix("edition") {
            let rest = rest.trim_start_matches(['=', ' ', '\t']);
            if let Some(inner) = rest.strip_prefix('"').and_then(|r| r.split('"').next()) {
                return match inner {
                    "legacy" => MoveEdition::Legacy,
                    "2024" => MoveEdition::Edition2024,
                    _ => MoveEdition::Legacy,
                };
            }
        }
    }
    MoveEdition::Legacy
}

/// Read edition for the package holding `file` (walk up to Move.toml).
pub fn edition_for_file(file: &std::path::Path) -> MoveEdition {
    let Some(root) = crate::code_action::package_root(file) else {
        return MoveEdition::Legacy;
    };
    match std::fs::read_to_string(root.join("Move.toml")) {
        Ok(text) => parse_edition(&text),
        Err(_) => MoveEdition::Legacy,
    }
}

/// True when the 2024 prelude already provides `name` — so we should
/// not offer an import for it (it would only create a duplicate-alias
/// warning).
pub fn is_prelude(name: &str, edition: &MoveEdition) -> bool {
    edition.has_prelude() && (PRELUDE_MODULES.contains(&name) || PRELUDE_MEMBERS.contains(&name))
}

// ---------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_legacy() {
        let text = "[package]\nname = \"x\"\nedition = \"legacy\"\n";
        assert_eq!(parse_edition(text), MoveEdition::Legacy);
    }

    #[test]
    fn parse_2024() {
        let text = "[package]\nname = \"x\"\nedition = \"2024\"\n";
        assert_eq!(parse_edition(text), MoveEdition::Edition2024);
    }

    #[test]
    fn parse_missing_is_legacy() {
        // No `edition` key → treat as legacy (per Kanari convention).
        let text = "[package]\nname = \"x\"\n";
        assert_eq!(parse_edition(text), MoveEdition::Legacy);
        assert!(!MoveEdition::Legacy.has_prelude());
    }

    #[test]
    fn parse_with_spaces_and_comment() {
        // `edition = "2024" # comment` still parses as 2024.
        let text = "edition = \"2024\" # 2024 edition\n";
        assert_eq!(parse_edition(text), MoveEdition::Edition2024);
    }

    #[test]
    fn prelude_only_in_non_legacy() {
        assert!(is_prelude("vector", &MoveEdition::Edition2024));
        assert!(is_prelude("option", &MoveEdition::Edition2024));
        assert!(is_prelude("Option", &MoveEdition::Edition2024));
        assert!(!is_prelude("vector", &MoveEdition::Legacy));
        assert!(!is_prelude("TxContext", &MoveEdition::Edition2024));
    }

    #[test]
    fn parse_crlf_and_tabs() {
        assert_eq!(
            parse_edition("edition\t=\t\"2024\"\r\n"),
            MoveEdition::Edition2024
        );
        assert_eq!(parse_edition("edition=\"legacy\""), MoveEdition::Legacy);
        assert_eq!(parse_edition("edition = \"unknown\""), MoveEdition::Legacy);
        assert_eq!(parse_edition("edition = '2024'"), MoveEdition::Legacy);
    }

    #[test]
    fn is_prelude_case_sensitive() {
        assert!(!is_prelude("Vector", &MoveEdition::Edition2024));
        assert!(!is_prelude("VECTOR", &MoveEdition::Edition2024));
        assert!(!is_prelude(" vector ", &MoveEdition::Edition2024));
        assert!(!is_prelude("TxContext", &MoveEdition::Legacy));
    }

    #[test]
    fn edition_for_file_missing_toml_is_legacy() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("mod.move");
        std::fs::write(&file, "module x::m {}").unwrap();
        assert_eq!(edition_for_file(&file), MoveEdition::Legacy);
    }

    #[test]
    fn edition_for_file_walks_up() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = dir.path().join("pkg");
        std::fs::create_dir_all(pkg.join("sources")).unwrap();
        std::fs::write(
            pkg.join("Move.toml"),
            "[package]\nname=\"x\"\nedition=\"2024\"\n",
        )
        .unwrap();
        let file = pkg.join("sources").join("mod.move");
        std::fs::write(&file, "module x::m {}").unwrap();
        assert_eq!(edition_for_file(&file), MoveEdition::Edition2024);
    }
}
