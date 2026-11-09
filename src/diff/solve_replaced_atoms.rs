/*  This file is part of the OmniDiff code diffing tool.
 *
 *  Copyright (C) 2026 Marko Ivankovic
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, version 3 of the License.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

//! Phase 9b, after the leaf re-pairings: pairs an argument replaced in place. When a matched call
//! swaps one atomic argument for another in the same slot - `append(c)` becoming
//! `append("&#xa0;")` (`java-defects4j-jsoup-46-entities`), `';'` becoming `">;"`, an identifier
//! becoming `0` (`java-defects4j-cli-8-helpformatter`) - the reader sees that argument changed,
//! not one removed and an unrelated one added. The tree edit distance cannot say so: the two
//! differ in kind, or in shape (a leaf against a string with quotes and a fragment), and it
//! prices that above a delete and an insert.
//!
//! Only in argument lists (call and template arguments) and comparisons (`c < '0'` becoming
//! `c < 0x20`, `java-defects4j-csv-14-csvformat`). Elsewhere the hand-painted corpus reads
//! a replaced value as removed and added: an assignment's new right-hand side (`argName = "arg"`
//! becoming `argName = null`), a literal extracted into a constant (`3.14159` becoming `PI`), a
//! field's type made generic (`int` becoming `T`).
//!
//! A built-in type's keyword pairs with the keyword of the built-in type its wrapper was paired
//! with (`float toJavaVersionInt(..)` becoming `int toJavaVersionInt(..)`,
//! `java-defects4j-lang-29-systemutils`): each Java type keyword is a node kind of its own, so the
//! tree edit distance pairs the two wrappers and drops both keywords.
//!
//! The slot must be unambiguous: under a matched parent pair with as many named children on each
//! side, exactly one named child per side is wholly unpaired, at the same index. Both must be
//! atoms - a leaf, or a literal whose children are all leaves - and neither a comment. Neither
//! value may survive as an unpaired copy elsewhere on the other side, which makes it a move (a
//! literal extracted into a constant) rather than a replacement. Nor may a name have another
//! unpaired copy on its own side declared beside the other value: that is the constant the old
//! value moved into, even when the declaration's value was paired elsewhere
//! (`get_env("DISPLAY")` becoming `get_env(ENV_KEY_DISPLAY)` beside a new
//! `const ENV_KEY_DISPLAY: &str = "DISPLAY";`,
//! `rust-rustdesk-rustdesk-complex-change-with-a-lot-of-add`), or the one inlined. Two leaves
//! pair as one match, across kinds where need be (`ASTDiff::is_valid` accepts that from this
//! pass alone); anything else is an all-to-all group of the two subtrees, the way a reader
//! would paint it: every part of the old value became every part of the new one.

use crate::code::ASTMetadata;
use crate::diff::{ASTDiff, ASTMapping, ASTMappingOperation, ASTMappingReason, PassCtx};
use rustc_hash::FxHashMap;

/// `id` and its descendants in document order.
fn subtree(meta: &ASTMetadata, id: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut stack = vec![id];
    while let Some(id) = stack.pop() {
        out.push(id);
        if let Some(info) = meta.node_info.get(&id) {
            stack.extend(info.children.iter().rev().copied());
        }
    }
    out
}

fn unpaired(map: &FxHashMap<usize, usize>, id: usize) -> bool {
    map.get(&id).is_none_or(|&partner| partner == 0)
}

/// A leaf, or a literal whose children are all leaves (`"&#xa0;"`: two quotes and a fragment);
/// never a comment.
fn is_atom(meta: &ASTMetadata, id: usize) -> bool {
    let Some(info) = meta.node_info.get(&id) else {
        return false;
    };
    if crate::diff::nodes::is_comment(&info.kind) {
        return false;
    }
    info.children.is_empty()
        || (info.kind.contains("literal")
            && info.children.iter().all(|child| {
                meta.node_info
                    .get(child)
                    .is_some_and(|c| c.children.is_empty())
            }))
}

/// An atom's text, its leaves' texts joined: what it reads as, wherever it sits.
fn atom_text(meta: &ASTMetadata, id: usize) -> String {
    subtree(meta, id)
        .iter()
        .filter_map(|id| meta.node_info.get(id))
        .filter(|info| info.children.is_empty())
        .map(|info| info.text.as_str())
        .collect::<Vec<_>>()
        .join("\u{1}")
}

/// How many wholly unpaired atoms read as each text, on one side. A leaf, most of them, is
/// counted under its own text, borrowed: [`atom_text`] of a leaf is that text.
fn unpaired_atom_texts<'m>(
    meta: &'m ASTMetadata,
    map: &FxHashMap<usize, usize>,
) -> FxHashMap<std::borrow::Cow<'m, str>, usize> {
    let mut counts: FxHashMap<std::borrow::Cow<'m, str>, usize> = FxHashMap::default();
    for (&id, info) in &meta.node_info {
        // `subtree` starts with `id`, so a paired node fails here before its subtree is listed.
        if !unpaired(map, id) || !is_atom(meta, id) {
            continue;
        }
        let text = if info.children.is_empty() {
            std::borrow::Cow::Borrowed(info.text.as_str())
        } else if subtree(meta, id).iter().all(|&id| unpaired(map, id)) {
            std::borrow::Cow::Owned(atom_text(meta, id))
        } else {
            continue;
        };
        *counts.entry(text).or_default() += 1;
    }
    counts
}

/// Whether `id`, an identifier, has another unpaired copy on its side beside a value reading
/// `value`: the name declared for the value that moved (`KEY = "DISPLAY"`), not a replacement for
/// it.
fn named_after(meta: &ASTMetadata, id: usize, value: &str, map: &FxHashMap<usize, usize>) -> bool {
    let Some(info) = meta.node_info.get(&id) else {
        return false;
    };
    if !crate::diff::nodes::is_identifier_kind(&info.kind) {
        return false;
    }
    meta.node_info.iter().any(|(&other, other_info)| {
        other != id
            && other_info.children.is_empty()
            && other_info.kind == info.kind
            && other_info.text == info.text
            && unpaired(map, other)
            && meta
                .node_to_parent
                .get(&other)
                .and_then(|parent| meta.node_info.get(parent))
                .is_some_and(|parent| {
                    parent
                        .children
                        .iter()
                        .any(|&sibling| sibling != other && atom_text(meta, sibling) == value)
                })
    })
}

/// The one wholly unpaired named child of `parent`, by its index among the named children, and
/// how many named children there are.
fn sole_unpaired_child(
    meta: &ASTMetadata,
    parent: usize,
    map: &FxHashMap<usize, usize>,
) -> Option<(usize, usize, usize)> {
    let named: Vec<usize> = meta
        .node_info
        .get(&parent)?
        .children
        .iter()
        .copied()
        .filter(|child| meta.node_info.get(child).is_some_and(|c| c.is_named))
        .collect();
    let mut found = None;
    for (index, &child) in named.iter().enumerate() {
        if subtree(meta, child).iter().all(|&id| unpaired(map, id)) {
            if found.is_some() {
                return None;
            }
            found = Some((index, child));
        } else if unpaired(map, child) {
            // Half-paired: whatever replaced it is not a plain swap.
            return None;
        }
    }
    found.map(|(index, child)| (index, child, named.len()))
}

/// A construct whose operands a reader reads as values compared or passed: an argument list, a
/// type in a template's arguments, or a comparison. Not arithmetic or concatenation, where the
/// corpus reads a replaced operand as removed and added (`'"'` becoming `"'"` in
/// `java-genymobile-scrcpy-char-to-string-bugfix`).
fn is_value_slot(meta: &ASTMetadata, id: usize) -> bool {
    let Some(info) = meta.node_info.get(&id) else {
        return false;
    };
    match info.kind.as_str() {
        "type_descriptor" => true,
        "binary_expression" => info.children.iter().any(|child| {
            meta.node_info.get(child).is_some_and(|c| {
                !c.is_named && matches!(c.text.as_str(), "<" | ">" | "<=" | ">=" | "==" | "!=")
            })
        }),
        kind => kind.contains("argument"),
    }
}

/// A built-in type that wraps its keyword: Java's `integral_type`, `floating_point_type` and
/// `boolean_type`.
fn is_builtin_type_wrapper(meta: &ASTMetadata, id: usize) -> bool {
    meta.node_info.get(&id).is_some_and(|info| {
        matches!(
            info.kind.as_str(),
            "integral_type" | "floating_point_type" | "boolean_type"
        ) && info.children.len() == 1
    })
}

/// The name `parent` declares: its own `identifier` child, or its declarator's
/// (`int n = 1;`).
fn declared_name(meta: &ASTMetadata, parent: usize) -> Option<usize> {
    let children = &meta.node_info.get(&parent)?.children;
    let first_identifier = |ids: &[usize]| {
        ids.iter().copied().find(|id| {
            meta.node_info
                .get(id)
                .is_some_and(|i| i.kind == "identifier")
        })
    };
    first_identifier(children).or_else(|| {
        let declarator = children.iter().copied().find(|id| {
            meta.node_info
                .get(id)
                .is_some_and(|i| i.kind.ends_with("declarator"))
        })?;
        first_identifier(&meta.node_info.get(&declarator)?.children)
    })
}

/// Pairs the keywords of a built-in type the tree edit distance paired with another, which it
/// cannot relabel because each keyword is a kind of its own (`float` becoming `int`). Only when
/// the declaration the type belongs to keeps its name: a declaration replaced by another
/// (`long id = ..` by `byte[] idBuffer = ..`, `java-genymobile-scrcpy-add-delete`) reads as
/// removed and added, types and all.
fn pair_builtin_type_keywords(before: &ASTMetadata, after: &ASTMetadata, diff: &mut ASTDiff) {
    let keeps_name = |b: usize, a: usize, diff: &ASTDiff| {
        let (Some(&before_parent), Some(&after_parent)) =
            (before.node_to_parent.get(&b), after.node_to_parent.get(&a))
        else {
            return false;
        };
        match (
            declared_name(before, before_parent),
            declared_name(after, after_parent),
        ) {
            (Some(before_name), Some(after_name)) => {
                diff.before_node_map.get(&before_name) == Some(&after_name)
                    && before.node_info[&before_name].text == after.node_info[&after_name].text
            }
            _ => false,
        }
    };
    let mut pairs: Vec<(usize, usize)> = diff
        .before_node_map
        .iter()
        .filter(|&(&b, &a)| {
            b != 0
                && a != 0
                && is_builtin_type_wrapper(before, b)
                && is_builtin_type_wrapper(after, a)
                && keeps_name(b, a, diff)
        })
        .map(|(&b, &a)| {
            (
                before.node_info[&b].children[0],
                after.node_info[&a].children[0],
            )
        })
        .filter(|&(b, a)| unpaired(&diff.before_node_map, b) && unpaired(&diff.after_node_map, a))
        .collect();
    // Sorted so `mapping`'s history is reproducible run to run.
    pairs.sort_unstable();
    for (b, a) in pairs {
        diff.remove_delete_mapping(b);
        diff.remove_insert_mapping(a);
        diff.add_mapping(
            b,
            a,
            ASTMapping {
                cost: 0,
                operation: ASTMappingOperation::MatchButNotIdentical,
                reason: ASTMappingReason::ReplacedAtom,
            },
        );
    }
}

pub fn solve(ctx: &PassCtx, diff: &mut ASTDiff) {
    let (before, after) = (ctx.before_metadata(), ctx.after_metadata());
    let mut parents: Vec<(usize, usize)> = diff
        .before_node_map
        .iter()
        .filter(|&(&b, &a)| b != 0 && a != 0)
        .map(|(&b, &a)| (b, a))
        .collect();
    // Sorted so the pairing order, and so `mapping`'s history, is reproducible run to run.
    parents.sort_unstable();
    pair_builtin_type_keywords(before, after, diff);
    let before_texts = unpaired_atom_texts(before, &diff.before_node_map);
    let after_texts = unpaired_atom_texts(after, &diff.after_node_map);
    for (before_parent, after_parent) in parents {
        if !is_value_slot(before, before_parent) || !is_value_slot(after, after_parent) {
            continue;
        }
        let Some((before_index, before_child, before_count)) =
            sole_unpaired_child(before, before_parent, &diff.before_node_map)
        else {
            continue;
        };
        let Some((after_index, after_child, after_count)) =
            sole_unpaired_child(after, after_parent, &diff.after_node_map)
        else {
            continue;
        };
        if before_index != after_index
            || before_count != after_count
            || !is_atom(before, before_child)
            || !is_atom(after, after_child)
        {
            continue;
        }
        // A value that still has an unpaired copy on the other side was moved, not replaced: a
        // literal extracted into a constant (`"DISPLAY"` into `ENV_KEY_DISPLAY`), or two argument
        // lists the tree edit distance paired one call apart.
        let (before_text, after_text) = (
            atom_text(before, before_child),
            atom_text(after, after_child),
        );
        let same = usize::from(before_text == after_text);
        if after_texts.get(before_text.as_str()).copied().unwrap_or(0) > same
            || before_texts.get(after_text.as_str()).copied().unwrap_or(0) > same
        {
            continue;
        }
        if named_after(before, before_child, &after_text, &diff.before_node_map)
            || named_after(after, after_child, &before_text, &diff.after_node_map)
        {
            continue;
        }
        let before_members = subtree(before, before_child);
        let after_members = subtree(after, after_child);
        for &id in &before_members {
            diff.remove_delete_mapping(id);
        }
        for &id in &after_members {
            diff.remove_insert_mapping(id);
        }
        let mapping = ASTMapping {
            cost: 0,
            operation: ASTMappingOperation::MatchButNotIdentical,
            reason: ASTMappingReason::ReplacedAtom,
        };
        if before_members.len() == 1 && after_members.len() == 1 {
            diff.add_mapping(before_child, after_child, mapping);
        } else {
            diff.add_group(&before_members, &after_members, mapping);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::code::{Code, Language};
    use crate::diff::{ASTMappingReason, diff_code};

    fn replaced(before_src: &str, after_src: &str, language: Language) -> usize {
        let before = Code::from_string(before_src, &language);
        let after = Code::from_string(after_src, &language);
        let diff = diff_code(&before, &after).ast.expect("ast diff");
        diff.mapping
            .values()
            .filter(|m| m.reason == ASTMappingReason::ReplacedAtom)
            .count()
    }

    #[test]
    fn an_argument_replaced_by_a_string_is_one_group() {
        assert!(
            replaced(
                "class C { void f() { accum.append(c); } }",
                "class C { void f() { accum.append(\"&#xa0;\"); } }",
                Language::Java,
            ) > 0
        );
    }

    #[test]
    fn an_identifier_replaced_by_a_number_is_paired() {
        assert_eq!(
            replaced(
                "int f() { return g(a, FLAG); }",
                "int f() { return g(a, 0); }",
                Language::C,
            ),
            1
        );
    }

    #[test]
    fn a_built_in_return_type_replaced_pairs_its_keyword() {
        assert_eq!(
            replaced(
                "class C { static float f(String v) { return 1; } }",
                "class C { static int f(String v) { return 1; } }",
                Language::Java,
            ),
            1
        );
    }

    #[test]
    fn an_assigned_value_replaced_is_left_alone() {
        assert_eq!(
            replaced(
                "class C { void f() { argName = \"arg\"; } }",
                "class C { void f() { argName = null; } }",
                Language::Java,
            ),
            0
        );
    }

    #[test]
    fn a_literal_extracted_into_a_constant_is_left_alone() {
        assert_eq!(
            replaced(
                "fn f() { g(\"DISPLAY\"); }",
                "const KEY: &str = \"DISPLAY\";\nfn f() { g(KEY); }",
                Language::Rust,
            ),
            0
        );
    }

    #[test]
    fn a_call_replacing_an_identifier_is_not_an_atom() {
        assert_eq!(
            replaced(
                "int f() { return g(a, FLAG); }",
                "int f() { return g(a, h(1)); }",
                Language::C,
            ),
            0
        );
    }

    #[test]
    fn a_literal_moved_into_a_new_constant_is_left_alone() {
        assert_eq!(
            replaced(
                "fn f() { g(\"DISPLAY\"); }\nfn k() { h(\"DISPLAY\"); }",
                "const KEY: &str = \"DISPLAY\";\nfn f() { g(KEY); }\nfn k() { h(\"X\"); }",
                Language::Rust,
            ),
            0
        );
    }
}
