// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Printer: converts a tree-sitter Move syntax tree into a pretty document.
//!
//! The Rust formatter is the source of truth for Move formatting rules; the
//! TypeScript `prettier-plugin-move` under `third_party` is no longer the
//! reference. The one policy that shapes most of this file: blank lines are
//! preserved exactly as authored — between two items the author's line
//! breaks are reproduced verbatim, and the formatter never adds, removes or
//! collapses blank lines.

use pretty::RcDoc;
use tree_sitter::Node;

use super::FormatOptions;

pub type Doc<'s> = RcDoc<'s>;

fn text<'s>(s: impl Into<std::borrow::Cow<'s, str>>) -> Doc<'s> {
    RcDoc::text(s)
}

fn node_text<'s, 't>(node: &Node<'t>, source: &'s str) -> &'s str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}

fn is_content(node: &Node) -> bool {
    // The vendored grammar emits `newline` nodes that carry no content.
    // Filter them everywhere so positional and named access matches the
    // logical tree shape the formatting rules were written against.
    node.kind() != "newline"
}

fn content_children<'t>(node: &Node<'t>) -> Vec<Node<'t>> {
    let mut out = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if is_content(&child) {
            out.push(child);
        }
    }
    out
}

fn named_children<'t>(node: &Node<'t>) -> Vec<Node<'t>> {
    content_children(node)
        .into_iter()
        .filter(|child| child.is_named())
        .collect()
}

fn content_child<'t>(node: &Node<'t>, index: usize) -> Option<Node<'t>> {
    content_children(node).into_iter().nth(index)
}

fn content_named_child<'t>(node: &Node<'t>, index: usize) -> Option<Node<'t>> {
    named_children(node).into_iter().nth(index)
}

fn content_named_count(node: &Node) -> usize {
    named_children(node).len()
}

fn child_kind<'t>(node: &Node<'t>, index: usize) -> Option<&'t str> {
    content_child(node, index).map(|c| c.kind())
}

fn print_named<'s, 't>(
    node: &Node<'t>,
    source: &'s str,
    opts: &FormatOptions,
    index: usize,
) -> Doc<'s> {
    match content_named_child(node, index) {
        Some(child) => print_node(&child, source, opts),
        None => RcDoc::nil(),
    }
}

fn first_named<'s, 't>(node: &Node<'t>, source: &'s str, opts: &FormatOptions) -> Doc<'s> {
    print_named(node, source, opts, 0)
}

fn concat<'s>(docs: Vec<Doc<'s>>) -> Doc<'s> {
    docs.into_iter().fold(RcDoc::nil(), |acc, d| acc.append(d))
}

fn join<'s>(separator: Doc<'s>, docs: Vec<Doc<'s>>) -> Doc<'s> {
    RcDoc::intersperse(docs, separator)
}

fn indent<'s>(doc: Doc<'s>, opts: &FormatOptions) -> Doc<'s> {
    doc.nest(opts.tab_width as isize)
}

fn needs_comma(kind: &str) -> bool {
    kind != "line_comment" && kind != "annotation"
}

/// Number of line breaks that reproduce the gap between the row an item ends
/// on and the row the next item starts on: the author's blank lines are kept
/// verbatim (never collapsed into one), and two items are never joined onto
/// a single line, hence the minimum of one break.
fn breaks(from_end_row: usize, to_start_row: usize) -> usize {
    to_start_row.saturating_sub(from_end_row).max(1)
}

/// `count` hardline breaks (used to replay the author's blank lines).
fn hardlines<'s>(count: usize) -> Vec<Doc<'s>> {
    (0..count).map(|_| RcDoc::hardline()).collect()
}

fn print_members<'s, 't>(
    node: &Node<'t>,
    source: &'s str,
    opts: &FormatOptions,
    inline_trailing_comments: bool,
) -> Vec<Doc<'s>> {
    let members = named_children(node);
    let mut parts: Vec<Doc<'s>> = Vec::new();
    let mut prev: Option<Node> = None;
    let mut i = 0;
    while i < members.len() {
        let member = &members[i];
        let next = members.get(i + 1);
        if let Some(p) = &prev {
            // Replay the blank lines (if any) the author left between the two
            // members exactly as written.
            parts.extend(hardlines(breaks(
                p.end_position().row,
                member.start_position().row,
            )));
        }
        parts.push(print_named(node, source, opts, i));
        prev = Some(*member);
        if inline_trailing_comments
            && let Some(n) = next
            && n.kind() == "line_comment"
            && n.start_position().row == member.end_position().row
        {
            parts.push(text(format!(" {}", node_text(n, source))));
            i += 2;
            continue;
        }
        i += 1;
    }
    parts
}

fn breakable_comma_separated_list<'s, 't>(
    node: &Node<'t>,
    source: &'s str,
    opts: &FormatOptions,
    start: &'static str,
    end: &'static str,
    force_break: bool,
) -> Doc<'s> {
    let separator: Doc<'s> = if force_break {
        RcDoc::hardline()
    } else {
        RcDoc::line()
    };
    let trailing: Doc<'s> = if force_break {
        RcDoc::hardline()
    } else {
        RcDoc::line_()
    };
    let children = named_children(node);
    let mut parts: Vec<Doc<'s>> = Vec::new();
    for (index, _child) in children.iter().enumerate() {
        if index > 0 {
            if let Some(prev) = children.get(index - 1)
                && needs_comma(prev.kind())
            {
                parts.push(text(","));
            }
            parts.push(separator.clone());
        }
        parts.push(print_named(node, source, opts, index));
    }
    // Trailing comma only when the group breaks (mirrors prettier's
    // `ifBreak` on the items group by nesting it inside the group).
    let trailing_comma: Doc<'s> = match children.last() {
        Some(last) if needs_comma(last.kind()) => {
            concat(vec![text(","), trailing]).flat_alt(RcDoc::nil())
        }
        _ => trailing.flat_alt(RcDoc::nil()),
    };
    let inner = if force_break {
        RcDoc::hardline()
    } else {
        RcDoc::line_()
    };
    concat(vec![
        text(start),
        concat(vec![
            indent(concat(vec![inner, concat(parts)]), opts),
            trailing_comma,
        ])
        .group(),
        text(end),
    ])
}

fn block<'s, 't>(
    node: &Node<'t>,
    source: &'s str,
    opts: &FormatOptions,
    line_ending: &'static str,
) -> Doc<'s> {
    let members = named_children(node);
    if members.is_empty() {
        return text(" {}");
    }
    // Blank lines right after `{` and right before `}` are part of the
    // author's layout and are reproduced verbatim.
    let leading = breaks(node.start_position().row, members[0].start_position().row);
    let trailing = breaks(
        members[members.len() - 1].end_position().row,
        node.end_position().row,
    );
    if line_ending.is_empty() {
        return concat(vec![
            text(" {"),
            indent(
                concat(vec![
                    concat(hardlines(leading)),
                    concat(print_members(node, source, opts, true)),
                ]),
                opts,
            ),
            concat(hardlines(trailing)),
            text("}"),
        ]);
    }
    // Comma-separated block (e.g. struct fields): the author's blank lines
    // between fields are preserved the same way.
    let mut parts: Vec<Doc<'s>> = Vec::new();
    for (index, member) in members.iter().enumerate() {
        if index > 0 {
            parts.extend(hardlines(breaks(
                members[index - 1].end_position().row,
                member.start_position().row,
            )));
        }
        parts.push(print_named(node, source, opts, index));
        if needs_comma(member.kind()) {
            parts.push(text(line_ending));
        }
    }
    concat(vec![
        text(" {"),
        indent(
            concat(vec![concat(hardlines(leading)), concat(parts)]),
            opts,
        ),
        concat(hardlines(trailing)),
        text("}"),
    ])
}

pub fn print_node<'s, 't>(node: &Node<'t>, source: &'s str, opts: &FormatOptions) -> Doc<'s> {
    match node.kind() {
        "source_file" => concat(print_members(node, source, opts, true)),
        "module_definition" => {
            let children = named_children(node);
            let mut parts: Vec<Doc<'s>> = Vec::new();
            let mut index = 0;
            while index < children.len() {
                let done = !matches!(
                    children.get(index),
                    Some(c) if c.kind() == "annotation" || c.kind() == "line_comment"
                );
                if done {
                    break;
                }
                parts.push(print_named(node, source, opts, index));
                parts.push(RcDoc::hardline());
                index += 1;
            }
            parts.push(text("module "));
            parts.push(print_named(node, source, opts, index));
            index += 1;
            parts.push(text(" "));
            while index < children.len() {
                parts.push(print_named(node, source, opts, index));
                index += 1;
            }
            // No trailing hardline here: the line breaks between top-level
            // items come from the source gap alone (adding one here would
            // make every re-format grow a blank line).
            concat(parts)
        }
        "module_identity" => concat(vec![
            print_named(node, source, opts, 0),
            text("::"),
            print_named(node, source, opts, 1),
        ]),
        "module_body" => {
            let members = named_children(node);
            if members.is_empty() {
                // empty module (the only content children are the curlies)
                text("{}")
            } else {
                // Blank lines right after `{` and right before `}` belong to
                // the author's layout and are reproduced verbatim.
                let leading = breaks(node.start_position().row, members[0].start_position().row);
                let trailing = breaks(
                    members[members.len() - 1].end_position().row,
                    node.end_position().row,
                );
                concat(vec![
                    text("{"),
                    indent(
                        concat(vec![
                            concat(hardlines(leading)),
                            concat(print_members(node, source, opts, true)),
                        ]),
                        opts,
                    ),
                    concat(hardlines(trailing)),
                    text("}"),
                ])
            }
        }
        "constant" => concat(vec![
            text("const "),
            print_named(node, source, opts, 0),
            text(": "),
            print_named(node, source, opts, 1),
            text(" ="),
            indent(
                concat(vec![RcDoc::line(), print_named(node, source, opts, 2)]),
                opts,
            ),
            text(";"),
        ])
        .group(),
        "struct_definition" => concat(vec![
            match child_kind(node, 0) {
                Some("public") => text("public "),
                _ => RcDoc::nil(),
            },
            text("struct "),
            print_named(node, source, opts, 0),
            print_named(node, source, opts, 1),
            print_named(node, source, opts, 2),
            print_named(node, source, opts, 3),
            print_named(node, source, opts, 4),
        ]),
        "native_struct_definition" => concat(vec![
            text("struct "),
            print_named(node, source, opts, 0),
            print_named(node, source, opts, 1),
            print_named(node, source, opts, 2),
            text(";"),
        ]),
        "function_definition" => {
            // Leading `modifier` nodes (`public`, `entry`) precede the rest.
            // They are normalized to `public entry` order, matching the
            // TypeScript implementation.
            let named = named_children(node);
            let mut index = 0;
            let mut is_public = false;
            let mut is_entry = false;
            let mut extras: Vec<Doc> = Vec::new();
            while index < named.len() && named[index].kind() == "modifier" {
                match node_text(&named[index], source) {
                    "public" => is_public = true,
                    "entry" => is_entry = true,
                    other => extras.push(text(format!("{other} "))),
                }
                index += 1;
            }
            let mut parts: Vec<Doc> = Vec::new();
            if is_public {
                parts.push(text("public "));
            }
            if is_entry {
                parts.push(text("entry "));
            }
            parts.extend(extras);
            parts.push(text("fun "));
            while index < named.len() {
                parts.push(print_named(node, source, opts, index));
                index += 1;
            }
            concat(parts)
        }
        "native_function_definition" => {
            // Like functions, but with a mandatory `native` modifier that is
            // printed literally.
            let named = named_children(node);
            let mut index = 0;
            let mut prefix: Vec<Doc> = Vec::new();
            while index < named.len() && named[index].kind() == "modifier" {
                match node_text(&named[index], source) {
                    "public" => prefix.push(text("public ")),
                    "native" => {}
                    other => prefix.push(text(format!("{other} "))),
                }
                index += 1;
            }
            let mut parts = prefix;
            parts.push(text("native "));
            parts.push(text("fun "));
            while index < named.len() {
                parts.push(print_named(node, source, opts, index));
                index += 1;
            }
            parts.push(text(";"));
            concat(parts)
        }
        "ability_decls" => concat(vec![
            text(" has "),
            join(
                text(", "),
                named_children(node)
                    .iter()
                    .enumerate()
                    .map(|(i, _)| print_named(node, source, opts, i))
                    .collect(),
            ),
        ]),
        "postfix_ability_decls" => concat(vec![
            text(" has "),
            join(
                text(", "),
                named_children(node)
                    .iter()
                    .enumerate()
                    .map(|(i, _)| print_named(node, source, opts, i))
                    .collect(),
            ),
            text(";"),
        ]),
        "type_parameters" => breakable_comma_separated_list(node, source, opts, "<", ">", false),
        "type_parameter" => {
            let count = content_named_count(node);
            let mut abilities = Vec::new();
            for i in 1..count {
                abilities.push(print_named(node, source, opts, i));
            }
            let first_is_dollar = child_kind(node, 0) == Some("$");
            let first_is_phantom = child_kind(node, 0) == Some("phantom");
            concat(vec![
                if first_is_dollar {
                    text("$")
                } else if first_is_phantom {
                    text("phantom ")
                } else {
                    RcDoc::nil()
                },
                first_named(node, source, opts),
                if named_children(node).len() > 1 {
                    text(": ")
                } else {
                    RcDoc::nil()
                },
                join(text(" + "), abilities),
            ])
        }
        "datatype_fields" => first_named(node, source, opts),
        "named_fields" => block(node, source, opts, ","),
        "field_annotation" => concat(vec![
            print_named(node, source, opts, 0),
            text(": "),
            print_named(node, source, opts, 1),
        ]),
        "positional_fields" => breakable_comma_separated_list(node, source, opts, "(", ")", false),
        "block" => block(node, source, opts, ""),
        "function_parameters" => {
            let children = named_children(node);
            let multiline =
                !children.is_empty() && node.end_position().row > node.start_position().row;
            breakable_comma_separated_list(node, source, opts, "(", ")", multiline)
        }
        "function_parameter" => concat(vec![
            print_named(node, source, opts, 0),
            text(": "),
            print_named(node, source, opts, 1),
        ]),
        "ret_type" => concat(vec![text(": "), print_named(node, source, opts, 0)]),
        "struct_identifier"
        | "ability"
        | "type_parameter_identifier"
        | "field_identifier"
        | "function_identifier"
        | "variable_identifier" => text(node_text(node, source)),
        _ => text(node_text(node, source)),
    }
}
