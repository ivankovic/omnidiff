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
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
//! Which before-definition is which after-definition, and what happened to each.
//!
//! **Pairing reads the engine's node mapping**, the thing a name-based tool (sem-core, GitHub
//! Semantic's `--toc`) does not have, in three steps of falling trust:
//!
//! 1. the two definition nodes are paired by the mapping (under a new name, only with step 2's
//!    majority too: the engine pairs two functions by their shell when nothing better is left);
//! 2. otherwise the before definition goes to the after definition most of its leaves landed in
//!    with their text: more than half of the larger one's named leaves (identifiers, literals),
//!    its own name left out, each definition taken once, the strongest vote first, so a function
//!    renamed and edited still pairs;
//! 3. otherwise to an unpaired after definition of the same qualified name.
//!
//! Only definitions of one family pair (functions with methods, types with types, ...): a
//! function that became a struct is a removal and an addition.
//!
//! **What happened** is read off the painted ranges of the Minimal preset, the one that paints
//! only what changed: a definition is *modified* when a range of either side falls inside it,
//! except a `Move` range over its own name, which is the definition itself moving, and a range
//! within its name, which is its rename. It is
//! *moved* when its enclosing definition is not its old one's partner, or when it moved within
//! its parent (the engine paired it as a moved subtree, or painted its name `Move`). It is
//! *renamed* when its name differs. *Cosmetic only* means every range inside it is whitespace or
//! within a comment.
//!
//! **Order:** the after side's document order, a removed definition right after the
//! paired definition that preceded it in the before side.

use std::collections::HashMap;
use std::ops::Range;

use super::definitions::{Definition, DefinitionChange};
use super::{Diff, Side};
use crate::diff::text::{RangeMatch, TextOperation};
use crate::diff::text_range::{SourceColumn, SourceRow, SourceText};
use crate::diff::{ASTMappingReason, nodes};

pub(crate) fn pair(diff: &Diff, sides: &[Vec<Definition>; 2]) -> Vec<DefinitionChange> {
    let Some(ast) = diff.engine_ast() else {
        return Vec::new();
    };
    let [before, after] = sides;
    let mut partner_of_before: Vec<Option<usize>> = vec![None; before.len()];
    let mut partner_of_after: Vec<Option<usize>> = vec![None; after.len()];

    // Where each before definition's leaves went: for every named leaf (an identifier, a
    // literal; punctuation and keywords would let any two functions' shells outvote their
    // bodies) outside the definition's own name, the innermost after definition around its
    // partner when the partner kept its text, else `None`.
    let (before_text, after_text) = (diff.text(Side::Before), diff.text(Side::After));
    let after_leaves = leaves_by_innermost(diff, Side::After, after);
    let after_leaf: HashMap<usize, (usize, &Range<usize>)> = after_leaves
        .iter()
        .filter_map(|(id, owner, bytes)| Some((*id, ((*owner)?, bytes))))
        .collect();
    let mut size_after = vec![0usize; after.len()];
    for (_, owner, bytes) in &after_leaves {
        for j in chain(after, *owner).filter(|&j| !covers(&after[j].name_span().bytes, bytes)) {
            size_after[j] += 1;
        }
    }
    let mut landed: Vec<Vec<Option<usize>>> = vec![Vec::new(); before.len()];
    for (leaf, owner, bytes) in leaves_by_innermost(diff, Side::Before, before) {
        let into = ast
            .before_node_map
            .get(&leaf)
            .and_then(|p| after_leaf.get(p))
            .filter(|(_, partner)| {
                after_text.get((*partner).clone()) == before_text.get(bytes.clone())
            })
            .map(|(owner, _)| *owner);
        for i in chain(before, owner).filter(|&i| !covers(&before[i].name_span().bytes, &bytes)) {
            landed[i].push(into);
        }
    }
    // More than half of the larger definition's leaves are matched across the two.
    let majority =
        |i: usize, j: usize, votes: usize| 2 * votes > landed[i].len().max(size_after[j]);
    let support = |i: usize, j: usize| {
        landed[i]
            .iter()
            .filter(|l| chain(after, **l).any(|x| x == j))
            .count()
    };

    // 1. The definition nodes themselves are paired. Under another name only with the majority
    // of step 2 as well: the engine pairs two functions by their shell (`fn`, `(&self)`, the
    // braces) when nothing better is left, and a reader sees one removed and another added.
    let after_by_node: HashMap<usize, usize> = after
        .iter()
        .enumerate()
        .map(|(j, d)| (d.node_id, j))
        .collect();
    for (i, d) in before.iter().enumerate() {
        let Some(&partner) = ast.before_node_map.get(&d.node_id) else {
            continue;
        };
        if let Some(&j) = after_by_node.get(&partner)
            && partner_of_after[j].is_none()
            && after[j].kind().family() == d.kind().family()
            && (after[j].name() == d.name() || majority(i, j, support(i, j)))
        {
            link(i, j, &mut partner_of_before, &mut partner_of_after);
        }
    }

    // 2. Where the leaves went.
    let mut votes: HashMap<(usize, usize), usize> = HashMap::new();
    for (i, leaves) in landed.iter().enumerate() {
        if partner_of_before[i].is_some() {
            continue;
        }
        for &into in leaves {
            for j in chain(after, into) {
                if partner_of_after[j].is_none()
                    && after[j].kind().family() == before[i].kind().family()
                {
                    *votes.entry((i, j)).or_default() += 1;
                }
            }
        }
    }
    let mut candidates: Vec<((usize, usize), usize)> = votes
        .into_iter()
        .filter(|&((i, j), v)| majority(i, j, v))
        .collect();
    candidates.sort_by_key(|&((i, j), v)| (std::cmp::Reverse(v), i, j));
    for ((i, j), _) in candidates {
        if partner_of_before[i].is_none() && partner_of_after[j].is_none() {
            link(i, j, &mut partner_of_before, &mut partner_of_after);
        }
    }

    // 3. The same qualified name.
    for (i, d) in before.iter().enumerate() {
        if partner_of_before[i].is_some() {
            continue;
        }
        if let Some(j) = (0..after.len()).find(|&j| {
            partner_of_after[j].is_none()
                && after[j].kind().family() == d.kind().family()
                && after[j].qualified_name() == d.qualified_name()
        }) {
            link(i, j, &mut partner_of_before, &mut partner_of_after);
        }
    }

    let painted = [
        changed_bytes(diff, Side::Before),
        changed_bytes(diff, Side::After),
    ];
    let mut entries: Vec<((usize, u8, usize), DefinitionChange)> = Vec::new();
    for (j, a) in after.iter().enumerate() {
        let change = match partner_of_after[j] {
            None => DefinitionChange {
                before: None,
                after: Some(a.clone()),
                renamed: false,
                moved: false,
                modified: false,
                cosmetic_only: false,
            },
            Some(i) => {
                let b = &before[i];
                let parent_moved = b.parent.and_then(|p| partner_of_before[p]) != a.parent;
                let moved_within = ast
                    .mapping
                    .get(&(b.node_id, a.node_id))
                    .is_some_and(|m| m.reason == ASTMappingReason::MovedSubtree)
                    || painted[1]
                        .overlapping(a.name_span().bytes.clone())
                        .any(|r| {
                            r.operation == TextOperation::Move
                                && covers(&r.bytes, &a.name_span().bytes)
                        });
                let inside = |side: usize, d: &Definition| -> Vec<&Changed> {
                    painted[side]
                        .overlapping(d.span().bytes.clone())
                        // The definition's own move, and its own rename, are not changes to it.
                        .filter(|r| {
                            !(r.operation == TextOperation::Move
                                && covers(&r.bytes, &d.name_span().bytes))
                                && !covers(&d.name_span().bytes, &r.bytes)
                        })
                        .collect()
                };
                let changed_before = inside(0, b);
                let changed_after = inside(1, a);
                let modified = !changed_before.is_empty() || !changed_after.is_empty();
                let cosmetic_only = modified
                    && changed_before
                        .iter()
                        .all(|r| is_cosmetic(diff, Side::Before, &r.bytes))
                    && changed_after
                        .iter()
                        .all(|r| is_cosmetic(diff, Side::After, &r.bytes));
                DefinitionChange {
                    before: Some(b.clone()),
                    after: Some(a.clone()),
                    renamed: b.name() != a.name(),
                    moved: parent_moved || moved_within,
                    modified,
                    cosmetic_only,
                }
            }
        };
        entries.push(((a.span().bytes.start, 0, j), change));
    }
    // A removed definition goes right after the paired one before it in the before side.
    let mut anchor = 0usize;
    for (i, b) in before.iter().enumerate() {
        match partner_of_before[i] {
            Some(j) => anchor = after[j].span().bytes.start,
            None => entries.push((
                (anchor, 1, i),
                DefinitionChange {
                    before: Some(b.clone()),
                    after: None,
                    renamed: false,
                    moved: false,
                    modified: false,
                    cosmetic_only: false,
                },
            )),
        }
    }
    entries.sort_by_key(|(key, _)| *key);
    entries.into_iter().map(|(_, change)| change).collect()
}

fn link(
    i: usize,
    j: usize,
    partner_of_before: &mut [Option<usize>],
    partner_of_after: &mut [Option<usize>],
) {
    partner_of_before[i] = Some(j);
    partner_of_after[j] = Some(i);
}

/// `owner` and the definitions around it, innermost first.
fn chain(definitions: &[Definition], owner: Option<usize>) -> impl Iterator<Item = usize> + '_ {
    std::iter::successors(owner, |&i| definitions[i].parent)
}

/// Every named leaf of a side's tree, with the innermost definition around it, if any, and its
/// bytes.
fn leaves_by_innermost(
    diff: &Diff,
    side: Side,
    definitions: &[Definition],
) -> Vec<(usize, Option<usize>, Range<usize>)> {
    let Some(tree) = diff.code(side).ast.as_ref() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    let mut next = 0usize;
    let mut cursor = tree.walk();
    'walk: loop {
        let node = cursor.node();
        while open
            .last()
            .is_some_and(|&i| node.start_byte() >= definitions[i].node_bytes.end)
        {
            open.pop();
        }
        // Definitions are in preorder, as this walk visits nodes.
        while next < definitions.len() && definitions[next].node_id == node.id() {
            open.push(next);
            next += 1;
        }
        if node.child_count() == 0 && node.is_named() {
            out.push((node.id(), open.last().copied(), node.byte_range()));
        }
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                continue 'walk;
            }
            if !cursor.goto_parent() {
                break 'walk;
            }
        }
    }
    out
}

/// A painted range of one side, in bytes.
struct Changed {
    bytes: Range<usize>,
    operation: TextOperation,
}

/// One side's [`Changed`] ranges by start, for finding the ones a definition overlaps without
/// a scan of all of them per definition.
struct Painted {
    ranges: Vec<Changed>,
    /// The longest range: one overlapping `bytes` starts at most this far before it.
    longest: usize,
}

impl Painted {
    fn overlapping(&self, bytes: Range<usize>) -> impl Iterator<Item = &Changed> {
        let from = self
            .ranges
            .partition_point(|r| r.bytes.start + self.longest <= bytes.start);
        self.ranges[from..]
            .iter()
            .take_while(move |r| r.bytes.start < bytes.end)
            .filter(move |r| overlaps(&r.bytes, &bytes))
    }
}

/// `side`'s Minimal ranges that say something changed.
fn changed_bytes(diff: &Diff, side: Side) -> Painted {
    let text = SourceText::new(diff.text(side));
    let ranges: &[RangeMatch] = &diff.painted_minimal()[match side {
        Side::Before => 0,
        Side::After => 1,
    }];
    let mut changed: Vec<Changed> = ranges
        .iter()
        .filter(|r| {
            matches!(
                r.operation,
                TextOperation::Insert
                    | TextOperation::Delete
                    | TextOperation::Update
                    | TextOperation::Move
            )
        })
        .filter_map(|r| {
            let at = |row, column| {
                text.byte_index(SourceRow::from_raw(row), SourceColumn::from_raw(column))
                    .map(|o| o.get())
            };
            let start = at(r.source.start_row, r.source.start_column)?;
            let end = at(r.source.end_row, r.source.end_column)?;
            (end > start).then(|| Changed {
                bytes: start..end,
                operation: r.operation.clone(),
            })
        })
        .collect();
    changed.sort_by_key(|r| (r.bytes.start, r.bytes.end));
    let longest = changed.iter().map(|r| r.bytes.len()).max().unwrap_or(0);
    Painted {
        ranges: changed,
        longest,
    }
}

fn overlaps(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}

fn covers(outer: &Range<usize>, inner: &Range<usize>) -> bool {
    outer.start <= inner.start && inner.end <= outer.end && !inner.is_empty()
}

/// Whether `bytes` of `side` are only whitespace, or inside a comment.
fn is_cosmetic(diff: &Diff, side: Side, bytes: &Range<usize>) -> bool {
    let text = diff.text(side);
    if text.get(bytes.clone()).is_some_and(|t| t.trim().is_empty()) {
        return true;
    }
    let Some(tree) = diff.code(side).ast.as_ref() else {
        return false;
    };
    let mut node = tree
        .root_node()
        .descendant_for_byte_range(bytes.start, bytes.end);
    while let Some(n) = node {
        if nodes::is_comment(n.kind()) {
            return true;
        }
        node = n.parent();
    }
    false
}
