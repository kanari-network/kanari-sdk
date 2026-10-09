// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

import type { AstPath, Doc, ParserOptions } from 'prettier';
import * as prettier from 'prettier';
import { Node as SyntaxNode } from 'web-tree-sitter'

const { hardline, indent, join, line, softline, group, ifBreak } = prettier.doc.builders;

type printFn = (path: AstPath) => Doc;

export function print(path: AstPath, options: ParserOptions, print: printFn) {
    const node = path.getValue()

    switch (node.type) {
        case 'source_file':
            // Top level (file header comments, modules): preserve the
            // author's blank lines like statement blocks do.
            return printMembers(path, node, print, statementSeparator, true);
        case 'module_definition':
            // A module may carry leading attributes (e.g. `#[test_only] module ...`);
            // print them first so the identity/body that follow stay correct.
            // (Printing by fixed indices here used to merge the attribute into
            // the module line and drop the whole module body.)
            const modChildren = node.namedChildren;
            const modParts: Doc[] = [];
            let modIndex = 0;
            while (modIndex < modChildren.length) {
                const modChild = modChildren[modIndex];
                if (modChild === undefined || (modChild.type !== 'annotation' && modChild.type !== 'line_comment')) {
                    break;
                }
                modParts.push(path.call(print, 'namedChildren', modIndex));
                modParts.push(hardline);
                modIndex++;
            }
            modParts.push('module ');
            modParts.push(path.call(print, 'namedChildren', modIndex)); // module_identity
            modIndex++;
            modParts.push(' ');
            while (modIndex < modChildren.length) {
                modParts.push(path.call(print, 'namedChildren', modIndex));
                modIndex++;
            }
            modParts.push(hardline);
            return modParts;
        case 'module_identity':
            return [
                path.call(print, 'namedChildren', 0),
                '::',
                path.call(print, 'namedChildren', 1)
            ];
        case 'module_body':
            if (node.children.length == 2) {
                // empty module (the only children are curlies)
                return [ '{}' ];
            } else {
                return [
                    '{',
                    indent([hardline, printMembers(path, node, print, (p, c, n) => memberSeparator(p, c, n), true)]),
                    hardline,
                    '}'
                ];
            }
        case 'constant':
            // break and indent only on the equal sign so long form looks as follows:
            //
            // const c: u64 =
            //    42;
            return group([
                'const ',
                path.call(print, 'namedChildren', 0),
                ': ',
                path.call(print, 'namedChildren', 1),
                ' =',
                indent([line, path.call(print, 'namedChildren', 2)]),
                ';',
            ]);
        case 'struct_definition':
            // type parameters are on separate lines if they don't fit on one, but fields are always
            // on separate lines:
            //
            // struct SomeStruct<T1: key, T2: drop> has key {
            //     f: u64,
            // }
            //
            // struct AnotherStruct<
            //     T1: store + drop + key,
            //     T2: store + drop + key,
            //     T3: store + drop + key,
            // > has key, store {
            //     f1: u64,
            //     f2: u64,
            // }
            return [
                node.child(0).type === 'public' ? 'public ' : '',
                'struct ',
                path.call(print, 'namedChildren', 0),
                path.call(print, 'namedChildren', 1),
                path.call(print, 'namedChildren', 2),
                path.call(print, 'namedChildren', 3),
                path.call(print, 'namedChildren', 4),
            ]
        case 'native_struct_definition':
            // same formatting as "regular" struct but (of course) without fields
            return [
                'struct ',
                path.call(print, 'namedChildren', 0),
                path.call(print, 'namedChildren', 1),
                path.call(print, 'namedChildren', 2),
                ';',
            ]
        case 'function_definition':
            let is_entry = false;
            for (let i = 0; i < node.childCount; i++) {
                if (node.child(i).type === 'entry') {
                    is_entry = true;
                }
            }
            // first named child may be a visibility modifier
            return [
                node.namedChild(0).type === 'visibility_modifier' ? [ path.call(print, 'namedChildren', 0), ' '] : '',
                is_entry ? 'entry ' : '',
                'fun ',
                node.namedChild(0).type !== 'visibility_modifier' ? path.call(print, 'namedChildren', 0) : '',
                path.call(print, 'namedChildren', 1),
                path.call(print, 'namedChildren', 2),
                path.call(print, 'namedChildren', 3),
                path.call(print, 'namedChildren', 4),
                path.call(print, 'namedChildren', 5),
            ];
        case 'native_function_definition':
            // first named child may be a visibility modifier
            return [
                node.namedChild(0).type === 'visibility_modifier' ? [ path.call(print, 'namedChildren', 0), ' '] : '',
                'native ',
                'fun ',
                node.namedChild(0).type !== 'visibility_modifier' ? path.call(print, 'namedChildren', 0) : '',
                path.call(print, 'namedChildren', 1),
                path.call(print, 'namedChildren', 2),
                path.call(print, 'namedChildren', 3),
                path.call(print, 'namedChildren', 4),
                ';',
            ];
        // TODO: do macros
        case 'ability_decls':
            return [
                ' has ',
                join(', ', path.map(print, 'namedChildren'))
            ];
        case 'postfix_ability_decls':
            return [
                ' has ',
                join(', ', path.map(print, 'namedChildren')),
                ';',
            ];
        case 'type_parameters':
            return breakable_comma_separated_list(path, node, '<', '>', print);
        case 'type_parameter':
            let abilities = [];
            for (let i = 1; i < node.namedChildCount; i++) {
                abilities.push(path.call(print, 'namedChildren', i));
            }
            return [
                // '$' and 'phantom' are mutually exclusive (one for macros and the other for structs)
                node.child(0).type === '$' ? '$' : (node.child(0).type === 'phantom' ? 'phantom ' : ''),
                path.call(print, 'firstNamedChild'),
                node.namedChildren.length > 1 ? ': ' : '' ,
                join(' + ', abilities),
            ];
        case 'datatype_fields':
            return path.call(print, 'firstNamedChild');
        case 'named_fields':
            return block(path, node, print, ',');
        case 'field_annotation':
            return [
                path.call(print, 'namedChildren', 0),
                ': ',
                path.call(print, 'namedChildren', 1),
            ];
        case 'positional_fields':
            return breakable_comma_separated_list(path, node, '(', ')', print);
        case 'block':
            return block(path, node, print, '');
        case 'function_parameters': {
            // Preserve the author's layout: if the parenthesis pair spans
            // multiple source lines, keep one parameter per line; otherwise
            // follow the usual group (fit-or-break) rules. An empty list
            // always collapses to `()`.
            const multiline =
                node.namedChildren.length > 0 &&
                node.endPosition.row > node.startPosition.row;
            return breakable_comma_separated_list(path, node, '(', ')', print, multiline);
        }
        case 'function_parameter':
            return [
                path.call(print, 'namedChildren', 0),
                ': ',
                path.call(print, 'namedChildren', 1),
            ];
        case 'ret_type':
            return [ ': ', path.call(print, 'namedChildren', 0) ];
        case 'struct_identifier':
        case 'ability':
        case 'type_parameter_identifier':
        case 'field_identifier':
        case 'function_identifier':
        case 'variable_identifier':
        default:
            return node.text;
    }
}

function isTightMember(type: string): boolean {
    // Module members printed without a separating blank line when adjacent
    // to each other (import/use groups and their comments).
    return type === 'use_declaration' || type === 'line_comment';
}

function memberSeparator(
    prev: SyntaxNode | undefined,
    curr: SyntaxNode,
    next: SyntaxNode | undefined,
): Doc {
    // Attributes and comments stay attached to the item that follows them.
    if (prev !== undefined && prev.type === 'annotation') {
        return hardline;
    }
    if (prev !== undefined && prev.type === 'line_comment') {
        if (curr.type === 'line_comment') {
            // Preserve the author's blank lines between comments (at most one).
            return curr.startPosition.row - prev.endPosition.row > 1 ? [hardline, hardline] : hardline;
        }
        return hardline;
    }
    // A comment that starts a documented block gets a blank line before it,
    // except when it sits inside a `use` group.
    if (
        curr.type === 'line_comment' &&
        (next === undefined || next.type !== 'use_declaration')
    ) {
        return [hardline, hardline];
    }
    // Consecutive `use` declarations and consecutive `const` declarations
    // stay grouped without blank lines (but a blank line still separates
    // a `use` group from a `const` group and vice versa).
    if (
        prev !== undefined &&
        prev.type === curr.type &&
        (curr.type === 'use_declaration' || curr.type === 'constant')
    ) {
        return hardline;
    }
    if (
        prev !== undefined &&
        isTightMember(prev.type) &&
        isTightMember(curr.type)
    ) {
        return hardline;
    }
    return [hardline, hardline];
}

function statementSeparator(prev: SyntaxNode | undefined, curr: SyntaxNode, _next: SyntaxNode | undefined): Doc {
    // Preserve the author's blank lines between statements (at most one),
    // as they mark logical groups. Collapse runs of blank lines into one.
    // `prev` is always defined here (the separator is only used past the
    // first member).
    return curr.startPosition.row - prev!.endPosition.row > 1 ? [hardline, hardline] : hardline;
}

function printMembers(
    path: AstPath,
    node: SyntaxNode,
    print: printFn,
    separator: (prev: SyntaxNode | undefined, curr: SyntaxNode, next: SyntaxNode | undefined) => Doc,
    inlineTrailingComments: boolean,
): Doc[] {
    const members = node.namedChildren;
    const parts: Doc[] = [];
    let prev: SyntaxNode | undefined;
    for (let i = 0; i < members.length; i++) {
        const member = members[i];
        if (member === undefined) {
            continue;
        }
        const next = i + 1 < members.length ? members[i + 1] : undefined;
        if (prev !== undefined) {
            parts.push(separator(prev, member, next));
        }
        parts.push(path.call(print, 'namedChildren', i));
        prev = member;
        // Keep a trailing same-line comment on the member's own line instead
        // of moving it to a line of its own.
        if (
            inlineTrailingComments &&
            next !== undefined &&
            next.type === 'line_comment' &&
            next.startPosition.row === member.endPosition.row
        ) {
            parts.push(' ' + next.text);
            i++;
        }
    }
    return parts;
}

function needsComma(node: SyntaxNode): boolean {
    // Commas belong after code items, never after comments or attributes.
    return node.type !== 'line_comment' && node.type !== 'annotation';
}

function breakable_comma_separated_list(path: AstPath,
                                        node: SyntaxNode,
                                        start: string,
                                        end: string,
                                        print: printFn,
                                        forceBreak = false) {

    const items = Symbol('items');
    const separator: Doc = forceBreak ? hardline : line;
    const trailing: Doc = forceBreak ? hardline : softline;
    const children = node.namedChildren;
    const parts: Doc[] = [];
    children.forEach((child, index) => {
        if (child === undefined) {
            return;
        }
        if (index > 0) {
            const prev = children[index - 1];
            if (prev !== undefined && needsComma(prev)) {
                parts.push(',');
            }
            parts.push(separator);
        }
        parts.push(path.call(print, 'namedChildren', index));
    });
    const last = children.length > 0 ? children[children.length - 1] : undefined;
    const trailingComma: Doc = last !== undefined && needsComma(last) ? [',', trailing] : trailing;
    return [
        start,
        group([
            indent(forceBreak ? hardline : softline),
            indent(parts),
        ], {id: items}),
        ifBreak(trailingComma, '', {groupId: items}),
        end,
    ];
}

function block(path: AstPath, node: SyntaxNode, print: printFn, line_ending: string) {
    if (node.namedChildren.length == 0) {
        return ' {}';
    }
    if (line_ending === '') {
        // Statement block (e.g. function body): the author's blank lines
        // between statements are preserved (at most one) and trailing
        // same-line comments stay on their own statement's line.
        return [
            ' {',
            indent(hardline),
            indent(printMembers(path, node, print, statementSeparator, true)),
            hardline,
            '}',
        ];
    }
    // Comma-separated block (e.g. struct fields): commas go after code
    // items only, never after comments or attributes.
    const members = node.namedChildren;
    const parts: Doc[] = [];
    members.forEach((member, index) => {
        if (member === undefined) {
            return;
        }
        if (index > 0) {
            parts.push(hardline);
        }
        parts.push(path.call(print, 'namedChildren', index));
        if (needsComma(member)) {
            parts.push(line_ending);
        }
    });
    return [
        ' {',
        indent(hardline),
        indent(parts),
        hardline,
        '}',
    ];
}
