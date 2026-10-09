// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Native Rust formatter for the Move language.
//!
//! Implements the same formatting rules as the `prettier-plugin-move`
//! TypeScript implementation (used by the Move on Kanari VS Code extension)
//! on top of the tree-sitter Move grammar.

mod printer;

/// Default tab width used when rendering indentation.
pub const DEFAULT_TAB_WIDTH: usize = 4;

/// Default maximum line width before lists are broken across lines.
pub const DEFAULT_PRINT_WIDTH: usize = 100;

/// Formatting options.
#[derive(Clone, Copy, Debug)]
pub struct FormatOptions {
    /// Spaces per indentation level.
    pub tab_width: usize,
    /// Maximum line width before breaking lists.
    pub print_width: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            tab_width: DEFAULT_TAB_WIDTH,
            print_width: DEFAULT_PRINT_WIDTH,
        }
    }
}

/// Format Move `source` according to `options`.
///
/// The output ends with exactly one trailing newline, matching the
/// TypeScript implementation.
pub fn format_source(source: &str, options: &FormatOptions) -> anyhow::Result<String> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_move::language();
    parser
        .set_language(&language)
        .map_err(|e| anyhow::anyhow!("failed to load Move grammar: {e:?}"))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("failed to parse Move source"))?;
    let root = tree.root_node();
    let doc = printer::print_node(&root, source, options);
    let mut rendered = String::new();
    doc.render_fmt(options.print_width, &mut rendered)
        .map_err(|e| anyhow::anyhow!("failed to render formatted source: {e}"))?;
    // The `pretty` renderer keeps indentation on otherwise blank lines;
    // strip it so blank lines are truly empty (matching prettier output).
    let cleaned: Vec<&str> = rendered.split('\n').map(str::trim_end).collect();
    let trimmed = cleaned
        .join("\n")
        .trim_end_matches(['\n', '\r'])
        .to_string();
    Ok(format!("{trimmed}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_fun_children() {
        let src =
            "module m::n {\n    fun g<$T: key, phantom U, V>(x: $T): $T {\n        x\n    }\n}";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_move::language()).unwrap();
        let tree = parser.parse(src, None).unwrap();
        let root = tree.root_node();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if node.kind() == "type_parameter" {
                eprintln!("== type_parameter ==");
                let mut c = node.walk();
                for child in node.children(&mut c) {
                    let txt: String = child
                        .utf8_text(src.as_bytes())
                        .unwrap_or("")
                        .chars()
                        .take(16)
                        .collect();
                    eprintln!(
                        "tparamchild: kind={} named={} text={:?}",
                        child.kind(),
                        child.is_named(),
                        txt
                    );
                }
            }
            let mut cc = node.walk();
            for child in node.children(&mut cc) {
                stack.push(child);
            }
        }
    }

    /// Formats the shared fixtures also used by the TypeScript
    /// `prettier-plugin-move` suite, asserting byte-identical output.
    /// (Line endings are normalized: fixtures are stored with LF.)
    fn check_fixture(name: &str) {
        let dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../third_party/move/crates/move-analyzer/prettier-plugin/tests/"
        );
        let input = std::fs::read_to_string(format!("{dir}{name}/test.move"))
            .unwrap_or_else(|_| panic!("missing fixture {name}/test.move"));
        let expected = std::fs::read_to_string(format!("{dir}{name}/test.exp"))
            .unwrap_or_else(|_| panic!("missing fixture {name}/test.exp"));
        let out = format_source(
            &input,
            &FormatOptions {
                print_width: 80,
                ..FormatOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            out,
            expected.replace("\r\n", "\n"),
            "fixture {name} mismatch"
        );
    }

    #[test]
    fn fixture_constants() {
        check_fixture("constants");
    }

    #[test]
    fn fixture_empty_module() {
        check_fixture("empty_module");
    }

    #[test]
    fn fixture_functions() {
        check_fixture("functions");
    }

    #[test]
    fn fixture_structs() {
        check_fixture("structs");
    }

    #[test]
    fn formats_grouped_uses() {
        let src = "module james::euro {\n    use kanari_system::coin;\n    use kanari_system::transfer;\n    struct EURO has drop {}\n}";
        let out = format_source(src, &FormatOptions::default()).unwrap();
        let expected = "module james::euro {\n    use kanari_system::coin;\n    use kanari_system::transfer;\n\n    struct EURO has drop {}\n}\n";
        assert_eq!(out, expected);
    }

    /// Every sample in the vendored grammar corpus must parse without
    /// errors and format idempotently (formatting twice yields the same
    /// output).
    #[test]
    fn vendored_grammar_samples() {
        let dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../third_party/tree-sitter-move/tests/"
        );
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .expect("grammar tests dir missing")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".move"))
            .collect();
        names.sort();
        assert!(!names.is_empty(), "no grammar samples found");
        for name in &names {
            let source = std::fs::read_to_string(format!("{dir}{name}")).expect("sample missing");
            let mut parser = tree_sitter::Parser::new();
            parser.set_language(&tree_sitter_move::language()).unwrap();
            let tree = parser.parse(&source, None).expect("parse failed");
            assert!(
                !tree.root_node().has_error(),
                "sample {name} has parse errors"
            );
            let once = format_source(&source, &FormatOptions::default()).unwrap();
            let twice = format_source(&once, &FormatOptions::default()).unwrap();
            assert_eq!(once, twice, "sample {name} is not idempotent");
        }
    }
}
