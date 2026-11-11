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
//! **Rows follow omnidiff's own matching, not git's patch.** Every `Identical`, `Update` and `Move`
//! range says that the text it covers on one side *is* the text it names on the other, so each
//! line it touches is paired with the line its destination covers at the same offset (a range
//! whose destination has fewer rows, such as an `Update`'s zero-width anchor, clamps to its last
//! row). Both sides' lists are read, since a range can be recorded on one side only. `Insert` and
//! `Delete` carry a destination too, but it is an anchor, not a match, and pairs nothing. A pair
//! drawn from several ranges is the same pair; its `support` counts them.
//!
//! **One in-order chain.** A row can only hold lines that read top to bottom on both sides, so the
//! rows keep the best chain of pairs strictly increasing in both lines (a weighted longest
//! increasing subsequence, `O(P log P)` over a Fenwick tree of prefix maxima). Best is, in order:
//! the most *updated* lines (a line painted `~`, counted on each side, so a chain through a
//! rewritten function wins over one through a function moved intact - the rewrite is what a
//! reader compares across a row); then the most pairs; then the chain that ends earlier in the
//! old file, and among equals the predecessor earlier in it. Weighting by the painting makes the
//! choice follow the `M` panel's options: under `--full` a `return s` -> `return sum(numbers)`
//! line is `~` on both sides, under the saved default only on one.
//!
//! **Blank lines.** No range covers a line of only whitespace, so between two neighbouring chain
//! pairs the lines no range touches are paired by equal text (an LCS, skipped past
//! [`GAP_FILL_LIMIT`] cells), else an unchanged blank line would sit on a row of its own.
//!
//! **Everything else is one-sided**: the old lines between two rows of pairs first, then the new
//! ones, as git orders a change block. A one-sided line that does have a match elsewhere (a move,
//! or the losing side of a crossing) names it: [`Alignment::old_counterpart`] holds, per old line,
//! the new line its best-supported pair names (lowest line on a tie), and the reverse.
//!
//! **The cap.** Above [`PAIR_CAP`] pairs the chain is not computed and the rows follow git's patch
//! instead ([`rows_by_patch`]): context lines share a row, and in a change block the n-th deleted
//! line sits beside the n-th added one.

use std::cmp::Reverse;

use crate::diff::text::{RangeMatch, TextOperation};
use crate::diff::text_range::TextRange;

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

/// A chain's rank: updated lines, then pairs, then ending earlier in the old file (and then the
/// new). Larger is better.
type Score = (u64, u64, Reverse<usize>, Reverse<usize>);

/// A Fenwick tree of prefix maxima over new lines: the best chain ending before a new line.
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

/// The indices into `pairs` (sorted by old, then new line) of the best chain strictly increasing
/// in both lines, in order, where a pair's `updated` is how many of its two lines are updated (see
/// the module doc). `None` above `cap` pairs.
pub(crate) fn best_chain(
    pairs: &[LinePair],
    updated: impl Fn(&LinePair) -> u64,
    cap: usize,
) -> Option<Vec<usize>> {
    if pairs.len() > cap {
        return None;
    }
    let new_size = pairs.iter().map(|pair| pair.new + 1).max().unwrap_or(0);
    let mut prefix = PrefixMax::new(new_size);
    let mut previous: Vec<Option<usize>> = vec![None; pairs.len()];
    let mut best: Option<(Score, usize)> = None;
    let mut group_start = 0;
    while group_start < pairs.len() {
        // Pairs of one old line cannot chain to each other: all read the tree before any writes.
        let old = pairs[group_start].old;
        let group_end = group_start + pairs[group_start..].partition_point(|pair| pair.old == old);
        let scored: Vec<(Score, usize)> = (group_start..group_end)
            .map(|index| {
                let pair = &pairs[index];
                let before = prefix.best_before(pair.new);
                previous[index] = before.map(|(_, at)| at);
                let (updated_so_far, length) = before.map_or((0, 0), |((u, n, _, _), _)| (u, n));
                (
                    (
                        updated_so_far + updated(pair),
                        length + 1,
                        Reverse(pair.old),
                        Reverse(pair.new),
                    ),
                    index,
                )
            })
            .collect();
        for entry in scored {
            prefix.offer(pairs[entry.1].new, entry);
            best = best.max(Some(entry));
        }
        group_start = group_end;
    }
    let mut chain = Vec::new();
    let mut at = best.map(|(_, index)| index);
    while let Some(index) = at {
        chain.push(index);
        at = previous[index];
    }
    chain.reverse();
    Some(chain)
}

/// The pairs of equal lines, by a longest common subsequence of `old` and `new` (indices into
/// them); none when the table would pass [`GAP_FILL_LIMIT`] cells.
fn equal_lines(old: &[&str], new: &[&str]) -> Vec<(usize, usize)> {
    if old.is_empty() || new.is_empty() || old.len().saturating_mul(new.len()) > GAP_FILL_LIMIT {
        return Vec::new();
    }
    let width = new.len() + 1;
    // `table[i * width + j]`: the LCS of `old[i..]` and `new[j..]`.
    let mut table = vec![0u32; (old.len() + 1) * width];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            table[i * width + j] = if old[i] == new[j] {
                table[(i + 1) * width + j + 1] + 1
            } else {
                table[(i + 1) * width + j].max(table[i * width + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < old.len() && j < new.len() {
        if old[i] == new[j] {
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

/// The rows of a side-by-side layout of `old` and `new` (each side's lines) from omnidiff's `pairs`
/// (see the module doc), `updated` weighting the chain; `None` above `cap` pairs.
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

    let mut rows = Vec::with_capacity(old.len().max(new.len()));
    let (mut next_old, mut next_new) = (0, 0);
    let anchors = chain
        .iter()
        .map(|&index| (pairs[index].old, pairs[index].new))
        .chain(std::iter::once((old.len(), new.len())));
    for (anchor_old, anchor_new) in anchors {
        // The gap before the anchor: its unmatched lines pair by equal text.
        let free_old: Vec<usize> = (next_old..anchor_old)
            .filter(|&line| old_counterpart[line].is_none())
            .collect();
        let free_new: Vec<usize> = (next_new..anchor_new)
            .filter(|&line| new_counterpart[line].is_none())
            .collect();
        let old_texts: Vec<&str> = free_old.iter().map(|&line| old[line]).collect();
        let new_texts: Vec<&str> = free_new.iter().map(|&line| new[line]).collect();
        let fills = equal_lines(&old_texts, &new_texts);
        let one_sided = |rows: &mut Vec<RowLines>, from: (usize, usize), to: (usize, usize)| {
            rows.extend((from.0..to.0).map(RowLines::old));
            rows.extend((from.1..to.1).map(RowLines::new));
        };
        for (i, j) in fills {
            let (fill_old, fill_new) = (free_old[i], free_new[j]);
            one_sided(&mut rows, (next_old, next_new), (fill_old, fill_new));
            rows.push(RowLines::pair(fill_old, fill_new));
            (next_old, next_new) = (fill_old + 1, fill_new + 1);
        }
        one_sided(&mut rows, (next_old, next_new), (anchor_old, anchor_new));
        if anchor_old < old.len() {
            rows.push(RowLines::pair(anchor_old, anchor_new));
        }
        (next_old, next_new) = (anchor_old + 1, anchor_new + 1);
    }

    // Only a one-sided line names its counterpart.
    let mut old_named: Vec<Option<usize>> = vec![None; old.len()];
    let mut new_named: Vec<Option<usize>> = vec![None; new.len()];
    for row in &rows {
        match (row.old, row.new) {
            (Some(line), None) => old_named[line] = old_counterpart[line].map(|(_, other)| other),
            (None, Some(line)) => new_named[line] = new_counterpart[line].map(|(_, other)| other),
            _ => {}
        }
    }
    Some(Alignment {
        rows,
        old_counterpart: old_named,
        new_counterpart: new_named,
    })
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
