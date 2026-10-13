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
//! Pre-matching passes that pin obvious pairs before the general search runs: identical statement
//! siblings, identical if-chain clauses wherever they moved in the chain, and locals that are
//! unique by name.

use super::*;

/// Minimum direct-child count worth pre-matching via [`prematch_identical_statement_siblings`].
/// Far below `FLAT_MIN_CHILDREN` because that pass never commits a non-match.
pub(crate) const STATEMENT_PREMATCH_MIN_CHILDREN: usize = 4;

/// `(child count, id)` of the widest `nodes::is_statement_sequence_body` node at the shallowest
/// depth that has one, `root_id` included. Not `node_to_widest_subtree_node`, which is
/// kind-agnostic and can pick a wider but irrelevant node.
pub(crate) fn widest_statement_sequence_body(
    root_id: usize,
    meta: &ASTMetadata,
) -> Option<(usize, usize)> {
    // Shallowest, not widest overall: a nested loop body is the same kind as the function body,
    // and may be wider on one side only, pairing two unrelated sequences.
    let mut level = vec![root_id];
    while !level.is_empty() {
        let mut best: Option<(usize, usize)> = None;
        let mut next_level = Vec::new();
        for id in level {
            let Some(info) = meta.node_info.get(&id) else {
                continue;
            };
            if nodes::is_statement_sequence_body(&info.kind) {
                let count = info.children.len();
                if best.is_none_or(|(best_count, _)| count > best_count) {
                    best = Some((count, id));
                }
            } else {
                next_level.extend(info.children.iter().copied());
            }
        }
        if best.is_some() {
            return best;
        }
        level = next_level;
    }
    None
}

/// Pre-matches the byte-identical direct children of the two sides' statement-sequence bodies
/// (Myers over full hashes), so the scoped APTED call that follows indexes only the rest.
///
/// Unlike [`resolve_flat_tree_pair`] it emits only identical matches and leaves every other
/// child untouched: in a body of 10-40 statements the few that differ deserve a real APTED
/// resolution, not a committed delete/insert.
pub(crate) fn prematch_identical_statement_siblings(
    before_id: usize,
    after_id: usize,
    before_meta: &ASTMetadata,
    after_meta: &ASTMetadata,
    source: &'static str,
    diff: &mut ASTDiff,
) {
    let Some((before_count, before_flat)) = widest_statement_sequence_body(before_id, before_meta)
    else {
        return;
    };
    let Some((after_count, after_flat)) = widest_statement_sequence_body(after_id, after_meta)
    else {
        return;
    };
    if before_count < STATEMENT_PREMATCH_MIN_CHILDREN
        || after_count < STATEMENT_PREMATCH_MIN_CHILDREN
    {
        return;
    }

    let Some(before_info) = before_meta.node_info.get(&before_flat) else {
        return;
    };
    let Some(after_info) = after_meta.node_info.get(&after_flat) else {
        return;
    };
    let before_children: Vec<usize> = before_info
        .children
        .iter()
        .copied()
        .filter(|id| !diff.before_node_map.contains_key(id))
        .collect();
    let after_children: Vec<usize> = after_info
        .children
        .iter()
        .copied()
        .filter(|id| !diff.after_node_map.contains_key(id))
        .collect();
    if before_children.is_empty() || after_children.is_empty() {
        return;
    }

    let before_hashes: Vec<u64> = before_children
        .iter()
        .map(|&id| before_meta.node_to_full_hash.get(&id).copied().unwrap_or(0))
        .collect();
    let after_hashes: Vec<u64> = after_children
        .iter()
        .map(|&id| after_meta.node_to_full_hash.get(&id).copied().unwrap_or(0))
        .collect();

    let Some(pairs) = myers_lcs(&before_hashes, &after_hashes, FLAT_MAX_EDIT) else {
        return;
    };
    for (bi, ai) in pairs {
        emit_identical_subtree(
            before_children[bi],
            after_children[ai],
            before_meta,
            after_meta,
            source,
            diff,
        );
    }
}

/// Buckets every not-yet-mapped node under `node_id` (inclusive) by
/// `nodes::local_identity_name`. It descends into nested functions on purpose: two same-named
/// locals in different scopes then disqualify the name, which keeps uniqueness strict.
pub(crate) fn collect_local_identities(
    node_id: usize,
    meta: &ASTMetadata,
    node_map: &rustc_hash::FxHashMap<usize, usize>,
    language: &Language,
    groups: &mut rustc_hash::FxHashMap<(&'static str, String), Vec<usize>>,
) {
    if node_map.contains_key(&node_id) {
        return;
    }
    let Some(info) = meta.node_info.get(&node_id) else {
        return;
    };
    if let Some(key) = nodes::local_identity_name(node_id, meta, language) {
        groups.entry(key).or_default().push(node_id);
    }
    for &child_id in &info.children {
        collect_local_identities(child_id, meta, node_map, language, groups);
    }
}

/// Pre-matches locals (parameters, local variables, shell assignments; see
/// `nodes::local_identity_name`) whose name is unique on both sides, each through its own scoped
/// `for_nodes` call, so a changed body still gets `MatchButNotIdentical`.
///
/// Unit-cost APTED has no sense that "same name, shifted position" beats "same position,
/// different name", so an insertion mid-sequence often pairs locals by position. Ambiguous names
/// are left to APTED.
pub(crate) fn prematch_unique_named_locals(
    before_id: usize,
    after_id: usize,
    before_meta: &ASTMetadata,
    after_meta: &ASTMetadata,
    source: &'static str,
    diff: &mut ASTDiff,
) {
    let language = before_meta.language;
    if !nodes::has_local_identity_coverage(&language) {
        return;
    }

    let mut before_groups: rustc_hash::FxHashMap<(&'static str, String), Vec<usize>> =
        rustc_hash::FxHashMap::default();
    collect_local_identities(
        before_id,
        before_meta,
        &diff.before_node_map,
        &language,
        &mut before_groups,
    );
    if before_groups.is_empty() {
        return;
    }
    let mut after_groups: rustc_hash::FxHashMap<(&'static str, String), Vec<usize>> =
        rustc_hash::FxHashMap::default();
    collect_local_identities(
        after_id,
        after_meta,
        &diff.after_node_map,
        &language,
        &mut after_groups,
    );
    if after_groups.is_empty() {
        return;
    }

    let mut pairs: Vec<(usize, usize)> = before_groups
        .iter()
        .filter(|(_, ids)| ids.len() == 1)
        .filter_map(|(key, before_ids)| {
            let after_ids = after_groups.get(key)?;
            (after_ids.len() == 1).then(|| (before_ids[0], after_ids[0]))
        })
        .collect();
    // `HashMap` iteration order is not deterministic.
    pairs.sort_unstable_by_key(|&(b, _)| {
        before_meta
            .node_info
            .get(&b)
            .map(|i| i.preorder_index)
            .unwrap_or(usize::MAX)
    });

    for (before_child, after_child) in pairs {
        for_nodes(
            before_meta,
            after_meta,
            vec![before_child],
            vec![after_child],
            Algorithm::Apted,
            source,
            diff,
        );
    }
}
/// Kinds that open one clause of an if-chain: the `if` itself and the wrapped later clauses
/// (Lua `elseif_statement`, Python `elif_clause`, Ruby `elsif`, PHP `else_if_clause`).
fn is_if_clause_frame(kind: &str) -> bool {
    matches!(
        kind,
        "if_statement"
            | "if_expression"
            | "elseif_statement"
            | "elif_clause"
            | "else_if_clause"
            | "elsif"
            | "if"
    )
}

/// Kinds that hold the rest of an if-chain rather than a clause's own condition or body.
fn is_if_alternative(kind: &str) -> bool {
    kind.contains("else") || matches!(kind, "elif_clause" | "elsif")
}

/// A clause's own parts: its frame's named children (keywords and punctuation are anonymous)
/// minus the alternative - an `else`-kind child, or everything after a bare `else` token (Go's
/// `else if`).
fn if_clause_parts(frame: usize, meta: &ASTMetadata) -> Vec<usize> {
    let Some(info) = meta.node_info.get(&frame) else {
        return Vec::new();
    };
    let mut parts = Vec::new();
    for &child in &info.children {
        let Some(child_info) = meta.node_info.get(&child) else {
            continue;
        };
        if !child_info.is_named {
            if child_info.kind == "else" {
                break;
            }
            continue;
        }
        if is_if_alternative(&child_info.kind) {
            continue;
        }
        parts.push(child);
    }
    parts
}

/// Whether `frame` is one clause of several: it has an alternative, or it is one (an `elseif`
/// kind, or an `if` directly under an `else` or under another `if`, as Go's `else if` is).
fn in_if_chain(frame: usize, meta: &ASTMetadata) -> bool {
    let Some(info) = meta.node_info.get(&frame) else {
        return false;
    };
    let has_alternative = info.children.iter().any(|child| {
        meta.node_info.get(child).is_some_and(|c| {
            (c.is_named && is_if_alternative(&c.kind)) || (!c.is_named && c.kind == "else")
        })
    });
    let is_alternative = is_if_alternative(&info.kind)
        || meta
            .node_to_parent
            .get(&frame)
            .and_then(|parent| meta.node_info.get(parent))
            .is_some_and(|p| is_if_alternative(&p.kind) || is_if_clause_frame(&p.kind));
    has_alternative || is_alternative
}

/// An if-chain clause: its frame and its parts ([`if_clause_parts`]).
type IfClause = (usize, Vec<usize>);

/// Every unmapped clause under `root_id`, keyed by its parts' full hashes.
fn collect_if_clauses(
    root_id: usize,
    meta: &ASTMetadata,
    node_map: &rustc_hash::FxHashMap<usize, usize>,
    out: &mut rustc_hash::FxHashMap<Vec<u64>, Vec<IfClause>>,
) {
    let mut stack = vec![root_id];
    while let Some(id) = stack.pop() {
        if node_map.contains_key(&id) {
            continue;
        }
        let Some(info) = meta.node_info.get(&id) else {
            continue;
        };
        if info.is_named && is_if_clause_frame(&info.kind) {
            let parts = if_clause_parts(id, meta);
            if !parts.is_empty() && parts.iter().all(|p| !node_map.contains_key(p)) {
                let key: Vec<u64> = parts
                    .iter()
                    .map(|p| meta.node_to_full_hash.get(p).copied().unwrap_or(0))
                    .collect();
                out.entry(key).or_default().push((id, parts));
            }
        }
        stack.extend(info.children.iter().rev().copied());
    }
}

/// Pre-matches the if-chain clauses (condition and body) that are byte-identical and unique on
/// both sides, wherever they sit in the chain, so two swapped branches keep their content
/// (`lua-neovim-neovim-if-flips-two-branches`). Ordered APTED cannot express the swap and renames
/// both branches in place instead.
///
/// Both clauses must belong to a chain of several ([`in_if_chain`]). A lone `if` that moved is a
/// statement, left to APTED and phase 7: pinned here, `java-defects4j-cli-14-groupimpl`'s moved
/// `if (validate)` block maps as the human's does (19 -> 2 visible mismatches) but paints worse
/// (+28 Full, +38 Minimal bytes), the renderer blaming what the block jumped over.
///
/// Only the parts are matched, the frames only when the whole frame is identical: in Lua the
/// first clause sits bare in `if_statement` and the second in `elseif_statement`, so a swap moves
/// each body to a frame of another kind.
pub(crate) fn prematch_identical_if_clauses(
    before_id: usize,
    after_id: usize,
    before_meta: &ASTMetadata,
    after_meta: &ASTMetadata,
    source: &'static str,
    diff: &mut ASTDiff,
) {
    let mut before_clauses = rustc_hash::FxHashMap::default();
    collect_if_clauses(
        before_id,
        before_meta,
        &diff.before_node_map,
        &mut before_clauses,
    );
    if before_clauses.is_empty() {
        return;
    }
    let mut after_clauses = rustc_hash::FxHashMap::default();
    collect_if_clauses(
        after_id,
        after_meta,
        &diff.after_node_map,
        &mut after_clauses,
    );

    let mut pairs: Vec<(&IfClause, &IfClause)> = before_clauses
        .iter()
        .filter_map(|(key, before)| {
            let after = after_clauses.get(key)?;
            (before.len() == 1
                && after.len() == 1
                && in_if_chain(before[0].0, before_meta)
                && in_if_chain(after[0].0, after_meta))
            .then(|| (&before[0], &after[0]))
        })
        .collect();
    // `HashMap` iteration order is not deterministic.
    pairs.sort_unstable_by_key(|((b, _), _)| {
        before_meta
            .node_info
            .get(b)
            .map(|i| i.preorder_index)
            .unwrap_or(usize::MAX)
    });
    for ((before_frame, before_parts), (after_frame, after_parts)) in pairs {
        // A nested clause may already be inside an earlier pair's parts.
        if diff.before_node_map.contains_key(before_frame)
            || before_parts
                .iter()
                .any(|p| diff.before_node_map.contains_key(p))
            || after_parts
                .iter()
                .any(|p| diff.after_node_map.contains_key(p))
        {
            continue;
        }
        let same_frame = before_meta.node_to_full_hash.get(before_frame)
            == after_meta.node_to_full_hash.get(after_frame)
            && !diff.after_node_map.contains_key(after_frame);
        if same_frame {
            emit_identical_subtree(
                *before_frame,
                *after_frame,
                before_meta,
                after_meta,
                source,
                diff,
            );
            continue;
        }
        for (&b, &a) in before_parts.iter().zip(after_parts) {
            emit_identical_subtree(b, a, before_meta, after_meta, source, diff);
        }
    }
}
