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
use std::collections::HashMap;

use crate::code::metadata::metadata_of;
use crate::code::{ASTMetadata, Code};
use crate::diff::apted::SEGMENT_SIMILARITY_MAX_CELLS;
use crate::diff::nodes::is_reference;
use crate::diff::{ASTDiff, ASTMapping, ASTMappingOperation, ASTMappingReason, NodeCache};

/// Which nodes are hash-matching candidates: reference nodes, plus any node meeting both
/// thresholds.
#[derive(Debug, Clone)]
pub struct NodeSelectionConfig {
    pub min_depth: usize,
    pub min_subtree_size: usize,
}

impl Default for NodeSelectionConfig {
    fn default() -> Self {
        Self {
            // Tuned against the benchmark corpus.
            min_depth: 0,
            min_subtree_size: 45,
        }
    }
}

impl NodeSelectionConfig {
    /// [`build_extended_node_list`] as a selector for `solve_with_hash_map`.
    pub fn to_node_list_selector(&self) -> impl Fn(&ASTMetadata) -> Vec<usize> + '_ {
        move |metadata: &ASTMetadata| build_extended_node_list(metadata, self)
    }
}

/// The candidates `config` selects, largest subtree first, ties by start byte.
pub fn build_extended_node_list(
    metadata: &ASTMetadata,
    config: &NodeSelectionConfig,
) -> Vec<usize> {
    let language = metadata.language;
    let mut nodes_with_info: Vec<(usize, usize, usize)> = Vec::new(); // (id, size, start_byte)

    for (&node_id, info) in &metadata.node_info {
        let subtree_size = metadata
            .node_to_subtree_size
            .get(&node_id)
            .copied()
            .unwrap_or(0);
        let depth = metadata.node_to_depth.get(&node_id).copied().unwrap_or(0);
        let start_byte = info.start_byte;

        // A keyword token can share its statement's kind string (Kotlin `import`); unnamed, it
        // would pair with an arbitrary same keyword elsewhere.
        let is_reference_node = info.is_named && is_reference(&info.kind, &language);
        let is_big_enough = depth >= config.min_depth && subtree_size >= config.min_subtree_size;

        if is_reference_node || is_big_enough {
            nodes_with_info.push((node_id, subtree_size, start_byte));
        }
    }

    // Ties by `start_byte`; see `ASTNodeMetadata::start_byte`.
    nodes_with_info.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)));
    nodes_with_info
        .into_iter()
        .map(|(node_id, _, _)| node_id)
        .collect()
}

/// Where the after side sits relative to the before side, learnt from matched roots: a before
/// byte offset (a root's start or end) to the after offset minus it. Shared by the two tiers of
/// phase 1, so the kind-only tier reads the shifts the kind-and-value tier found.
pub(crate) type ShiftAnchors = std::collections::BTreeMap<usize, i64>;

/// The shift at `byte`: that of the nearest anchor at or before it, or 0 above the first one, as
/// the two files start together. An anchor above, because only an edit above a node moves it: an
/// anchor below can carry the size of new code inserted between the two
/// (`go-kubernetes-kubernetes-add-unit-test-cases`: new test functions after the edited one).
fn shift_at(anchors: &ShiftAnchors, byte: usize) -> i64 {
    anchors
        .range(..=byte)
        .next_back()
        .map_or(0, |(_, &shift)| shift)
}

/// Whether an edit lies among `nodes`, as the anchors see it: not all at the same shift.
fn shifts_differ(nodes: &[tree_sitter::Node], anchors: &ShiftAnchors) -> bool {
    let mut shifts = nodes.iter().map(|n| shift_at(anchors, n.start_byte()));
    let first = shifts.next();
    shifts.any(|shift| Some(shift) != first)
}

/// Matches each selected, still-unmatched before node to an unmatched after node with the same
/// value in the caller's hash, then maps both subtrees in lockstep. The caller picks the hash;
/// whether a pair is identical is always decided by `node_to_kind_and_value_hash`.
///
/// A single candidate is taken. Among several, the whole bucket - every selected, unmatched before
/// node and every unmatched after node with that hash - is paired at once, in document order
/// ([`ordered_assignment`]), at the least total distance between each after node and where its
/// before node lands under the shift of the matched roots above it ([`shift_at`]). Picking per
/// before node by raw byte distance was one-sided, and an edit above a run of twins shifts every
/// offset, so a twin one line down could be nearer than the node's own counterpart
/// (`javascript-microsoft-typescript-broken-js-remove-string-fragment`, `cpp-cgdb-cgdb-struct-to-
/// pointer`).
///
/// Content comes before position. A shape-only hash matches subtrees whose leaves differ, and the
/// candidates are ranked by what the lockstep descent from each pair would cost (`descent_cost`):
/// the cheapest pairs are taken first, in document order, then the next cost, and so on. Shape
/// alone paired image-map `<area>`s by position, href and alt aside
/// (`html-berndporr-iir1-a-lot-of-new-functionality`). Byte-identical candidates all cost 0, so
/// the kind-and-value tier is ranked by position alone.
///
/// The per-node pick stays for buckets of more than [`SEGMENT_SIMILARITY_MAX_CELLS`] pairs, and
/// for buckets with more before nodes than after nodes when neither content nor position tells
/// them apart - every pair costs the same and the before nodes all sit at one shift. Then
/// nothing locates which twin went, and the shift is only as good as the anchors above it: an
/// edit below the last one moves the after twins without the anchors seeing it
/// (`c-genymobile-scrcpy-big-change`: new `#include`s above three identical parameters).
// Every parameter is distinct context; a params struct would only relocate them.
#[allow(clippy::too_many_arguments)]
pub(crate) fn solve_with_hash_map(
    before: &Code,
    after: &Code,
    node_cache: &NodeCache,
    diff: &mut ASTDiff,
    before_hash: &rustc_hash::FxHashMap<usize, u64>,
    after_hash_to_nodes: &rustc_hash::FxHashMap<u64, Vec<usize>>,
    root_reason: ASTMappingReason,
    descendant_reason: ASTMappingReason,
    node_list_selector: impl Fn(&ASTMetadata) -> Vec<usize>,
    anchors: &mut ShiftAnchors,
) {
    let before_metadata = metadata_of(before);
    let after_metadata = metadata_of(after);

    let classify = |before_id: usize, after_id: usize| -> (ASTMappingOperation, u64) {
        let before_kv = before_metadata.node_to_kind_and_value_hash.get(&before_id);
        let after_kv = after_metadata.node_to_kind_and_value_hash.get(&after_id);
        if before_kv.is_some() && before_kv == after_kv {
            return (ASTMappingOperation::Identical, 0);
        }
        // As in `apted::common::classify_match`, `Update` is a leaf's own value. An interior node
        // differs only through descendants that carry their own cost; labelling it `Update` would
        // turn one renamed leaf into a mismatch at every ancestor.
        if before_metadata.is_leaf(before_id) && after_metadata.is_leaf(after_id) {
            (ASTMappingOperation::Update, crate::diff::COST_UPDATE)
        } else if before_metadata
            .node_info
            .get(&before_id)
            .map(|info| info.owned_text_hash)
            != after_metadata
                .node_info
                .get(&after_id)
                .map(|info| info.owned_text_hash)
        {
            // Its own gap text differs, which no descendant entry carries (see `operation_cost`).
            (
                ASTMappingOperation::MatchButNotIdentical,
                crate::diff::COST_UPDATE,
            )
        } else {
            (ASTMappingOperation::MatchButNotIdentical, 0)
        }
    };

    let map_pair =
        |diff: &mut ASTDiff, before_node: tree_sitter::Node, after_node: tree_sitter::Node| {
            let before_node_id = before_node.id();
            let (operation, cost) = classify(before_node_id, after_node.id());
            diff.add_mapping(
                before_node_id,
                after_node.id(),
                ASTMapping {
                    cost,
                    operation,
                    reason: root_reason,
                },
            );

            // Nodes whose own children were reordered; their ancestors are downgraded afterwards.
            let mut reordered_ids: Vec<usize> = Vec::new();
            let mut stack = vec![(before_node, after_node)];
            while let Some((before_parent, after_parent)) = stack.pop() {
                let (pairs, reordered) = pair_children_for_descent(
                    before_parent,
                    after_parent,
                    &before_metadata,
                    &after_metadata,
                );

                // The parent's mapping is already recorded, so patch it. A pure reorder is not a no-op:
                // the ground truth records it as `MatchButNotIdentical` at `COST_UPDATE`.
                if reordered {
                    if let Some(mapping) = diff
                        .mapping
                        .get_mut(&(before_parent.id(), after_parent.id()))
                    {
                        mapping.reason = ASTMappingReason::FullyMappingSubtrees;
                        mapping.operation = ASTMappingOperation::MatchButNotIdentical;
                        mapping.cost = crate::diff::COST_UPDATE;
                    }
                    reordered_ids.push(before_parent.id());
                }

                for (before_child, after_child) in pairs {
                    // Either side may already be decided: the kind-only run descends through subtrees
                    // the kind-and-value run already matched inside, and `add_mapping` would silently
                    // overwrite the earlier partner.
                    if diff.before_node_map.contains_key(&before_child.id())
                        || diff.after_node_map.contains_key(&after_child.id())
                    {
                        continue;
                    }
                    let (operation, cost) = classify(before_child.id(), after_child.id());
                    diff.add_mapping(
                        before_child.id(),
                        after_child.id(),
                        ASTMapping {
                            cost,
                            operation,
                            reason: descendant_reason,
                        },
                    );
                    stack.push((before_child, after_child));
                }
            }

            // A container is not a no-op if anything inside it was reordered, up to this match's root.
            // The ancestors keep their reason: only the reordered container itself reordered.
            for reordered_id in reordered_ids {
                let mut cur = reordered_id;
                while let Some(&parent_id) = before_metadata.node_to_parent.get(&cur) {
                    let Some(&after_parent_id) = diff.before_node_map.get(&parent_id) else {
                        break;
                    };
                    if let Some(mapping) = diff.mapping.get_mut(&(parent_id, after_parent_id))
                        && mapping.operation == ASTMappingOperation::Identical
                    {
                        mapping.operation = ASTMappingOperation::MatchButNotIdentical;
                        mapping.cost = crate::diff::COST_UPDATE;
                    }
                    if parent_id == before_node_id {
                        break;
                    }
                    cur = parent_id;
                }
            }
        };

    // What the lockstep descent from a root pair would cost (`classify` over every pair
    // `pair_children_for_descent` makes): how far two same-shaped subtrees differ in content.
    let descent_cost = |before_node: tree_sitter::Node, after_node: tree_sitter::Node| -> u64 {
        let identical = before_metadata
            .node_to_kind_and_value_hash
            .get(&before_node.id())
            .is_some_and(|hash| {
                after_metadata
                    .node_to_kind_and_value_hash
                    .get(&after_node.id())
                    == Some(hash)
            });
        if identical {
            return 0;
        }
        let mut total = classify(before_node.id(), after_node.id()).1;
        let mut stack = vec![(before_node, after_node)];
        while let Some((before_parent, after_parent)) = stack.pop() {
            let (pairs, _) = pair_children_for_descent(
                before_parent,
                after_parent,
                &before_metadata,
                &after_metadata,
            );
            for (before_child, after_child) in pairs {
                total += classify(before_child.id(), after_child.id()).1;
                stack.push((before_child, after_child));
            }
        }
        total
    };

    let before_node_ids = node_list_selector(&before_metadata);
    // A bucket's members share a hash, hence a size: they sit together in the largest-first order,
    // and nothing processed between them can match inside one.
    let mut bucket_of: rustc_hash::FxHashMap<u64, Vec<usize>> = rustc_hash::FxHashMap::default();
    for &id in &before_node_ids {
        if let Some(&hash) = before_hash.get(&id) {
            bucket_of.entry(hash).or_default().push(id);
        }
    }
    let preorder = |meta: &ASTMetadata, id: usize| {
        meta.node_info
            .get(&id)
            .map_or(usize::MAX, |info| info.preorder_index)
    };

    for &before_node_id in &before_node_ids {
        if diff.before_node_map.contains_key(&before_node_id) {
            continue;
        }
        let Some(&before_node) = node_cache.before.get(&before_node_id) else {
            continue;
        };
        let Some(before_hash_value) = before_hash.get(&before_node_id) else {
            continue;
        };
        let Some(after_candidates) = after_hash_to_nodes.get(before_hash_value) else {
            continue;
        };
        let free_after: Vec<tree_sitter::Node> = after_candidates
            .iter()
            .filter(|&&id| !diff.after_node_map.contains_key(&id))
            .filter_map(|id| node_cache.after.get(id).copied())
            .collect();
        let free_before: Vec<tree_sitter::Node> = if free_after.len() > 1 {
            bucket_of
                .get(before_hash_value)
                .into_iter()
                .flatten()
                .filter(|&&id| !diff.before_node_map.contains_key(&id))
                .filter_map(|id| node_cache.before.get(id).copied())
                .collect()
        } else {
            Vec::new()
        };

        let assigned = (free_after.len() > 1
            && free_before.len() * free_after.len() <= SEGMENT_SIMILARITY_MAX_CELLS)
            .then(|| {
                let mut free_before = free_before.clone();
                let mut free_after = free_after.clone();
                free_before.sort_by_key(|n| preorder(&before_metadata, n.id()));
                free_after.sort_by_key(|n| preorder(&after_metadata, n.id()));
                let distance: Vec<Vec<u64>> = free_before
                    .iter()
                    .map(|b| {
                        let expected = b.start_byte() as i64 + shift_at(anchors, b.start_byte());
                        free_after
                            .iter()
                            .map(|a| (a.start_byte() as i64).abs_diff(expected))
                            .collect()
                    })
                    .collect();
                let content: Vec<Vec<u64>> = free_before
                    .iter()
                    .map(|&b| free_after.iter().map(|&a| descent_cost(b, a)).collect())
                    .collect();
                let mut levels: Vec<u64> = content.iter().flatten().copied().collect();
                levels.sort_unstable();
                levels.dedup();
                if free_before.len() > free_after.len()
                    && levels.len() == 1
                    && !shifts_differ(&free_before, anchors)
                {
                    return None;
                }
                let mut used_before = vec![false; free_before.len()];
                let mut used_after = vec![false; free_after.len()];
                let mut pairs = Vec::new();
                // Cheapest content first; within one cost, in document order, and again for pairs
                // that cross the ones taken (two same-shaped functions swapped).
                for level in levels {
                    loop {
                        let cells: Vec<Vec<Option<u64>>> = (0..free_before.len())
                            .map(|bi| {
                                (0..free_after.len())
                                    .map(|ai| {
                                        (!used_before[bi]
                                            && !used_after[ai]
                                            && content[bi][ai] == level)
                                            .then_some(distance[bi][ai])
                                    })
                                    .collect()
                            })
                            .collect();
                        let round = ordered_assignment(&cells);
                        if round.is_empty() {
                            break;
                        }
                        for (bi, ai) in round {
                            used_before[bi] = true;
                            used_after[ai] = true;
                            pairs.push((free_before[bi], free_after[ai]));
                        }
                    }
                }
                Some(pairs)
            })
            .flatten();
        let pairs = match assigned {
            Some(pairs) => pairs,
            None => {
                // Proximity in the file, not discovery order, tells true duplicates from
                // coincidences.
                let Some(&after_node) = free_after
                    .iter()
                    .min_by_key(|n| n.start_byte().abs_diff(before_node.start_byte()))
                else {
                    continue;
                };
                vec![(before_node, after_node)]
            }
        };

        for (b, a) in pairs {
            map_pair(diff, b, a);
            for (before_byte, after_byte) in [
                (b.start_byte(), a.start_byte()),
                (b.end_byte(), a.end_byte()),
            ] {
                anchors
                    .entry(before_byte)
                    .or_insert(after_byte as i64 - before_byte as i64);
            }
        }
    }
}

/// The order-preserving pairing of `costs`' rows with its columns (both in document order) with
/// the most pairs, then the least total cost; `None` cells may not pair. On a tie, earlier columns
/// (rows) are used. Order-preserving because twins keep their order more often than not: before
/// any shift is known, raw offsets make a later twin nearer an earlier one's counterpart
/// (`typescript-refactor-interface`: an interface added above two identical class bodies).
fn ordered_assignment(costs: &[Vec<Option<u64>>]) -> Vec<(usize, usize)> {
    let n = costs.len();
    let m = costs.first().map_or(0, Vec::len);
    let better = |x: (usize, u64), y: (usize, u64)| x.0 > y.0 || (x.0 == y.0 && x.1 < y.1);
    // best[i][j]: (pairs, cost) of the best pairing within the first `i` rows and `j` columns.
    let mut best = vec![vec![(0usize, 0u64); m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            let mut cell = best[i - 1][j];
            if better(best[i][j - 1], cell) {
                cell = best[i][j - 1];
            }
            if let Some(cost) = costs[i - 1][j - 1] {
                let (pairs, total) = best[i - 1][j - 1];
                let take = (pairs + 1, total.saturating_add(cost));
                if better(take, cell) {
                    cell = take;
                }
            }
            best[i][j] = cell;
        }
    }
    let (mut i, mut j) = (n, m);
    let mut pairs = Vec::new();
    while i > 0 && j > 0 {
        if best[i][j - 1] == best[i][j] {
            j -= 1;
        } else if best[i - 1][j] == best[i][j] {
            i -= 1;
        } else {
            pairs.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        }
    }
    pairs.reverse();
    pairs
}

/// Pairs two matched parents' children for the lockstep descent, and reports whether a
/// commutative container's children were reordered.
///
/// Ordinary parents pair positionally, dropping kind mismatches: equal hashes computed in document
/// order imply positional correspondence. A `nodes::is_commutative_container` hashes its children
/// unordered, so its children pair by hash: kind-and-value first, then kind-only for what is left
/// (a kind-only hash outer match allows values to differ). Kind-only alone would pair differently
/// named identifiers arbitrarily and hide a real reorder.
///
/// Ties go to the nearest sibling index, not byte offset: an edit before the parents shifts every
/// offset inside them, and a shifted neighbouring comma can be closer than the right one.
pub(crate) fn pair_children_for_descent<'a>(
    before_parent: tree_sitter::Node<'a>,
    after_parent: tree_sitter::Node<'a>,
    before_metadata: &ASTMetadata,
    after_metadata: &ASTMetadata,
) -> (Vec<(tree_sitter::Node<'a>, tree_sitter::Node<'a>)>, bool) {
    let mut before_cursor = before_parent.walk();
    let mut after_cursor = after_parent.walk();
    let before_children: Vec<_> = before_parent.children(&mut before_cursor).collect();
    let after_children: Vec<_> = after_parent.children(&mut after_cursor).collect();

    let language = before_metadata.language;
    if !crate::diff::nodes::is_commutative_container(before_parent.kind(), &language) {
        let pairs = before_children
            .into_iter()
            .zip(after_children)
            .filter(|(b, a)| b.kind() == a.kind())
            .collect();
        return (pairs, false);
    }

    let index_by_hash = |children: &[tree_sitter::Node<'a>],
                         hash_map: &rustc_hash::FxHashMap<usize, u64>| {
        let mut by_hash: HashMap<u64, Vec<(usize, tree_sitter::Node<'a>)>> = HashMap::new();
        for (index, &child) in children.iter().enumerate() {
            let hash = hash_map.get(&child.id()).copied().unwrap_or(0);
            by_hash.entry(hash).or_default().push((index, child));
        }
        by_hash
    };
    let after_by_kv = index_by_hash(&after_children, &after_metadata.node_to_kind_and_value_hash);
    let after_by_ko = index_by_hash(&after_children, &after_metadata.node_to_kind_only_hash);

    let mut used = std::collections::HashSet::new();
    let mut pairs = Vec::new();
    let mut reordered = false;
    let mut unmatched_before: Vec<(usize, tree_sitter::Node<'a>)> = Vec::new();

    for (before_index, before_child) in before_children.into_iter().enumerate() {
        let kv_hash = before_metadata
            .node_to_kind_and_value_hash
            .get(&before_child.id())
            .copied()
            .unwrap_or(0);
        let found = after_by_kv
            .get(&kv_hash)
            .and_then(|candidates| {
                candidates
                    .iter()
                    .filter(|(_, c)| !used.contains(&c.id()) && c.kind() == before_child.kind())
                    .min_by_key(|(after_index, _)| after_index.abs_diff(before_index))
            })
            .copied();
        match found {
            Some((after_index, best)) => {
                used.insert(best.id());
                if after_index != before_index {
                    reordered = true;
                }
                pairs.push((before_child, best));
            }
            None => unmatched_before.push((before_index, before_child)),
        }
    }

    // Kind-only fallback for what the exact tier left.
    for (before_index, before_child) in unmatched_before {
        let ko_hash = before_metadata
            .node_to_kind_only_hash
            .get(&before_child.id())
            .copied()
            .unwrap_or(0);
        let Some(candidates) = after_by_ko.get(&ko_hash) else {
            continue;
        };
        let Some(&(after_index, best)) = candidates
            .iter()
            .filter(|(_, c)| !used.contains(&c.id()) && c.kind() == before_child.kind())
            .min_by_key(|(after_index, _)| after_index.abs_diff(before_index))
        else {
            continue;
        };
        used.insert(best.id());
        if after_index != before_index {
            reordered = true;
        }
        pairs.push((before_child, best));
    }

    (pairs, reordered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::Language;
    use crate::test::helper::find_first_of_kind;

    #[test]
    fn build_extended_node_list_always_includes_reference_nodes_regardless_of_size() {
        let code = Code::from_string("fn f() {}\n", &Language::Rust);
        let metadata = metadata_of(&code);
        let config = NodeSelectionConfig {
            min_depth: 0,
            min_subtree_size: usize::MAX,
        };
        let list = build_extended_node_list(&metadata, &config);

        let root = code.ast.as_ref().unwrap().root_node();
        let function_item = find_first_of_kind(root, "function_item").unwrap();
        assert!(
            list.contains(&function_item.id()),
            "a reference node must be included even though it's far smaller than min_subtree_size"
        );
    }

    #[test]
    fn build_extended_node_list_excludes_small_non_reference_nodes() {
        let code = Code::from_string("fn f() { let x = 1; }\n", &Language::Rust);
        let metadata = metadata_of(&code);
        let config = NodeSelectionConfig {
            min_depth: 0,
            min_subtree_size: usize::MAX,
        };
        let list = build_extended_node_list(&metadata, &config);

        let root = code.ast.as_ref().unwrap().root_node();
        let let_decl = find_first_of_kind(root, "let_declaration").unwrap();
        assert!(
            !list.contains(&let_decl.id()),
            "a small, non-reference node must be excluded when it can't meet the size threshold"
        );
    }

    #[test]
    fn build_extended_node_list_includes_non_reference_nodes_above_the_size_threshold() {
        let code = Code::from_string("fn f() { let x = 1; }\n", &Language::Rust);
        let metadata = metadata_of(&code);
        let config = NodeSelectionConfig {
            min_depth: 0,
            min_subtree_size: 1,
        };
        let list = build_extended_node_list(&metadata, &config);

        let root = code.ast.as_ref().unwrap().root_node();
        let let_decl = find_first_of_kind(root, "let_declaration").unwrap();
        assert!(
            list.contains(&let_decl.id()),
            "a low enough min_subtree_size must let ordinary, non-reference nodes in too"
        );
    }

    #[test]
    fn build_extended_node_list_sorts_by_subtree_size_descending() {
        let code = Code::from_string("fn f() { let x = 1; }\nfn g() {}\n", &Language::Rust);
        let metadata = metadata_of(&code);
        let config = NodeSelectionConfig {
            min_depth: 0,
            min_subtree_size: usize::MAX,
        };
        let list = build_extended_node_list(&metadata, &config);

        let sizes: Vec<usize> = list
            .iter()
            .map(|id| metadata.node_to_subtree_size.get(id).copied().unwrap_or(0))
            .collect();
        let mut sorted_desc = sizes.clone();
        sorted_desc.sort_by(|a, b| b.cmp(a));
        assert_eq!(
            sizes, sorted_desc,
            "list must be sorted by subtree size descending"
        );
    }

    #[test]
    fn pair_children_for_descent_zips_positionally_and_drops_mismatched_kinds() {
        let before = Code::from_string("fn f() { let x = 1; let y = 2; }\n", &Language::Rust);
        let after = Code::from_string("fn f() { let x = 1; return; }\n", &Language::Rust);
        let before_metadata = metadata_of(&before);
        let after_metadata = metadata_of(&after);

        let before_root = before.ast.as_ref().unwrap().root_node();
        let after_root = after.ast.as_ref().unwrap().root_node();
        let before_block = find_first_of_kind(before_root, "block").unwrap();
        let after_block = find_first_of_kind(after_root, "block").unwrap();

        let (pairs, reordered) =
            pair_children_for_descent(before_block, after_block, &before_metadata, &after_metadata);

        assert!(
            !reordered,
            "a block is not a commutative container - reordered must always be false"
        );
        assert!(
            pairs.iter().all(|(b, a)| b.kind() == a.kind()),
            "every returned pair must share a kind - the positional zip filters out kind mismatches: {:?}",
            pairs
                .iter()
                .map(|(b, a)| (b.kind(), a.kind()))
                .collect::<Vec<_>>()
        );
        assert!(
            pairs
                .iter()
                .any(|(b, a)| b.kind() == "let_declaration" && a.kind() == "let_declaration"),
            "the first, still-matching let_declaration must still be paired"
        );
    }

    #[test]
    fn pair_children_for_descent_detects_reordering_in_a_commutative_container() {
        let before = Code::from_string("use std::{a, b, c};\n", &Language::Rust);
        let after = Code::from_string("use std::{c, a, b};\n", &Language::Rust);
        let before_metadata = metadata_of(&before);
        let after_metadata = metadata_of(&after);

        let before_root = before.ast.as_ref().unwrap().root_node();
        let after_root = after.ast.as_ref().unwrap().root_node();
        let before_use_list = find_first_of_kind(before_root, "use_list").unwrap();
        let after_use_list = find_first_of_kind(after_root, "use_list").unwrap();

        let (pairs, reordered) = pair_children_for_descent(
            before_use_list,
            after_use_list,
            &before_metadata,
            &after_metadata,
        );

        assert!(
            reordered,
            "identical identifiers reshuffled inside a commutative container must be detected as reordered"
        );
        assert_eq!(
            pairs.len(),
            7,
            "every child must still be paired despite the reorder"
        );
    }

    #[test]
    fn pair_children_for_descent_reports_no_reorder_when_nothing_moved() {
        let before = Code::from_string("use std::{a, b, c};\n", &Language::Rust);
        let after = Code::from_string("use std::{a, b, c};\n", &Language::Rust);
        let before_metadata = metadata_of(&before);
        let after_metadata = metadata_of(&after);

        let before_root = before.ast.as_ref().unwrap().root_node();
        let after_root = after.ast.as_ref().unwrap().root_node();
        let before_use_list = find_first_of_kind(before_root, "use_list").unwrap();
        let after_use_list = find_first_of_kind(after_root, "use_list").unwrap();

        let (_, reordered) = pair_children_for_descent(
            before_use_list,
            after_use_list,
            &before_metadata,
            &after_metadata,
        );

        assert!(
            !reordered,
            "an unchanged commutative container must not be flagged as reordered"
        );
    }

    #[test]
    fn pair_children_for_descent_pairs_reordered_children_by_value_not_by_position() {
        let before = Code::from_string("use std::{a, b, c};\n", &Language::Rust);
        let after = Code::from_string("use std::{c, a, b};\n", &Language::Rust);
        let before_metadata = metadata_of(&before);
        let after_metadata = metadata_of(&after);
        let before_use_list =
            find_first_of_kind(before.ast.as_ref().unwrap().root_node(), "use_list").unwrap();
        let after_use_list =
            find_first_of_kind(after.ast.as_ref().unwrap().root_node(), "use_list").unwrap();

        let (pairs, _) = pair_children_for_descent(
            before_use_list,
            after_use_list,
            &before_metadata,
            &after_metadata,
        );

        for (b, a) in &pairs {
            assert_eq!(
                b.utf8_text(before.contents.as_bytes()).unwrap(),
                a.utf8_text(after.contents.as_bytes()).unwrap()
            );
        }
    }

    /// `rust-firefox-webrenderer-borders`: an edit earlier in the file shifts every byte offset
    /// inside the matched parents.
    #[test]
    fn pair_children_for_descent_breaks_ties_by_sibling_index_not_byte_offset() {
        let before = Code::from_string("use std::{a, a, a};\n", &Language::Rust);
        let after = Code::from_string("//\nuse std::{a, a, a};\n", &Language::Rust);
        let before_metadata = metadata_of(&before);
        let after_metadata = metadata_of(&after);
        let before_use_list =
            find_first_of_kind(before.ast.as_ref().unwrap().root_node(), "use_list").unwrap();
        let after_use_list =
            find_first_of_kind(after.ast.as_ref().unwrap().root_node(), "use_list").unwrap();

        let (pairs, reordered) = pair_children_for_descent(
            before_use_list,
            after_use_list,
            &before_metadata,
            &after_metadata,
        );

        assert!(!reordered);
        for (b, a) in &pairs {
            assert_eq!(
                b.start_byte() - before_use_list.start_byte(),
                a.start_byte() - after_use_list.start_byte()
            );
        }
    }

    /// `javascript-microsoft-typescript-broken-js-remove-string-fragment`: text removed above a
    /// pair of identical functions shifts both, by more than half a function, so the second one's
    /// raw offset sits nearer the first one's. The matched function above them carries the shift.
    #[test]
    fn identical_twins_pair_by_their_shifted_position_not_their_raw_offset() {
        let twin = "fn f() { a(1); }\n";
        let anchor = "fn h() { q(1, 2, 3); r(4, 5, 6); s(7, 8, 9); }\n";
        let before = Code::from_string(
            &format!("// a comment that the change removes\n{anchor}{twin}{twin}"),
            &Language::Rust,
        );
        let after = Code::from_string(&format!("{anchor}{twin}{twin}"), &Language::Rust);
        let node_cache = NodeCache::build(&before, &after);
        let mut diff = ASTDiff::default();

        crate::diff::solve_hash_descent::solve(
            &crate::diff::PassCtx::new(&before, &after, &node_cache),
            &mut diff,
        );

        let functions = |code: &Code| {
            let root = code.ast.as_ref().unwrap().root_node();
            let mut cursor = root.walk();
            root.children(&mut cursor)
                .filter(|n| n.kind() == "function_item")
                .map(|n| n.id())
                .collect::<Vec<_>>()
        };
        let (before_fns, after_fns) = (functions(&before), functions(&after));
        for i in 0..3 {
            assert_eq!(
                diff.before_node_map.get(&before_fns[i]),
                Some(&after_fns[i]),
                "function {i} pairs with its own counterpart"
            );
        }
    }

    /// Twins keep their order: a crossing pairing is never chosen, however cheap.
    #[test]
    fn ordered_assignment_preserves_order_and_skips_the_farthest_surplus() {
        let some = |rows: &[&[u64]]| -> Vec<Vec<Option<u64>>> {
            rows.iter()
                .map(|row| row.iter().map(|&c| Some(c)).collect())
                .collect()
        };
        assert_eq!(
            ordered_assignment(&some(&[&[5, 1], &[1, 5]])),
            vec![(0, 0), (1, 1)]
        );
        assert_eq!(ordered_assignment(&some(&[&[9, 0, 9]])), vec![(0, 1)]);
        assert_eq!(ordered_assignment(&some(&[&[9], &[0], &[9]])), vec![(1, 0)]);
        assert_eq!(ordered_assignment(&some(&[&[3, 3]])), vec![(0, 0)]);
        // Of two crossing pairs only one is taken, the cheaper; a `None` cell never pairs.
        assert_eq!(
            ordered_assignment(&[vec![None, Some(7)], vec![Some(1), None]]),
            vec![(1, 0)]
        );
    }

    #[test]
    fn shift_at_reads_the_anchor_above_and_zero_above_the_first() {
        let anchors: ShiftAnchors = [(10, 5), (20, -3)].into_iter().collect();
        assert_eq!(shift_at(&anchors, 5), 0);
        assert_eq!(shift_at(&anchors, 10), 5);
        assert_eq!(shift_at(&anchors, 19), 5);
        assert_eq!(shift_at(&anchors, 25), -3);
    }

    /// Two same-shaped functions swapped, each edited: the shape-only tier pairs each with the
    /// one its content matches, not the one at its position.
    #[test]
    fn same_shaped_twins_pair_by_content_before_position() {
        let before = Code::from_string("fn a() { x(1); }\nfn b() { y(2); }\n", &Language::Rust);
        // The new `c` keeps the two files' roots from sharing a shape.
        let after = Code::from_string(
            "fn b() { y(3); }\nfn a() { x(4); }\nfn c() {}\n",
            &Language::Rust,
        );
        let node_cache = NodeCache::build(&before, &after);
        let mut diff = ASTDiff::default();

        crate::diff::solve_hash_descent::solve(
            &crate::diff::PassCtx::new(&before, &after, &node_cache),
            &mut diff,
        );

        let functions = |code: &Code| {
            let root = code.ast.as_ref().unwrap().root_node();
            let mut cursor = root.walk();
            root.children(&mut cursor)
                .filter(|n| n.kind() == "function_item")
                .map(|n| n.id())
                .collect::<Vec<_>>()
        };
        let (before_fns, after_fns) = (functions(&before), functions(&after));
        assert_eq!(
            diff.before_node_map.get(&before_fns[0]),
            Some(&after_fns[1])
        );
        assert_eq!(
            diff.before_node_map.get(&before_fns[1]),
            Some(&after_fns[0])
        );
    }
}
