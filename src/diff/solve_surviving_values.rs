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
//! Phase 9b, after `solve_replaced_atoms`: a value that survives a rewrite of the expression around
//! it keeps its identity. `int32(24)` becoming `"24"` (`go-kubernetes-kubernetes-int-to-string`)
//! leaves the `24` in place as the reader sees it - the conversion became a string, the number
//! stayed - but the old leaf is an `int_literal` under a call and the new one string content
//! under a literal, so no pass pairs them.
//!
//! A deleted named leaf is paired with an inserted one when both read the same and sit in one
//! slot: the nearest paired ancestor of each is the other's nearest paired ancestor's partner,
//! and inside those two subtrees each is the only unpaired leaf with that text. The slot must lie
//! strictly inside the leaf's statement (`solve_renamed_twins::enclosing_statement`) on both
//! sides: a statement or a body rewritten around a name that recurs in it is new code that
//! happens to reuse the name, and the reader sees it removed and added
//! (`java-defects4j-jxpath-7-*`, a method body replaced). As in `solve_renamed_twins`, a leaf that
//! is an operand of a binary operator on one side only is two values that read alike: the `0` of
//! `n > 0` and of `Math.Max(0, ...)` (`csharp-jellyfin-add-function`). Uniqueness is asked of the slot only:
//! asking it of the whole file loses most survivors (a table of small numbers repeats them), at
//! the price of a value whose reader saw it move to another copy elsewhere
//! (`csharp-jellyfin-jellyfin-add-and-move`). Across kinds the
//! pair is `MatchButNotIdentical` (accepted by `ASTDiff::is_valid` for this reason).

use crate::code::ASTMetadata;
use crate::diff::solve_renamed_twins::{enclosing_statement, is_operand};
use crate::diff::{ASTDiff, ASTMapping, ASTMappingReason, PassCtx};
use rustc_hash::FxHashMap;

fn unpaired(map: &FxHashMap<usize, usize>, id: usize) -> bool {
    map.get(&id).is_none_or(|&partner| partner == 0)
}

/// The nearest ancestor of `id` that is paired, with its partner.
fn paired_ancestor(
    meta: &ASTMetadata,
    map: &FxHashMap<usize, usize>,
    id: usize,
) -> Option<(usize, usize)> {
    let mut current = *meta.node_to_parent.get(&id)?;
    loop {
        if let Some(&partner) = map.get(&current)
            && partner != 0
        {
            return Some((current, partner));
        }
        current = *meta.node_to_parent.get(&current)?;
    }
}

/// The unpaired named leaves of `root`'s subtree, by text.
fn unpaired_leaves_by_text<'m>(
    meta: &'m ASTMetadata,
    map: &FxHashMap<usize, usize>,
    root: usize,
) -> FxHashMap<&'m str, Vec<usize>> {
    let mut out: FxHashMap<&str, Vec<usize>> = FxHashMap::default();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        let Some(info) = meta.node_info.get(&id) else {
            continue;
        };
        if info.children.is_empty() {
            if info.is_named
                && !info.text.is_empty()
                && !crate::diff::nodes::is_comment(&info.kind)
                && unpaired(map, id)
            {
                out.entry(info.text.as_str()).or_default().push(id);
            }
        } else {
            stack.extend(info.children.iter().copied());
        }
    }
    out
}

/// Whether `ancestor` lies strictly inside the statement that encloses `leaf`: the slot is a part
/// of one expression, not a statement or a body rewritten around a leaf that happens to recur.
fn inside_statement(meta: &ASTMetadata, leaf: usize, ancestor: usize) -> bool {
    let Some(statement) = enclosing_statement(meta, leaf) else {
        return false;
    };
    if ancestor == statement {
        return false;
    }
    let mut current = ancestor;
    while let Some(&parent) = meta.node_to_parent.get(&current) {
        if parent == statement {
            return true;
        }
        current = parent;
    }
    false
}

pub fn solve(ctx: &PassCtx, diff: &mut ASTDiff) {
    let (before, after) = (ctx.before_metadata(), ctx.after_metadata());
    // Candidate before leaves, in preorder for determinism.
    let mut leaves: Vec<(usize, usize)> = before
        .node_info
        .iter()
        .filter(|&(&id, info)| {
            info.children.is_empty()
                && info.is_named
                && !crate::diff::nodes::is_comment(&info.kind)
                && unpaired(&diff.before_node_map, id)
        })
        .map(|(&id, info)| (info.preorder_index, id))
        .collect();
    leaves.sort_unstable();

    // Per paired ancestor pair, the unpaired leaves of both subtrees, computed once.
    type Slot<'m> = (
        FxHashMap<&'m str, Vec<usize>>,
        FxHashMap<&'m str, Vec<usize>>,
    );
    let mut slots: FxHashMap<(usize, usize), Slot> = FxHashMap::default();
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for (_, leaf) in leaves {
        let Some((ancestor, partner)) = paired_ancestor(before, &diff.before_node_map, leaf) else {
            continue;
        };
        if !inside_statement(before, leaf, ancestor) {
            continue;
        }
        let slot = slots.entry((ancestor, partner)).or_insert_with(|| {
            (
                unpaired_leaves_by_text(before, &diff.before_node_map, ancestor),
                unpaired_leaves_by_text(after, &diff.after_node_map, partner),
            )
        });
        let Some(text) = before.node_info.get(&leaf).map(|info| info.text.as_str()) else {
            continue;
        };
        let (Some(olds), Some(news)) = (slot.0.get(text), slot.1.get(text)) else {
            continue;
        };
        if olds.len() != 1 || news.len() != 1 {
            continue;
        }
        let candidate = news[0];
        // The new leaf's own nearest paired ancestor must be this slot's, not one nested deeper.
        if paired_ancestor(after, &diff.after_node_map, candidate) != Some((partner, ancestor)) {
            continue;
        }
        if !inside_statement(after, candidate, partner)
            || is_operand(before, leaf) != is_operand(after, candidate)
        {
            continue;
        }
        pairs.push((leaf, candidate));
    }

    for (b, a) in pairs {
        let same_kind = before.node_info.get(&b).map(|info| &info.kind)
            == after.node_info.get(&a).map(|info| &info.kind);
        if diff.before_node_map.get(&b) == Some(&0) {
            diff.remove_delete_mapping(b);
        }
        if diff.after_node_map.get(&a) == Some(&0) {
            diff.remove_insert_mapping(a);
        }
        let mapping = if same_kind {
            ASTMapping::identical(ASTMappingReason::SurvivingValue)
        } else {
            ASTMapping::matched_not_identical(ASTMappingReason::SurvivingValue)
        };
        diff.add_mapping(b, a, mapping);
    }
}

#[cfg(test)]
mod tests {
    use crate::code::{Code, Language};
    use crate::diff::diff_code;

    /// The ids of the named leaves reading `text`, in document order.
    fn leaves(code: &Code, text: &str) -> Vec<usize> {
        let mut found = Vec::new();
        let mut stack = vec![code.ast.as_ref().unwrap().root_node()];
        while let Some(node) = stack.pop() {
            if node.child_count() == 0
                && node.is_named()
                && code.contents.get(node.byte_range()) == Some(text)
            {
                found.push((node.start_byte(), node.id()));
            }
            let mut cursor = node.walk();
            stack.extend(node.children(&mut cursor));
        }
        found.sort_unstable();
        found.into_iter().map(|(_, id)| id).collect()
    }

    /// go-kubernetes-kubernetes-int-to-string, minimized: a converted number became a string.
    #[test]
    fn a_number_that_became_a_string_keeps_its_digits() {
        let before = Code::from_string(
            "package p\n\nvar cells = []interface{}{\"a\", int32(24), \"b\"}\n",
            &Language::Go,
        );
        let after = Code::from_string(
            "package p\n\nvar cells = []interface{}{\"a\", \"24\", \"b\"}\n",
            &Language::Go,
        );
        let diff = diff_code(&before, &after).ast.unwrap();
        assert_eq!(
            diff.before_node_map.get(&leaves(&before, "24")[0]),
            Some(&leaves(&after, "24")[0])
        );
    }

    /// A statement rewritten around a name it reuses is new code: when the nearest paired
    /// ancestor is the block above the statements, the slot is not inside one statement.
    #[test]
    fn a_name_reused_by_a_rewritten_statement_is_not_paired_in_place() {
        use crate::diff::{ASTDiff, ASTMapping, ASTMappingReason, NodeCache, PassCtx};
        let before = Code::from_string(
            "class C {\n    void f() {\n        x = compute(a);\n    }\n}\n",
            &Language::Java,
        );
        let after = Code::from_string(
            "class C {\n    void f() {\n        print(a);\n    }\n}\n",
            &Language::Java,
        );
        // Only the chain down to the method body is paired; both statements are undecided.
        let chain = |code: &Code| {
            let mut node = code.ast.as_ref().unwrap().root_node();
            let mut ids = vec![node.id()];
            for kind in [
                "class_declaration",
                "class_body",
                "method_declaration",
                "block",
            ] {
                let mut cursor = node.walk();
                node = node
                    .named_children(&mut cursor)
                    .find(|child| child.kind() == kind)
                    .unwrap();
                ids.push(node.id());
            }
            ids
        };
        let node_cache = NodeCache::build(&before, &after);
        let mut diff = ASTDiff::default();
        for (b, a) in chain(&before).into_iter().zip(chain(&after)) {
            diff.add_mapping(
                b,
                a,
                ASTMapping::matched_not_identical(ASTMappingReason::MutualAncestor),
            );
        }
        super::solve(&PassCtx::new(&before, &after, &node_cache), &mut diff);
        assert_eq!(diff.before_node_map.get(&leaves(&before, "a")[0]), None);
    }
}
