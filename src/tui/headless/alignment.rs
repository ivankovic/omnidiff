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

//! Which before line sits beside which after line in the side-by-side layout
//! (`super::side_by_side`).
//!
//! **Rows follow the engine's node mapping** ([`mapping::node_lines`]), not the painting and not
//! git's patch. The painting depends on the `M` panel's render options (Full and Minimal paint
//! different ranges of the same diff), so rows read from it changed with them; the mapping does
//! not. Without a tree on either side (plain text, a side over `MAX_AST_NODES`) the painting's
//! ranges are all there is, and the older rule below applies.
//!
//! **Kept node pairs, top down.** The two roots are kept. Under a kept pair `P_b <-> P_a`, the
//! *candidates* are each side's frontier: walking down from `P_b`'s children, the topmost nodes
//! whose partner lies strictly inside `P_a`'s subtree (and the same from `P_a`'s side, so a node
//! below an unmapped or elsewhere-mapped node on either side is still found). Usually that is a
//! child mapped to a child; it is also a node mapped across parents, such as a statement wrapped
//! into a new `if` (example 2). The kept candidates are the heaviest chain of them that is in
//! order and nested in neither on both sides ([`best_span_chain`], `O(c log c)`), weight being
//! the number of mapped node pairs inside the two subtrees (counted for every mapped pair at once,
//! by an offline sweep over both preorders). Ties: more pairs, then earlier in the old file.
//! Every kept candidate is recursed into; every other one is a *move*, and so is any mapped node
//! below it or not reached from a kept pair: no row is drawn from it, and it is never revisited.
//!
//! **Lines from tokens.** A line is `Kept` if a token (leaf) on it is in a kept pair, else
//! `Moved` if one is mapped (naming the line of the first such token's partner), else `Unmatched`
//! if it holds a token, else `Blank`. Each kept token names its first rows on both sides, and its
//! last rows if it spans more than one, as a line pair; *tokens* rather than every kept node,
//! because a kept Python `block` starts on its first statement's row, which may be a moved one.
//! The rows keep the chain of those line pairs strictly increasing on both sides with the most
//! tokens naming them ([`best_chain`]); each is a row.
//!
//! **Between two anchor rows**, `Kept` lines of the two sides pair in order (a kept token whose
//! line pair lost to a crossing one, or the inside of a kept multi-line string), and so do
//! `Blank` lines of equal text (an LCS over the gap, skipped past [`GAP_FILL_LIMIT`] cells). A
//! `Moved` or `Unmatched` line never pairs: it is one-sided, the old lines first, then the new,
//! as git orders a change block; a `Moved` one names its counterpart, `→ N` / `← N`.
//!
//! Example 1, `def total` (loop, `return s`) moved below `def mean`, `total` -> `sum` inside it.
//! The module's two `def`s cross; the mapping holds 25 pairs inside `mean` (all of it, the
//! renamed call an update) and 12 inside `total` (its header and `return`; the loop deleted), so
//! `mean` keeps its two rows under every preset and `total`'s lines are one-sided: header and
//! `return` `→ 5`/`→ 6` and `← 1`/`← 5`, the loop's lines plain, the blank lines alone because
//! they fall in different gaps. Example 2, `return 1` wrapped in a new `if x > 1:`: the
//! `return_statement` is mapped from the outer block into the new `if`'s block, in order, so it
//! is a candidate of the kept outer blocks and keeps its row; `if x > 1:` is `Unmatched`, alone.
//!
//! **Without a tree: matched ranges.** Every `Identical`, `Update` and `Move` range says that the
//! text it covers on one side *is* the text it names on the other, so each line it touches is
//! paired with the line its destination covers at the same offset (a range whose destination has
//! fewer rows, such as an `Update`'s zero-width anchor, clamps to its last row). Both sides'
//! lists are read, since a range can be recorded on one side only. `Insert` and `Delete` carry a
//! destination too, but it is an anchor, not a match, and pairs nothing. A pair drawn from several
//! ranges is the same pair; its `support` counts them. The rows keep the best chain of pairs
//! strictly increasing in both lines: the most *updated* lines (painted `~`, each side counted),
//! then the most pairs, then the chain that ends earlier in the old file. Between two pairs of
//! the chain the lines no range touches pair by equal text; every other line is one-sided and
//! names, per old line, the new line its best-supported pair names (lowest on a tie), and back.
//!
//! **The cap.** Above [`PAIR_CAP`] range pairs the chain is not computed and the rows follow
//! git's patch instead ([`rows_by_patch`]): context lines share a row, and in a change block the
//! n-th deleted line sits beside the n-th added one.

use std::cmp::Reverse;

use crate::diff::text::{RangeMatch, TextOperation};
use crate::diff::text_range::TextRange;

pub(crate) mod mapping;

/// Above this many line pairs the rows follow git's patch ([`rows_by_patch`]).
pub(crate) const PAIR_CAP: usize = 2_000_000;

/// The largest gap (old lines times new lines without any pair) whose blank and other unmatched
/// lines are paired by equal text.
const GAP_FILL_LIMIT: usize = 1 << 20;

/// One row: the 0-based old line on the left, the new line on the right; `None` is a blank half.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RowLines {
    pub old: Option<usize>,
    pub new: Option<usize>,
}

impl RowLines {
    pub fn pair(old: usize, new: usize) -> Self {
        Self {
            old: Some(old),
            new: Some(new),
        }
    }

    pub fn old(old: usize) -> Self {
        Self {
            old: Some(old),
            new: None,
        }
    }

    pub fn new(new: usize) -> Self {
        Self {
            old: None,
            new: Some(new),
        }
    }
}

/// Old line `old` is new line `new` by `support` ranges (0-based lines).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinePair {
    pub old: usize,
    pub new: usize,
    pub support: u32,
}

/// The rows of a side-by-side layout, and for each line on a one-sided row the line on the other
/// side it was matched to, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Alignment {
    pub rows: Vec<RowLines>,
    pub old_counterpart: Vec<Option<usize>>,
    pub new_counterpart: Vec<Option<usize>>,
}

/// The rows `range` touches, its first and last: an end at column 0 of a later row does not
/// touch that row.
fn touched_rows(range: &TextRange) -> (usize, usize) {
    let last = if range.end_column == 0 && range.end_row > range.start_row {
        range.end_row - 1
    } else {
        range.end_row
    };
    (range.start_row, last.max(range.start_row))
}

/// Every line pair the matched ranges of both sides name (see the module doc), sorted by old then
/// new line, each once with its support; lines past `old_lines` and `new_lines` are dropped.
pub(crate) fn line_pairs(
    before_ranges: &[RangeMatch],
    after_ranges: &[RangeMatch],
    old_lines: usize,
    new_lines: usize,
) -> Vec<LinePair> {
    let matched = |range: &RangeMatch| {
        matches!(
            range.operation,
            TextOperation::Identical | TextOperation::Update | TextOperation::Move
        ) && !range.source.is_empty()
    };
    let mut raw: Vec<(usize, usize)> = Vec::new();
    let mut add = |source: &TextRange, destination: &TextRange, flip: bool| {
        let (first, last) = touched_rows(source);
        let (dest_first, dest_last) = touched_rows(destination);
        for row in first..=last {
            let other = (dest_first + (row - first)).min(dest_last);
            raw.push(if flip { (other, row) } else { (row, other) });
        }
    };
    for range in before_ranges.iter().filter(|range| matched(range)) {
        add(&range.source, &range.destination, false);
    }
    for range in after_ranges.iter().filter(|range| matched(range)) {
        add(&range.source, &range.destination, true);
    }
    raw.retain(|&(old, new)| old < old_lines && new < new_lines);
    raw.sort_unstable();
    let mut pairs: Vec<LinePair> = Vec::with_capacity(raw.len());
    for (old, new) in raw {
        match pairs.last_mut() {
            Some(last) if (last.old, last.new) == (old, new) => last.support += 1,
            _ => pairs.push(LinePair {
                old,
                new,
                support: 1,
            }),
        }
    }
    pairs
}

/// A chain's rank: weight (updated lines, or mapped nodes), then members, then ending earlier in
/// the old file (and then the new). Larger is better.
type Score = (u64, u64, Reverse<usize>, Reverse<usize>);

/// A Fenwick tree of prefix maxima over new positions: the best chain ending before a position.
struct PrefixMax {
    tree: Vec<Option<(Score, usize)>>,
}

impl PrefixMax {
    fn new(size: usize) -> Self {
        Self {
            tree: vec![None; size + 1],
        }
    }

    /// The best entry at positions `0..end`.
    fn best_before(&self, end: usize) -> Option<(Score, usize)> {
        let mut best = None;
        let mut i = end;
        while i > 0 {
            best = best.max(self.tree[i]);
            i &= i - 1;
        }
        best
    }

    fn offer(&mut self, position: usize, entry: (Score, usize)) {
        let mut i = position + 1;
        while i < self.tree.len() {
            if self.tree[i].is_none_or(|current| current < entry) {
                self.tree[i] = Some(entry);
            }
            i += i & i.wrapping_neg();
        }
    }
}

/// Something with an extent on each side, half-open and never empty: a line (`[n, n + 1)`) or a
/// node's subtree in preorder (`[first, past its last descendant)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Span2 {
    pub old: (usize, usize),
    pub new: (usize, usize),
    pub weight: u64,
}

/// The indices into `items` of the heaviest chain in which each item ends, on both sides, before
/// the next one starts, in order; ranked by [`Score`]. A weighted longest increasing subsequence,
/// `O(n log n)` whatever the positions (they are ranked first, so a call on a few children of a
/// node deep in a large tree stays small): items are taken by old start, and one joins the
/// prefix-maximum tree (keyed by its new end) once an item starting at or after its old end is
/// reached.
pub(crate) fn best_span_chain(items: &[Span2]) -> Vec<usize> {
    let mut by_start: Vec<usize> = (0..items.len()).collect();
    by_start.sort_unstable_by_key(|&index| (items[index].old.0, items[index].new.0));
    let mut by_end: Vec<usize> = (0..items.len()).collect();
    by_end.sort_unstable_by_key(|&index| items[index].old.1);
    // Each item's last new position, ranked.
    let mut lasts: Vec<usize> = items.iter().map(|item| item.new.1 - 1).collect();
    lasts.sort_unstable();
    lasts.dedup();
    let rank = |position: usize| lasts.partition_point(|&last| last < position);
    let mut prefix = PrefixMax::new(lasts.len());
    let mut scores: Vec<Option<(Score, usize)>> = vec![None; items.len()];
    let mut previous: Vec<Option<usize>> = vec![None; items.len()];
    let mut best: Option<(Score, usize)> = None;
    let mut joined = 0;
    for &index in &by_start {
        let item = &items[index];
        while joined < by_end.len() && items[by_end[joined]].old.1 <= item.old.0 {
            let done = by_end[joined];
            if let Some(entry) = scores[done] {
                prefix.offer(rank(items[done].new.1 - 1), entry);
            }
            joined += 1;
        }
        let before = prefix.best_before(rank(item.new.0));
        previous[index] = before.map(|(_, at)| at);
        let (weight, length) = before.map_or((0, 0), |((w, n, _, _), _)| (w, n));
        let entry = (
            (
                weight + item.weight,
                length + 1,
                Reverse(item.old.0),
                Reverse(item.new.0),
            ),
            index,
        );
        scores[index] = Some(entry);
        best = best.max(Some(entry));
    }
    let mut chain = Vec::new();
    let mut at = best.map(|(_, index)| index);
    while let Some(index) = at {
        chain.push(index);
        at = previous[index];
    }
    chain.reverse();
    chain
}

/// The indices into `pairs` (sorted by old, then new line) of the best chain strictly increasing
/// in both lines, in order, where a pair's `updated` is its weight (see the module doc). `None`
/// above `cap` pairs.
pub(crate) fn best_chain(
    pairs: &[LinePair],
    updated: impl Fn(&LinePair) -> u64,
    cap: usize,
) -> Option<Vec<usize>> {
    if pairs.len() > cap {
        return None;
    }
    let spans: Vec<Span2> = pairs
        .iter()
        .map(|pair| Span2 {
            old: (pair.old, pair.old + 1),
            new: (pair.new, pair.new + 1),
            weight: updated(pair),
        })
        .collect();
    Some(best_span_chain(&spans))
}

/// The pairs `(i, j)` with `matches(i, j)` of a longest common subsequence of `0..old` and `0..new`
/// under that predicate; none when the table would pass [`GAP_FILL_LIMIT`] cells.
fn lcs(old: usize, new: usize, matches: impl Fn(usize, usize) -> bool) -> Vec<(usize, usize)> {
    if old == 0 || new == 0 || old.saturating_mul(new) > GAP_FILL_LIMIT {
        return Vec::new();
    }
    let width = new + 1;
    // `table[i * width + j]`: the LCS of `old[i..]` and `new[j..]`.
    let mut table = vec![0u32; (old + 1) * width];
    for i in (0..old).rev() {
        for j in (0..new).rev() {
            table[i * width + j] = if matches(i, j) {
                table[(i + 1) * width + j + 1] + 1
            } else {
                table[(i + 1) * width + j].max(table[i * width + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < old && j < new {
        if matches(i, j) {
            out.push((i, j));
            i += 1;
            j += 1;
        } else if table[(i + 1) * width + j] >= table[i * width + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

/// The rows through `anchors` (strictly increasing line pairs, each a row of its own): between
/// two anchors the lines `free` on both sides pair by an LCS under `matches`, every other line is
/// one-sided, old lines first. A one-sided line names `counterpart`'s line, if any.
fn rows_through(
    (old_lines, new_lines): (usize, usize),
    anchors: impl Iterator<Item = (usize, usize)>,
    (free_old, free_new): (impl Fn(usize) -> bool, impl Fn(usize) -> bool),
    matches: impl Fn(usize, usize) -> bool,
    (old_counterpart, new_counterpart): (
        impl Fn(usize) -> Option<usize>,
        impl Fn(usize) -> Option<usize>,
    ),
) -> Alignment {
    let mut rows = Vec::with_capacity(old_lines.max(new_lines));
    let (mut next_old, mut next_new) = (0, 0);
    let one_sided = |rows: &mut Vec<RowLines>, from: (usize, usize), to: (usize, usize)| {
        rows.extend((from.0..to.0).map(RowLines::old));
        rows.extend((from.1..to.1).map(RowLines::new));
    };
    for (anchor_old, anchor_new) in anchors.chain(std::iter::once((old_lines, new_lines))) {
        let gap_old: Vec<usize> = (next_old..anchor_old).filter(|&l| free_old(l)).collect();
        let gap_new: Vec<usize> = (next_new..anchor_new).filter(|&l| free_new(l)).collect();
        for (i, j) in lcs(gap_old.len(), gap_new.len(), |i, j| {
            matches(gap_old[i], gap_new[j])
        }) {
            let (fill_old, fill_new) = (gap_old[i], gap_new[j]);
            one_sided(&mut rows, (next_old, next_new), (fill_old, fill_new));
            rows.push(RowLines::pair(fill_old, fill_new));
            (next_old, next_new) = (fill_old + 1, fill_new + 1);
        }
        one_sided(&mut rows, (next_old, next_new), (anchor_old, anchor_new));
        if anchor_old < old_lines {
            rows.push(RowLines::pair(anchor_old, anchor_new));
        }
        (next_old, next_new) = (anchor_old + 1, anchor_new + 1);
    }

    // Only a one-sided line names its counterpart.
    let mut old_named: Vec<Option<usize>> = vec![None; old_lines];
    let mut new_named: Vec<Option<usize>> = vec![None; new_lines];
    for row in &rows {
        match (row.old, row.new) {
            (Some(line), None) => old_named[line] = old_counterpart(line),
            (None, Some(line)) => new_named[line] = new_counterpart(line),
            _ => {}
        }
    }
    Alignment {
        rows,
        old_counterpart: old_named,
        new_counterpart: new_named,
    }
}

/// The rows of a side-by-side layout of `old` and `new` (each side's lines) from the painting's
/// `pairs` (see the module doc), `updated` weighting the chain; `None` above `cap` pairs.
pub(crate) fn align(
    old: &[&str],
    new: &[&str],
    pairs: &[LinePair],
    updated: impl Fn(&LinePair) -> u64,
    cap: usize,
) -> Option<Alignment> {
    let chain = best_chain(pairs, updated, cap)?;

    // The best-supported counterpart of every line with a pair at all.
    let mut old_counterpart: Vec<Option<(Reverse<u32>, usize)>> = vec![None; old.len()];
    let mut new_counterpart: Vec<Option<(Reverse<u32>, usize)>> = vec![None; new.len()];
    for pair in pairs {
        let offer = |slot: &mut Option<(Reverse<u32>, usize)>, other: usize| {
            let candidate = (Reverse(pair.support), other);
            if slot.is_none_or(|current| candidate < current) {
                *slot = Some(candidate);
            }
        };
        offer(&mut old_counterpart[pair.old], pair.new);
        offer(&mut new_counterpart[pair.new], pair.old);
    }
    let anchors = chain
        .iter()
        .map(|&index| (pairs[index].old, pairs[index].new));
    Some(rows_through(
        (old.len(), new.len()),
        anchors,
        (
            |line: usize| old_counterpart[line].is_none(),
            |line: usize| new_counterpart[line].is_none(),
        ),
        |o, n| old[o] == new[n],
        (
            |line: usize| old_counterpart[line].map(|(_, other)| other),
            |line: usize| new_counterpart[line].map(|(_, other)| other),
        ),
    ))
}

/// What the node mapping says of one line (see the module doc), by the tokens on it, strongest
/// first: one of them is in a kept pair; else one is mapped elsewhere (a move, to the line of its
/// partner's start); else one is deleted or inserted; else there is no token on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineClass {
    Kept,
    Moved(usize),
    Unmatched,
    Blank,
}

impl LineClass {
    fn rank(self) -> u8 {
        match self {
            LineClass::Kept => 3,
            LineClass::Moved(_) => 2,
            LineClass::Unmatched => 1,
            LineClass::Blank => 0,
        }
    }

    /// `self` raised to `other` if `other` is stronger; the first move named stays.
    pub fn raise(&mut self, other: LineClass) {
        if other.rank() > self.rank() {
            *self = other;
        }
    }
}

/// The lines of a pair as the node mapping sees them ([`mapping::node_lines`]): the line pairs
/// the kept tokens name, each with how many name it, and each line's [`LineClass`]. Independent
/// of the `M` panel's render options.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct NodeLines {
    pub anchors: Vec<LinePair>,
    pub old: Vec<LineClass>,
    pub new: Vec<LineClass>,
}

/// The rows of a side-by-side layout of `old` and `new` from the node mapping's `lines` (see the
/// module doc).
pub(crate) fn align_by_nodes(old: &[&str], new: &[&str], lines: &NodeLines) -> Alignment {
    let anchors: Vec<LinePair> = lines
        .anchors
        .iter()
        .copied()
        .filter(|pair| pair.old < old.len() && pair.new < new.len())
        .collect();
    let chain = best_chain(&anchors, |pair| u64::from(pair.support), usize::MAX).expect("no cap");
    let class =
        |classes: &[LineClass], line: usize| classes.get(line).copied().unwrap_or(LineClass::Blank);
    let (old_class, new_class) = (
        |line: usize| class(&lines.old, line),
        |line: usize| class(&lines.new, line),
    );
    let free = |class: LineClass| matches!(class, LineClass::Kept | LineClass::Blank);
    let moved = |class: LineClass| match class {
        LineClass::Moved(other) => Some(other),
        _ => None,
    };
    rows_through(
        (old.len(), new.len()),
        chain
            .iter()
            .map(|&index| (anchors[index].old, anchors[index].new)),
        (|line| free(old_class(line)), |line| free(new_class(line))),
        |o, n| match (old_class(o), new_class(n)) {
            (LineClass::Kept, LineClass::Kept) => true,
            (LineClass::Blank, LineClass::Blank) => old[o] == new[n],
            _ => false,
        },
        (|line| moved(old_class(line)), |line| moved(new_class(line))),
    )
}

/// The rows by git's patch, for a pair above the cap: an old line `is_deleted` deletes and a new
/// line `is_added` adds (0-based) form change blocks, whose n-th deleted line sits beside their
/// n-th added one; every other line is context and shares a row with its counterpart.
pub(crate) fn rows_by_patch(
    old_lines: usize,
    new_lines: usize,
    is_deleted: impl Fn(usize) -> bool,
    is_added: impl Fn(usize) -> bool,
) -> Alignment {
    let mut rows = Vec::with_capacity(old_lines.max(new_lines));
    let (mut old, mut new) = (0, 0);
    while old < old_lines || new < new_lines {
        let deleted_end = (old..old_lines)
            .find(|&line| !is_deleted(line))
            .unwrap_or(old_lines);
        let added_end = (new..new_lines)
            .find(|&line| !is_added(line))
            .unwrap_or(new_lines);
        if deleted_end == old && added_end == new {
            // Context; a patch that does not fit leaves one side's tail one-sided.
            match (old < old_lines, new < new_lines) {
                (true, true) => rows.push(RowLines::pair(old, new)),
                (true, false) => rows.push(RowLines::old(old)),
                (false, _) => rows.push(RowLines::new(new)),
            }
            old += usize::from(old < old_lines);
            new += usize::from(new < new_lines);
            continue;
        }
        let (deleted, added) = (deleted_end - old, added_end - new);
        for k in 0..deleted.max(added) {
            rows.push(RowLines {
                old: (k < deleted).then_some(old + k),
                new: (k < added).then_some(new + k),
            });
        }
        (old, new) = (deleted_end, added_end);
    }
    Alignment {
        rows,
        old_counterpart: vec![None; old_lines],
        new_counterpart: vec![None; new_lines],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(old: usize, new: usize) -> LinePair {
        LinePair {
            old,
            new,
            support: 1,
        }
    }

    fn chain_of(pairs: &[LinePair], updated: &[(usize, usize)]) -> Vec<(usize, usize)> {
        best_chain(
            pairs,
            |p| u64::from(updated.contains(&(p.old, p.new))),
            PAIR_CAP,
        )
        .unwrap()
        .into_iter()
        .map(|index| (pairs[index].old, pairs[index].new))
        .collect()
    }

    #[test]
    fn crossing_pairs_keep_the_longer_in_order_chain() {
        // 0-3 crossed by 1-0, 2-1, 3-2: the three in order beat the one.
        let pairs = [pair(0, 3), pair(1, 0), pair(2, 1), pair(3, 2)];
        assert_eq!(chain_of(&pairs, &[]), vec![(1, 0), (2, 1), (3, 2)]);
    }

    #[test]
    fn a_chain_is_strictly_increasing_on_both_sides() {
        // One old line matched to two new lines, and two old lines to one new line.
        let pairs = [pair(0, 0), pair(0, 1), pair(1, 1), pair(2, 1), pair(3, 2)];
        let chain = chain_of(&pairs, &[]);
        assert_eq!(chain.len(), 3, "{chain:?}");
        assert!(chain.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 < w[1].1));
    }

    #[test]
    fn updated_lines_win_over_more_pairs() {
        // A block of three moved intact (0-2..2-4) against two crossing pairs, one updated.
        let pairs = [pair(0, 3), pair(1, 4), pair(2, 5), pair(3, 0), pair(4, 1)];
        assert_eq!(chain_of(&pairs, &[]), vec![(0, 3), (1, 4), (2, 5)]);
        assert_eq!(chain_of(&pairs, &[(4, 1)]), vec![(3, 0), (4, 1)]);
    }

    #[test]
    fn a_tie_goes_to_the_chain_earlier_in_the_old_file() {
        let pairs = [pair(0, 2), pair(1, 3), pair(2, 0), pair(3, 1)];
        assert_eq!(chain_of(&pairs, &[]), vec![(0, 2), (1, 3)]);
        assert_eq!(
            chain_of(&pairs, &[(0, 2), (3, 1)]),
            vec![(0, 2), (1, 3)],
            "one updated line each: still a tie"
        );
    }

    #[test]
    fn above_the_cap_there_is_no_chain() {
        let pairs = [pair(0, 0), pair(1, 1)];
        assert!(best_chain(&pairs, |_| 0, 1).is_none());
        assert!(align(&["a", "b"], &["a", "b"], &pairs, |_| 0, 1).is_none());
        assert_eq!(best_chain(&pairs, |_| 0, 2).unwrap().len(), 2);
    }

    #[test]
    fn one_sided_lines_come_old_first_and_name_their_counterpart() {
        // Old 0 moved to new 2; old 1 and new 0..2 unmatched; old 2 == new 3.
        let old = ["moved", "gone", "same"];
        let new = ["fresh", "fresh2", "moved", "same"];
        let pairs = [pair(0, 2), pair(2, 3)];
        let alignment = align(&old, &new, &pairs, |_| 0, PAIR_CAP).unwrap();
        // The chain keeps both (0-2, 2-3); new 0 and 1 come before the first pair.
        assert_eq!(
            alignment.rows,
            vec![
                RowLines::new(0),
                RowLines::new(1),
                RowLines::pair(0, 2),
                RowLines::old(1),
                RowLines::pair(2, 3),
            ]
        );
        assert_eq!(alignment.old_counterpart, vec![None; 3]);

        // Crossing: old 0 <-> new 1 and old 1 <-> new 0; the earlier old line wins.
        let pairs = [pair(0, 1), pair(1, 0)];
        let alignment = align(&["a", "b"], &["b", "a"], &pairs, |_| 0, PAIR_CAP).unwrap();
        assert_eq!(
            alignment.rows,
            vec![RowLines::new(0), RowLines::pair(0, 1), RowLines::old(1)]
        );
        assert_eq!(alignment.old_counterpart, vec![None, Some(0)]);
        assert_eq!(alignment.new_counterpart, vec![Some(1), None]);
    }

    #[test]
    fn unmatched_equal_lines_between_pairs_share_a_row() {
        // Blank lines no range covers.
        let old = ["a", "", "b"];
        let new = ["a", "", "x", "b"];
        let alignment = align(&old, &new, &[pair(0, 0), pair(2, 3)], |_| 0, PAIR_CAP).unwrap();
        assert_eq!(
            alignment.rows,
            vec![
                RowLines::pair(0, 0),
                RowLines::pair(1, 1),
                RowLines::new(2),
                RowLines::pair(2, 3),
            ]
        );
    }

    #[test]
    fn between_anchors_kept_lines_pair_in_order_and_moved_or_unmatched_ones_never() {
        use LineClass::*;
        // Old: a, X (moved to new 5), "", k1, k2, z; new: a, I (inserted), "", k1, Y (moved from
        // old 1), X, z. Anchors a and z only.
        let old = ["a", "x", "", "k1", "k2", "z"];
        let new = ["a", "i", "", "k1", "y", "x", "z"];
        let lines = NodeLines {
            anchors: vec![pair(0, 0), pair(5, 6)],
            old: vec![Kept, Moved(5), Blank, Kept, Kept, Kept],
            new: vec![Kept, Unmatched, Blank, Kept, Moved(1), Kept, Kept],
        };
        let alignment = align_by_nodes(&old, &new, &lines);
        assert_eq!(
            alignment.rows,
            vec![
                RowLines::pair(0, 0),
                RowLines::old(1),
                RowLines::new(1),
                RowLines::pair(2, 2),
                RowLines::pair(3, 3),
                RowLines::new(4),
                RowLines::pair(4, 5),
                RowLines::pair(5, 6),
            ]
        );
        assert_eq!(alignment.old_counterpart[1], Some(5));
        assert_eq!(alignment.new_counterpart[4], Some(1));
        assert_eq!(
            alignment.new_counterpart[1], None,
            "inserted, no counterpart"
        );

        // Crossing anchors: the chain keeps the better-supported one.
        let lines = NodeLines {
            anchors: vec![
                pair(0, 1),
                LinePair {
                    old: 1,
                    new: 0,
                    support: 3,
                },
            ],
            old: vec![Kept, Kept],
            new: vec![Kept, Kept],
        };
        assert_eq!(
            align_by_nodes(&["p", "q"], &["q", "p"], &lines).rows,
            vec![RowLines::old(0), RowLines::pair(1, 0), RowLines::new(1)]
        );
    }

    #[test]
    fn rows_by_patch_pair_the_nth_deleted_line_with_the_nth_added() {
        // Old: a b c d; new: a B C X d (b, c -> B, C, X).
        let alignment = rows_by_patch(4, 5, |l| l == 1 || l == 2, |l| (1..=3).contains(&l));
        assert_eq!(
            alignment.rows,
            vec![
                RowLines::pair(0, 0),
                RowLines::pair(1, 1),
                RowLines::pair(2, 2),
                RowLines::new(3),
                RowLines::pair(3, 4),
            ]
        );
        // A pure deletion at the end, and an empty new file.
        assert_eq!(
            rows_by_patch(2, 1, |l| l == 1, |_| false).rows,
            vec![RowLines::pair(0, 0), RowLines::old(1)]
        );
        assert_eq!(
            rows_by_patch(2, 0, |_| true, |_| false).rows,
            vec![RowLines::old(0), RowLines::old(1)]
        );
    }

    #[test]
    fn line_pairs_read_matched_ranges_of_both_sides_and_skip_anchors() {
        let range = |operation, source: TextRange, destination: TextRange| RangeMatch {
            source,
            destination,
            operation,
        };
        let before = [
            // Rows 0-1 are new rows 4-5; the end at column 0 of row 2 does not touch it.
            range(
                TextOperation::Identical,
                TextRange::new(0, 0, 2, 0),
                TextRange::new(4, 0, 6, 0),
            ),
            // An update's zero-width destination still names its row.
            range(
                TextOperation::Update,
                TextRange::new(3, 2, 3, 5),
                TextRange::new(7, 1, 7, 1),
            ),
            // A deletion's destination is an anchor.
            range(
                TextOperation::Delete,
                TextRange::new(2, 0, 3, 0),
                TextRange::new(6, 0, 6, 0),
            ),
        ];
        let after = [range(
            TextOperation::Identical,
            TextRange::new(5, 0, 5, 3),
            TextRange::new(1, 0, 1, 3),
        )];
        let pairs = line_pairs(&before, &after, 10, 10);
        assert_eq!(
            pairs,
            vec![
                pair(0, 4),
                LinePair {
                    old: 1,
                    new: 5,
                    support: 2
                },
                pair(3, 7),
            ]
        );
    }
}
