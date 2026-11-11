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

//! [`NodeLines`] from the engine's node mapping: which node pairs keep their rows, top down, and
//! what that says of each line. The rules are in the parent module's doc.

use rustc_hash::FxHashMap;
use tree_sitter::Node;

use super::{LineClass, LinePair, NodeLines, Span2, best_span_chain};
use crate::code::Code;
use crate::diff::ASTDiff;

/// No node.
const NONE: u32 = u32::MAX;

/// One side's tree in preorder: each node, where its subtree ends (exclusive), and the preorder
/// index of its partner on the other side ([`NONE`] when deleted, inserted, or outside that tree).
struct Side<'t> {
    nodes: Vec<Node<'t>>,
    end: Vec<u32>,
    partner: Vec<u32>,
}

impl Side<'_> {
    fn children(&self, node: u32) -> impl Iterator<Item = u32> + '_ {
        let end = self.end[node as usize];
        let mut next = node + 1;
        std::iter::from_fn(move || {
            let child = (next < end).then_some(next)?;
            next = self.end[child as usize];
            Some(child)
        })
    }

    /// Whether `node` lies strictly inside `ancestor`'s subtree.
    fn strictly_inside(&self, node: u32, ancestor: u32) -> bool {
        node != NONE && node > ancestor && node < self.end[ancestor as usize]
    }

    fn span(&self, node: u32) -> (usize, usize) {
        (node as usize, self.end[node as usize] as usize)
    }
}

/// The nodes of the tree under `root` in preorder, and each one's subtree end.
fn preorder(root: Node<'_>) -> (Vec<Node<'_>>, Vec<u32>) {
    let mut nodes = Vec::new();
    let mut end = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    let mut cursor = root.walk();
    loop {
        open.push(nodes.len());
        nodes.push(cursor.node());
        end.push(0);
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            let closed = open.pop().expect("an open node");
            end[closed] = u32::try_from(nodes.len()).expect("fewer than 2^32 nodes");
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return (nodes, end);
            }
        }
    }
}

/// [`preorder`], and each node's preorder index by its id.
#[allow(clippy::type_complexity)]
fn indexed(root: Node<'_>) -> (Vec<Node<'_>>, Vec<u32>, FxHashMap<usize, u32>) {
    let (nodes, end) = preorder(root);
    let mut index: FxHashMap<usize, u32> = FxHashMap::default();
    index.reserve(nodes.len());
    for (at, node) in nodes.iter().enumerate() {
        index.insert(node.id(), at as u32);
    }
    (nodes, end, index)
}

/// Per node, its partner's preorder index in `other` by `map` (node id to node id, 0 for none).
fn partners(
    nodes: &[Node<'_>],
    map: &FxHashMap<usize, usize>,
    other: &FxHashMap<usize, u32>,
) -> Vec<u32> {
    nodes
        .iter()
        .map(|node| {
            map.get(&node.id())
                .filter(|&&partner| partner != 0)
                .and_then(|partner| other.get(partner))
                .copied()
                .unwrap_or(NONE)
        })
        .collect()
}

/// A Fenwick tree of counts.
struct Counts(Vec<u32>);

impl Counts {
    fn add(&mut self, position: usize) {
        let mut i = position + 1;
        while i < self.0.len() {
            self.0[i] += 1;
            i += i & i.wrapping_neg();
        }
    }

    /// How many were added at positions `0..end`.
    fn before(&self, end: usize) -> u32 {
        let (mut sum, mut i) = (0, end);
        while i > 0 {
            sum += self.0[i];
            i &= i - 1;
        }
        sum
    }
}

/// For each `(old, new)` in `pairs`, how many mapped pairs (old node to its partner by the before
/// map) lie in the two subtrees: points in a rectangle of the two preorders, counted offline in
/// one sweep over the old preorder, the rectangles' corners bucketed by their old position.
fn weights(before: &Side<'_>, after: &Side<'_>, pairs: &[(u32, u32)]) -> Vec<u64> {
    // F(x, y): points with old < x and new < y; a rectangle is four corners, two at each x.
    let mut starts = vec![0u32; before.nodes.len() + 2];
    for &(old, _) in pairs {
        starts[old as usize + 1] += 2;
        starts[before.end[old as usize] as usize + 1] += 2;
    }
    for x in 1..starts.len() {
        starts[x] += starts[x - 1];
    }
    let mut fill = starts.clone();
    // (y, pair, whether it adds)
    let mut corners: Vec<(u32, u32, bool)> = vec![(0, 0, false); pairs.len() * 4];
    for (index, &(old, new)) in pairs.iter().enumerate() {
        let (y0, y1) = (new, after.end[new as usize]);
        for (x, ys) in [
            (old, [(y0, true), (y1, false)]),
            (before.end[old as usize], [(y1, true), (y0, false)]),
        ] {
            for (y, plus) in ys {
                corners[fill[x as usize] as usize] = (y, index as u32, plus);
                fill[x as usize] += 1;
            }
        }
    }
    let mut counts = Counts(vec![0; after.nodes.len() + 1]);
    let mut totals = vec![0i64; pairs.len()];
    for x in 0..=before.nodes.len() {
        for &(y, index, plus) in &corners[starts[x] as usize..starts[x + 1] as usize] {
            let count = i64::from(counts.before(y as usize));
            totals[index as usize] += if plus { count } else { -count };
        }
        if x < before.nodes.len() && before.partner[x] != NONE {
            counts.add(before.partner[x] as usize);
        }
    }
    totals
        .into_iter()
        .map(|total| u64::try_from(total).expect("a count"))
        .collect()
}

/// The rows `node` touches, its first and last: an end at column 0 of a later row does not touch
/// that row.
fn rows_of(node: &Node<'_>) -> (usize, usize) {
    let (start, end) = (node.start_position(), node.end_position());
    let last = if end.column == 0 && end.row > start.row {
        end.row - 1
    } else {
        end.row
    };
    (start.row, last.max(start.row))
}

/// Both sides' trees in preorder and indexed by node id: what [`node_lines`] needs before there
/// is a mapping, so it is built beside the engine's diff.
pub(crate) struct Trees<'code> {
    before: Indexed<'code>,
    after: Indexed<'code>,
}

type Indexed<'code> = (Vec<Node<'code>>, Vec<u32>, FxHashMap<usize, u32>);

impl<'code> Trees<'code> {
    /// `None` when either side has no tree.
    pub(crate) fn of(before_code: &'code Code, after_code: &'code Code) -> Option<Self> {
        let (before_tree, after_tree) = (before_code.ast.as_ref()?, after_code.ast.as_ref()?);
        let (before, after) = crate::code::build_sides(
            crate::code::worth_building_in_parallel(&before_code.contents, &after_code.contents),
            || indexed(before_tree.root_node()),
            || indexed(after_tree.root_node()),
        );
        Some(Self { before, after })
    }
}

/// [`NodeLines`] of a pair from `ast`'s mapping of the before tree onto the after tree (see the
/// parent module's doc), `trees` being [`Trees::of`] `before_code` and `after_code`.
pub(crate) fn node_lines(
    trees: Trees<'_>,
    before_code: &Code,
    after_code: &Code,
    ast: &ASTDiff,
) -> NodeLines {
    let Trees {
        before: (before_nodes, before_end, before_index),
        after: (after_nodes, after_end, after_index),
    } = trees;
    let (before_partner, after_partner) = crate::code::build_sides(
        crate::code::worth_building_in_parallel(&before_code.contents, &after_code.contents),
        || partners(&before_nodes, &ast.before_node_map, &after_index),
        || partners(&after_nodes, &ast.after_node_map, &before_index),
    );
    let before = Side {
        nodes: before_nodes,
        end: before_end,
        partner: before_partner,
    };
    let after = Side {
        nodes: after_nodes,
        end: after_end,
        partner: after_partner,
    };

    // Every mapped pair weighed: the before map's, per old node, and the after map's that are not
    // also the before map's (group members), per new node.
    let mut pairs: Vec<(u32, u32)> = Vec::new();
    for (old, &new) in before.partner.iter().enumerate() {
        if new != NONE {
            pairs.push((old as u32, new));
        }
    }
    let before_pairs = pairs.len();
    for (new, &old) in after.partner.iter().enumerate() {
        if old != NONE && before.partner[old as usize] != new as u32 {
            pairs.push((old, new as u32));
        }
    }
    let weighed = weights(&before, &after, &pairs);
    let mut before_weight = vec![0u64; before.nodes.len()];
    for (&(old, _), &weight) in pairs[..before_pairs].iter().zip(&weighed) {
        before_weight[old as usize] = weight;
    }
    let mut after_weight = vec![0u64; after.nodes.len()];
    for (new, &old) in after.partner.iter().enumerate() {
        if old != NONE && before.partner[old as usize] == new as u32 {
            after_weight[new] = before_weight[old as usize];
        }
    }
    for (&(_, new), &weight) in pairs[before_pairs..].iter().zip(&weighed[before_pairs..]) {
        after_weight[new as usize] = weight;
    }
    // Top down from the two roots, kept whatever they map to.
    let mut kept_before = vec![NONE; before.nodes.len()];
    let mut kept_after = vec![NONE; after.nodes.len()];
    let mut stack: Vec<(u32, u32)> = vec![(0, 0)];
    let mut candidates: Vec<(u32, u32, u64)> = Vec::new();
    let mut walk: Vec<u32> = Vec::new();
    while let Some((old, new)) = stack.pop() {
        kept_before[old as usize] = new;
        kept_after[new as usize] = old;
        candidates.clear();
        // The frontier of each side: the topmost nodes mapped into the other side's subtree.
        for (side, other, root, other_root, flip) in [
            (&before, &after, old, new, false),
            (&after, &before, new, old, true),
        ] {
            walk.clear();
            walk.extend(side.children(root));
            while let Some(node) = walk.pop() {
                let partner = side.partner[node as usize];
                if other.strictly_inside(partner, other_root) {
                    let weight = if flip { &after_weight } else { &before_weight }[node as usize];
                    candidates.push(if flip {
                        (partner, node, weight)
                    } else {
                        (node, partner, weight)
                    });
                } else {
                    walk.extend(side.children(node));
                }
            }
        }
        candidates.sort_unstable();
        candidates.dedup_by_key(|&mut (old, new, _)| (old, new));
        let spans: Vec<Span2> = candidates
            .iter()
            .map(|&(old, new, weight)| Span2 {
                old: before.span(old),
                new: after.span(new),
                weight,
            })
            .collect();
        stack.extend(best_span_chain(&spans).into_iter().map(|index| {
            let (old, new, _) = candidates[index];
            (old, new)
        }));
    }

    // Each line from its tokens; each kept token names its rows and its partner's.
    let line_count = |code: &Code| code.contents.split('\n').count();
    let ((old_lines, old_anchors), (new_lines, new_anchors)) = crate::code::build_sides(
        crate::code::worth_building_in_parallel(&before_code.contents, &after_code.contents),
        || classify(&before, &after, &kept_before, line_count(before_code)),
        || classify(&after, &before, &kept_after, line_count(after_code)),
    );
    let mut anchors: Vec<(usize, usize)> = old_anchors;
    anchors.extend(new_anchors.into_iter().map(|(new, old)| (old, new)));
    anchors.sort_unstable();
    let mut counted: Vec<LinePair> = Vec::with_capacity(anchors.len());
    for (old, new) in anchors {
        match counted.last_mut() {
            Some(last) if (last.old, last.new) == (old, new) => last.support += 1,
            _ => counted.push(LinePair {
                old,
                new,
                support: 1,
            }),
        }
    }
    NodeLines {
        anchors: counted,
        old: old_lines,
        new: new_lines,
    }
}

/// `side`'s `lines` lines classed by their tokens, and the line pairs (this side's line first) its
/// kept tokens name, `kept` giving each node's kept partner in `other`.
fn classify(
    side: &Side<'_>,
    other: &Side<'_>,
    kept: &[u32],
    lines: usize,
) -> (Vec<LineClass>, Vec<(usize, usize)>) {
    let mut classes = vec![LineClass::Blank; lines];
    let mut anchors = Vec::new();
    for (at, node) in side.nodes.iter().enumerate() {
        if side.end[at] as usize != at + 1 || node.start_byte() == node.end_byte() {
            continue;
        }
        let (first, last) = rows_of(node);
        let class = if kept[at] != NONE {
            let (other_first, other_last) = rows_of(&other.nodes[kept[at] as usize]);
            anchors.push((first, other_first));
            if (first, other_first) != (last, other_last) {
                anchors.push((last, other_last));
            }
            LineClass::Kept
        } else if side.partner[at] != NONE {
            LineClass::Moved(other.nodes[side.partner[at] as usize].start_position().row)
        } else {
            LineClass::Unmatched
        };
        for line in classes.iter_mut().take(last + 1).skip(first) {
            line.raise(class);
        }
    }
    (classes, anchors)
}

#[cfg(test)]
mod tests;
