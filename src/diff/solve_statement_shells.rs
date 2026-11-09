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
//! Phase 9b, after the leaf re-pairings: keeps a statement whose expression moved out of it. The
//! tree edit distance keeps ancestry, so when a statement's value moves into a new statement and
//! a new value takes its place, it can keep the value or the statement around it, not both:
//! `return base64.readResults(b, offset, len);` becoming `readLen = base64.readResults(...);` and
//! `return readLen;` (`java-defects4j-codec-6-base64inputstream`) keeps the call and deletes the
//! `return` and its `;`, inserting new ones a line below. A reader sees the `return` stay and its
//! value change; the call moved.
//!
//! A removed statement and a removed statement of the same kind are paired when they are the only
//! removed child of that kind under their parents, which are paired with each other; neither holds
//! a body (a block, or a nested statement or clause); and one of them holds content paired outside
//! the other - the moved value, the evidence that the edit distance gave the statement up to keep
//! it. Two removed statements with nothing moved between them are left alone: they were replaced,
//! and the edit distance already weighed pairing them.
//!
//! A statement with a keyword of its own (`return`, `assert`) must also be the only removed node
//! of its kind anywhere below its parent on each side: its value may have taken it along into a
//! new construct, a `return` into the new `if` that guards it
//! (`java-defects4j-cli-37-defaultparser`), and with two candidates the pass does not guess. A
//! statement with no keyword (an expression or an assignment) is its value: when the value went
//! into a new body-less statement beside the candidate, that one is the statement, changed kind
//! (`x.thenInject();` becoming `Object injected = x.thenInject();` beside a new
//! `mocks.remove(injected);`, `java-defects4j-mockito-28-defaultinjectionengine`).
//!
//! Inside a paired statement, a removed leaf is paired with a removed leaf of the same kind and
//! text when each side has as many of them, in order (`return` and `;`), and a removed inner
//! child with the only removed child of its kind on the other side, the same way down (`a == b`
//! in `assert a == b` keeping its `==`).

use crate::code::ASTMetadata;
use crate::diff::solve_renamed_twins::has_body;
use crate::diff::{ASTDiff, ASTMapping, ASTMappingReason, PassCtx};
use rustc_hash::FxHashMap;

/// No partner: deleted, inserted, or still undecided.
fn removed(map: &FxHashMap<usize, usize>, id: usize) -> bool {
    map.get(&id).is_none_or(|&partner| partner == 0)
}

/// A statement that can lose its value to a move: one with its own syntax (a keyword, a `;`) and
/// no body. A block is a sequence of statements, not one.
fn is_shell_statement(meta: &ASTMetadata, id: usize) -> bool {
    let Some(info) = meta.node_info.get(&id) else {
        return false;
    };
    let kind = info.kind.as_str();
    !info.children.is_empty()
        && kind.contains("statement")
        && !kind.contains("block")
        && !kind.contains("compound")
        && !crate::diff::nodes::is_statement_sequence_body(kind)
        && !has_body(meta, id)
}

/// Whether `id` is `root` or inside it.
fn within(meta: &ASTMetadata, mut id: usize, root: usize) -> bool {
    loop {
        if id == root {
            return true;
        }
        match meta.node_to_parent.get(&id) {
            Some(&parent) => id = parent,
            None => return false,
        }
    }
}

/// Whether `statement` is the only removed node of its kind anywhere below `parent`.
fn sole_removed_of_kind(
    meta: &ASTMetadata,
    parent: usize,
    statement: usize,
    map: &FxHashMap<usize, usize>,
) -> bool {
    let kind = &meta.node_info[&statement].kind;
    let mut stack = vec![parent];
    while let Some(id) = stack.pop() {
        let Some(info) = meta.node_info.get(&id) else {
            continue;
        };
        if id != statement && id != parent && info.kind == *kind && removed(map, id) {
            return false;
        }
        stack.extend(info.children.iter().copied());
    }
    true
}

/// Whether `statement` has a keyword of its own (`return`, `assert`), a word-like anonymous leaf.
fn has_keyword(meta: &ASTMetadata, statement: usize) -> bool {
    meta.node_info[&statement].children.iter().any(|child| {
        meta.node_info.get(child).is_some_and(|info| {
            !info.is_named
                && info.children.is_empty()
                && !info.text.is_empty()
                && info.text.chars().all(|c| c.is_alphabetic() || c == '_')
        })
    })
}

/// Whether content of `statement` went into a removed statement without a body directly under
/// `after_parent`, other than `after_statement`: the statement itself, changed kind in place,
/// which leaves nothing for a statement of its old kind beside it to take over.
fn replaced_in_place(
    before: &ASTMetadata,
    after: &ASTMetadata,
    statement: usize,
    after_statement: usize,
    after_parent: usize,
    diff: &ASTDiff,
) -> bool {
    let map = &diff.before_node_map;
    let mut stack = before.node_info[&statement].children.clone();
    while let Some(id) = stack.pop() {
        if let Some(&partner) = map.get(&id)
            && partner != 0
        {
            let mut holder = partner;
            while let Some(&parent) = after.node_to_parent.get(&holder) {
                if parent == after_parent {
                    break;
                }
                holder = parent;
            }
            if after.node_to_parent.get(&holder) == Some(&after_parent)
                && holder != after_statement
                && removed(&diff.after_node_map, holder)
                && !has_body(after, holder)
            {
                return true;
            }
            continue;
        }
        if let Some(info) = before.node_info.get(&id) {
            stack.extend(info.children.iter().copied());
        }
    }
    false
}

/// Whether a node below `root` is paired with a node outside `other_root`.
fn holds_moved_content(
    this: &ASTMetadata,
    other: &ASTMetadata,
    root: usize,
    other_root: usize,
    map: &FxHashMap<usize, usize>,
) -> bool {
    let mut stack: Vec<usize> = this
        .node_info
        .get(&root)
        .map(|info| info.children.clone())
        .unwrap_or_default();
    while let Some(id) = stack.pop() {
        if let Some(&partner) = map.get(&id)
            && partner != 0
            && !within(other, partner, other_root)
        {
            return true;
        }
        if let Some(info) = this.node_info.get(&id) {
            stack.extend(info.children.iter().copied());
        }
    }
    false
}

/// `parent`'s removed children, leaves by kind and text and inner nodes by kind, each list in
/// document order.
type RemovedChildren<'m> = (
    FxHashMap<(&'m str, &'m str), Vec<usize>>,
    FxHashMap<&'m str, Vec<usize>>,
);

fn removed_children<'m>(
    meta: &'m ASTMetadata,
    parent: usize,
    map: &FxHashMap<usize, usize>,
) -> RemovedChildren<'m> {
    let mut leaves: FxHashMap<(&str, &str), Vec<usize>> = FxHashMap::default();
    let mut inner: FxHashMap<&str, Vec<usize>> = FxHashMap::default();
    let Some(info) = meta.node_info.get(&parent) else {
        return (leaves, inner);
    };
    for &child in &info.children {
        if !removed(map, child) {
            continue;
        }
        let Some(child_info) = meta.node_info.get(&child) else {
            continue;
        };
        if child_info.children.is_empty() {
            leaves
                .entry((child_info.kind.as_str(), child_info.text.as_str()))
                .or_default()
                .push(child);
        } else {
            inner
                .entry(child_info.kind.as_str())
                .or_default()
                .push(child);
        }
    }
    (leaves, inner)
}

/// `parent`'s removed children by kind.
fn removed_by_kind<'m>(
    meta: &'m ASTMetadata,
    parent: usize,
    map: &FxHashMap<usize, usize>,
) -> FxHashMap<&'m str, Vec<usize>> {
    let mut by_kind: FxHashMap<&str, Vec<usize>> = FxHashMap::default();
    let children = meta
        .node_info
        .get(&parent)
        .map(|info| info.children.as_slice())
        .unwrap_or_default();
    for &child in children {
        if removed(map, child)
            && let Some(info) = meta.node_info.get(&child)
        {
            by_kind.entry(info.kind.as_str()).or_default().push(child);
        }
    }
    by_kind
}

fn pair(diff: &mut ASTDiff, before_id: usize, after_id: usize, mapping: ASTMapping) {
    diff.remove_delete_mapping(before_id);
    diff.remove_insert_mapping(after_id);
    diff.add_mapping(before_id, after_id, mapping);
}

/// Pairs `before_id` with `after_id`, then their removed children as the module doc says.
fn pair_shell(
    before: &ASTMetadata,
    after: &ASTMetadata,
    before_id: usize,
    after_id: usize,
    diff: &mut ASTDiff,
) {
    pair(
        diff,
        before_id,
        after_id,
        ASTMapping::matched_not_identical(ASTMappingReason::StatementShell),
    );
    let (before_leaves, before_inner) = removed_children(before, before_id, &diff.before_node_map);
    let (after_leaves, after_inner) = removed_children(after, after_id, &diff.after_node_map);

    let mut leaf_pairs: Vec<(usize, usize)> = Vec::new();
    for (key, before_ids) in &before_leaves {
        if let Some(after_ids) = after_leaves.get(key)
            && after_ids.len() == before_ids.len()
        {
            leaf_pairs.extend(before_ids.iter().copied().zip(after_ids.iter().copied()));
        }
    }
    let mut inner_pairs: Vec<(usize, usize)> = Vec::new();
    for (kind, before_ids) in &before_inner {
        if let ([before_child], Some([after_child])) = (
            before_ids.as_slice(),
            after_inner.get(kind).map(Vec::as_slice),
        ) {
            inner_pairs.push((*before_child, *after_child));
        }
    }
    // Sorted so `mapping`'s history is reproducible run to run.
    leaf_pairs.sort_unstable_by_key(|&(b, _)| before.node_info[&b].preorder_index);
    inner_pairs.sort_unstable_by_key(|&(b, _)| before.node_info[&b].preorder_index);
    for (b, a) in leaf_pairs {
        pair(
            diff,
            b,
            a,
            ASTMapping::identical(ASTMappingReason::StatementShell),
        );
    }
    for (b, a) in inner_pairs {
        pair_shell(before, after, b, a, diff);
    }
}

pub fn solve(ctx: &PassCtx, diff: &mut ASTDiff) {
    let (before, after) = (ctx.before_metadata(), ctx.after_metadata());

    // Parent pairs with a removed shell statement below the before side, in preorder.
    let mut parents: Vec<(usize, usize)> = diff
        .before_node_map
        .iter()
        .filter(|&(_, &after_id)| after_id != 0)
        .map(|(&before_id, &after_id)| (before_id, after_id))
        .filter(|&(before_id, _)| {
            before.node_info.get(&before_id).is_some_and(|info| {
                info.children.iter().any(|&child| {
                    removed(&diff.before_node_map, child) && is_shell_statement(before, child)
                })
            })
        })
        .collect();
    parents.sort_unstable_by_key(|&(before_id, _)| before.node_info[&before_id].preorder_index);

    for (before_parent, after_parent) in parents {
        let before_shells = removed_by_kind(before, before_parent, &diff.before_node_map);
        let after_shells = removed_by_kind(after, after_parent, &diff.after_node_map);
        let mut candidates: Vec<(usize, usize)> = before_shells
            .iter()
            .filter_map(|(kind, before_ids)| {
                match (
                    before_ids.as_slice(),
                    after_shells.get(kind).map(Vec::as_slice),
                ) {
                    ([b], Some([a])) => Some((*b, *a)),
                    _ => None,
                }
            })
            .filter(|&(b, a)| is_shell_statement(before, b) && is_shell_statement(after, a))
            .collect();
        candidates.sort_unstable_by_key(|&(b, _)| before.node_info[&b].preorder_index);
        for (b, a) in candidates {
            let moved = holds_moved_content(before, after, b, a, &diff.before_node_map)
                || holds_moved_content(after, before, a, b, &diff.after_node_map);
            let fits = if has_keyword(before, b) {
                sole_removed_of_kind(before, before_parent, b, &diff.before_node_map)
                    && sole_removed_of_kind(after, after_parent, a, &diff.after_node_map)
            } else {
                !replaced_in_place(before, after, b, a, after_parent, diff)
            };
            if moved && fits {
                pair_shell(before, after, b, a, diff);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::code::{Code, Language};
    use crate::diff::diff_code;

    fn all_nodes(root: tree_sitter::Node) -> Vec<tree_sitter::Node> {
        let mut all = vec![];
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            all.push(node);
            let mut cursor = node.walk();
            stack.extend(node.children(&mut cursor));
        }
        all
    }

    /// The after line of the partner of the first before leaf reading `text` on before line
    /// `row` (0-based), if it is paired.
    fn partner_row(before: &Code, after: &Code, text: &str, row: usize) -> Option<usize> {
        let diff = diff_code(before, after).ast.expect("ast diff");
        let leaf = all_nodes(before.ast.as_ref().unwrap().root_node())
            .into_iter()
            .filter(|n| {
                n.child_count() == 0
                    && n.start_position().row == row
                    && before.contents.get(n.byte_range()) == Some(text)
            })
            .min_by_key(|n| n.start_byte())?;
        let partner = *diff.before_node_map.get(&leaf.id())?;
        all_nodes(after.ast.as_ref().unwrap().root_node())
            .into_iter()
            .find(|n| n.id() == partner)
            .map(|n| n.start_position().row)
    }

    #[test]
    fn a_return_whose_value_moved_out_keeps_its_keyword() {
        let before_src = "class C {\n  int f() {\n    a();\n    return g(b, c);\n  }\n}\n";
        let after_src = "class C {\n  int f() {\n    a();\n    int r = g(b, c);\n    \
                         return r;\n  }\n}\n";
        let before = Code::from_string(before_src, &Language::Java);
        let after = Code::from_string(after_src, &Language::Java);
        assert_eq!(partner_row(&before, &after, "return", 3), Some(4));
        assert_eq!(partner_row(&before, &after, ";", 3), Some(4));
    }

    #[test]
    fn a_return_with_a_second_candidate_inside_a_new_if_is_left_alone() {
        let before_src =
            "class C {\n  boolean f() {\n    a();\n    return p(t) && q(t, 1, 2);\n  }\n}\n";
        let after_src = "class C {\n  boolean f() {\n    a();\n    if (!p(t)) { return false; }\n    \
                         int n = 3;\n    return r(n);\n  }\n}\n";
        let before = Code::from_string(before_src, &Language::Java);
        let after = Code::from_string(after_src, &Language::Java);
        let diff = diff_code(&before, &after).ast.expect("ast diff");
        assert!(
            !diff
                .mapping
                .values()
                .any(|m| m.reason == crate::diff::ASTMappingReason::StatementShell)
        );
    }
}
