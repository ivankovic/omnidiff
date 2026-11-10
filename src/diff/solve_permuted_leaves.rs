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
//! Phase 9b: leaves that only changed places among their siblings keep their text. When a
//! construct's shape survives (`StructurallyIdenticalSubtrees`, or the tree edit distance on two
//! same-shaped runs), its leaves are paired by position, so three comment lines rotated
//! (`ruby-mastodon-mastodon-move`: the `account_id` line moved from the top of a schema comment to
//! the bottom) come out as three renames, each line "changed" into its neighbour. A reader sees
//! one line moved and two untouched.
//!
//! Under one pair of paired parents, the same-kind leaves the mapping renames (`Update`) are
//! re-paired by text: a leaf whose text occurs exactly once among the renamed leaves on each side
//! is paired with that one, `Identical`; the leaves left over keep their order and are paired
//! among themselves (renamed, unless a pair happens to read the same). Each re-pairing replaces two renames by a match and at most one rename, so
//! the mapping only gets cheaper.

use crate::diff::{ASTDiff, ASTMapping, ASTMappingOperation, ASTMappingReason, PassCtx};

/// The before parent, the after parent, and the leaves' kind.
type GroupKey<'m> = (usize, usize, &'m str);

pub fn solve(ctx: &PassCtx, diff: &mut ASTDiff) {
    let (before, after) = (ctx.before_metadata(), ctx.after_metadata());
    let order = |meta: &crate::code::ASTMetadata, id: usize| {
        meta.node_info
            .get(&id)
            .map(|info| info.preorder_index)
            .unwrap_or(usize::MAX)
    };

    // Renamed leaf pairs, grouped by their parents' pair and their kind.
    let mut groups: rustc_hash::FxHashMap<GroupKey, Vec<(usize, usize)>> =
        rustc_hash::FxHashMap::default();
    for (&(b, a), mapping) in &diff.mapping {
        if b == 0 || a == 0 || mapping.operation != ASTMappingOperation::Update {
            continue;
        }
        let (Some(before_info), Some(after_info)) =
            (before.node_info.get(&b), after.node_info.get(&a))
        else {
            continue;
        };
        if !before_info.children.is_empty()
            || !after_info.children.is_empty()
            || before_info.kind != after_info.kind
        {
            continue;
        }
        let (Some(&before_parent), Some(&after_parent)) =
            (before.node_to_parent.get(&b), after.node_to_parent.get(&a))
        else {
            continue;
        };
        if diff.before_node_map.get(&before_parent) != Some(&after_parent) {
            continue;
        }
        groups
            .entry((before_parent, after_parent, before_info.kind.as_str()))
            .or_default()
            .push((b, a));
    }

    let mut groups: Vec<Vec<(usize, usize)>> = groups
        .into_values()
        .filter(|pairs| pairs.len() >= 2)
        .collect();
    // Deterministic order of work, whatever the map's iteration order.
    for pairs in &mut groups {
        pairs.sort_unstable_by_key(|&(b, _)| order(before, b));
    }
    groups.sort_unstable_by_key(|pairs| order(before, pairs[0].0));

    let text = |meta: &crate::code::ASTMetadata, id: usize| {
        meta.node_info
            .get(&id)
            .map(|info| info.text.clone())
            .unwrap_or_default()
    };
    for pairs in groups {
        let mut befores: Vec<usize> = pairs.iter().map(|&(b, _)| b).collect();
        let mut afters: Vec<usize> = pairs.iter().map(|&(_, a)| a).collect();
        befores.sort_unstable_by_key(|&id| order(before, id));
        afters.sort_unstable_by_key(|&id| order(after, id));
        let before_texts: Vec<String> = befores.iter().map(|&id| text(before, id)).collect();
        let after_texts: Vec<String> = afters.iter().map(|&id| text(after, id)).collect();

        let mut matched: Vec<(usize, usize)> = Vec::new();
        for (index, text) in before_texts.iter().enumerate() {
            let on_before = before_texts.iter().filter(|other| *other == text).count();
            let on_after: Vec<usize> = after_texts
                .iter()
                .enumerate()
                .filter(|(_, other)| *other == text)
                .map(|(position, _)| position)
                .collect();
            if on_before == 1 && on_after.len() == 1 {
                matched.push((index, on_after[0]));
            }
        }
        if matched.is_empty() {
            continue;
        }

        for &(b, a) in &pairs {
            diff.remove_match_mapping(b, a);
        }
        for &(b, a) in &matched {
            diff.add_mapping(
                befores[b],
                afters[a],
                ASTMapping::identical(ASTMappingReason::PermutedLeaf),
            );
        }
        let rest_before = (0..befores.len()).filter(|index| !matched.iter().any(|m| m.0 == *index));
        let rest_after: Vec<usize> = (0..afters.len())
            .filter(|index| !matched.iter().any(|m| m.1 == *index))
            .collect();
        for (b, a) in rest_before.zip(rest_after) {
            let mapping = if before_texts[b] == after_texts[a] {
                ASTMapping::identical(ASTMappingReason::PermutedLeaf)
            } else {
                ASTMapping::updated(ASTMappingReason::PermutedLeaf)
            };
            diff.add_mapping(befores[b], afters[a], mapping);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::code::{Code, Language};
    use crate::diff::{
        ASTDiff, ASTMapping, ASTMappingOperation, ASTMappingReason, NodeCache, PassCtx,
    };

    /// Every comment, in document order.
    fn comments(code: &Code) -> Vec<usize> {
        let root = code.ast.as_ref().unwrap().root_node();
        let mut cursor = root.walk();
        root.children(&mut cursor)
            .filter(|node| node.kind() == "comment")
            .map(|node| node.id())
            .collect()
    }

    /// Runs only this pass over the roots paired and comments renamed by position, the mapping
    /// a same-shape match leaves.
    fn solve_over_positional_renames(before: &Code, after: &Code) -> ASTDiff {
        let node_cache = NodeCache::build(before, after);
        let mut diff = ASTDiff::default();
        diff.add_mapping(
            before.ast.as_ref().unwrap().root_node().id(),
            after.ast.as_ref().unwrap().root_node().id(),
            ASTMapping::matched_not_identical(ASTMappingReason::StructurallyIdenticalSubtrees),
        );
        for (b, a) in comments(before).into_iter().zip(comments(after)) {
            diff.add_mapping(
                b,
                a,
                ASTMapping::updated(ASTMappingReason::StructurallyIdenticalAncestor),
            );
        }
        super::solve(&PassCtx::new(before, after, &node_cache), &mut diff);
        diff
    }

    /// ruby-mastodon-mastodon-move, minimized: the first line of a comment block moved to its end.
    #[test]
    fn rotated_comment_lines_keep_their_text() {
        let before = Code::from_string("# id\n# rank\n# reason\n", &Language::Ruby);
        let after = Code::from_string("# rank\n# reason\n# id\n", &Language::Ruby);
        let diff = solve_over_positional_renames(&before, &after);
        let (old, new) = (comments(&before), comments(&after));
        assert_eq!(diff.before_node_map.get(&old[0]), Some(&new[2]));
        assert_eq!(diff.before_node_map.get(&old[1]), Some(&new[0]));
        assert_eq!(diff.before_node_map.get(&old[2]), Some(&new[1]));
        assert!(
            diff.mapping
                .values()
                .all(|mapping| mapping.operation != ASTMappingOperation::Update)
        );
    }

    /// Lines whose text occurs on one side only stay renamed among themselves, in order.
    #[test]
    fn leaves_with_no_twin_stay_renamed_in_order() {
        let before = Code::from_string("# a\n# b\n# c\n", &Language::Ruby);
        let after = Code::from_string("# x\n# a\n# y\n", &Language::Ruby);
        let diff = solve_over_positional_renames(&before, &after);
        let (old, new) = (comments(&before), comments(&after));
        assert_eq!(diff.before_node_map.get(&old[0]), Some(&new[1]));
        assert_eq!(diff.before_node_map.get(&old[1]), Some(&new[0]));
        assert_eq!(diff.before_node_map.get(&old[2]), Some(&new[2]));
        assert_eq!(
            diff.mapping[&(old[1], new[0])].operation,
            ASTMappingOperation::Update
        );
    }

    /// A text that occurs twice on a side names no single partner: nothing moves.
    #[test]
    fn a_repeated_text_is_left_to_its_position() {
        let before = Code::from_string("# a\n# a\n# b\n", &Language::Ruby);
        let after = Code::from_string("# b\n# a\n# c\n", &Language::Ruby);
        let diff = solve_over_positional_renames(&before, &after);
        let (old, new) = (comments(&before), comments(&after));
        // `b` is unique on both sides and moves; the two `a`s are not and keep their order.
        assert_eq!(diff.before_node_map.get(&old[2]), Some(&new[0]));
        assert_eq!(diff.before_node_map.get(&old[0]), Some(&new[1]));
        assert_eq!(diff.before_node_map.get(&old[1]), Some(&new[2]));
        assert_eq!(
            diff.mapping[&(old[0], new[1])].operation,
            ASTMappingOperation::Identical,
            "a leftover pair that reads the same is no rename"
        );
    }
}
