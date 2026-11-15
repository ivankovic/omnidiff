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

//! The ranges [`RenderOptions::changed_trailing_whitespace`] paints: trailing whitespace that
//! changed, the one whitespace neither the tree nor the token diff sees.
//!
//! "Changed" is decided row by row, not token by token. The rows of both files are lined up by a
//! line diff over their contents with trailing whitespace cut off, so a row that differs from its
//! partner only after its last visible character is one row, kept. A kept row whose trailing run
//! differs from its partner's paints its own run whole, `Delete` on the before side and `Insert`
//! on the after side, the way the `Maximal` paintings mark a commit stripping trailing spaces
//! (`ruby-mikel-mail-whitespace-only-change`, `python-paddlepaddle-paddleocr-whitespace-only`).
//! Whole runs rather than the bytes one run has and the other lacks: two spaces turned into a tab
//! share no byte, and the paintings paint the spacing that changed, not its arithmetic
//! (invariant 30 reads spacing the same way).
//!
//! Rows that have no partner are new or gone lines, and their trailing whitespace goes with the
//! line: the `Maximal` paintings stop such a line at its last visible character (13 rows in 10
//! fixtures, none the other way), and leave a blank line of indentation added or removed on its
//! own unpainted (`rust-algorithm-change`, `java-defects4j-cli-23-helpformatter`), as invariant 29
//! leaves line breaks out. The one painting that marks such a row is a whole file of whitespace
//! edits (`ruby-mikel-mail-whitespace-only-change`, 4 bytes).
//!
//! A blank row lines up with any other blank row, so a lined-up blank row counts only between
//! two lined-up neighbours: at the edge of an inserted block, which blank row is kept and which is
//! new is the line diff's arbitrary choice (`java-defects4j-cli-23-helpformatter`).
//!
//! An edited row (`foo(a)  ` -> `foo(b)`) has no partner either, so its trailing whitespace stays
//! unpainted even when it changed: telling an edited row from a replaced one needs the row
//! similarity the line diff does not have.
//!
//! Every range is one row and ends at the row's end, before a `\r` of a CRLF break, which is the
//! line break and not whitespace on the row (invariant 29 leaves line breaks out the same way).
//! That shape is how [`super::render_options::ranges_for_options`] recognises them: under the
//! option it keeps an `Insert`/`Delete` that is exactly a row's trailing run instead of trimming it
//! away, and `MINIMAL` never builds them. So the ranges they join first lose any range of that
//! shape the walk built ([`without_trailing_runs`]), which every option trimmed to nothing before
//! (`java-defects4j-cli-23-helpformatter` has a 12-byte one).
//!
//! [`RenderOptions::changed_trailing_whitespace`]: super::RenderOptions::changed_trailing_whitespace

use crate::diff::text_range::TextRange;

use super::plain_text_diff::{PLAIN_TEXT_MAX_EDIT, hash_lines};
use super::render_options::{RangeMatch, TextOperation};

/// One row of a file without its line break: `split('\n')`, then a final `\r` dropped.
pub(crate) fn rows(text: &str) -> Vec<&str> {
    text.split('\n')
        .map(|row| row.strip_suffix('\r').unwrap_or(row))
        .collect()
}

/// Where `row`'s trailing whitespace starts.
pub(crate) fn content_end(row: &str) -> usize {
    row.trim_end().len()
}

/// Whether `range` is exactly one row's trailing whitespace run in `lines`: one row, from where
/// the row's visible text ends (or column 0 on a row with none) to the row's end.
pub(crate) fn is_trailing_run(lines: &[&str], range: &TextRange) -> bool {
    if range.start_row != range.end_row {
        return false;
    }
    let Some(row) = lines.get(range.start_row) else {
        return false;
    };
    let row = row.strip_suffix('\r').unwrap_or(row);
    range.start_column == content_end(row)
        && range.end_column == row.len()
        && range.start_column < range.end_column
}

/// The changed-trailing-whitespace ranges of both sides, `(before, after)`, each sorted by
/// position. Empty when the line diff gives up (`PLAIN_TEXT_MAX_EDIT`): then nothing can say which
/// rows correspond.
pub(crate) fn changed_trailing_whitespace(
    before: &str,
    after: &str,
) -> (Vec<RangeMatch>, Vec<RangeMatch>) {
    let sides = [rows(before), rows(after)];
    let contents: [Vec<&str>; 2] =
        [0, 1].map(|side| sides[side].iter().map(|row| row.trim_end()).collect());
    let Some(pairs) = crate::diff::apted::myers_lcs(
        &hash_lines(&contents[0]),
        &hash_lines(&contents[1]),
        PLAIN_TEXT_MAX_EDIT,
    ) else {
        return (Vec::new(), Vec::new());
    };

    let trailing = |side: usize, row: usize| -> TextRange {
        let line = sides[side][row];
        TextRange::new(row, content_end(line), row, line.len())
    };
    let operation = |side: usize| {
        if side == 0 {
            TextOperation::Delete
        } else {
            TextOperation::Insert
        }
    };
    let mut ranges: [Vec<RangeMatch>; 2] = [Vec::new(), Vec::new()];

    let kept: std::collections::HashSet<(usize, usize)> = pairs.iter().copied().collect();
    // A row with no visible text lines up with any other: beside an inserted or deleted block,
    // which blank row is the kept one is arbitrary (`java-defects4j-cli-23-helpformatter`). Such
    // a pair counts only between two kept neighbours, inside unchanged code; a missing neighbour
    // (the file's first or last row) counts as kept.
    let between_kept = |b: usize, a: usize| -> bool {
        let above = b == 0 || a == 0 || kept.contains(&(b - 1, a - 1));
        let below =
            b + 1 >= sides[0].len() || a + 1 >= sides[1].len() || kept.contains(&(b + 1, a + 1));
        above && below
    };
    for &(b, a) in &pairs {
        if contents[0][b].is_empty() && !between_kept(b, a) {
            continue;
        }
        let rows = [b, a];
        let runs = [0, 1].map(|side| {
            let line = sides[side][rows[side]];
            &line[content_end(line)..]
        });
        if runs[0] == runs[1] {
            continue;
        }
        for side in [0, 1] {
            if !runs[side].is_empty() {
                ranges[side].push(RangeMatch {
                    source: trailing(side, rows[side]),
                    destination: trailing(1 - side, rows[1 - side]),
                    operation: operation(side),
                });
            }
        }
    }

    for side in [0, 1] {
        ranges[side].sort_by_key(|range| (range.source.start_row, range.source.start_column));
    }
    let [before_ranges, after_ranges] = ranges;
    (before_ranges, after_ranges)
}

/// `ranges` without the `Insert`/`Delete` ranges that are exactly a row's trailing run, which
/// the walk can leave (a whitespace gap of an inserted subtree). Without the option the display
/// filter trims each of them to nothing; with it, it keeps every range of that shape, so only the
/// runs built here may have it.
pub(crate) fn without_trailing_runs(ranges: Vec<RangeMatch>, source: &str) -> Vec<RangeMatch> {
    let lines: Vec<&str> = source.split('\n').collect();
    ranges
        .into_iter()
        .filter(|range| {
            !(matches!(
                range.operation,
                TextOperation::Insert | TextOperation::Delete
            ) && is_trailing_run(&lines, &range.source))
        })
        .collect()
}

/// `extra` merged into `ranges`, the ranges of one side, so that no two claim the same byte (what
/// `corpus_ranges_are_addressable_byte_columns_sharing_nothing_but_line_terminators` holds every
/// side to). A run inside an `Identical`, `Insert` or `Delete` range cuts that range in two around
/// it: an `Identical` range paints nothing and an `Insert`/`Delete` destination is an anchor, so
/// neither half loses anything. A run inside a `Move` or `Update` is dropped: those are never cut,
/// since their destinations are real positions, and the row already shows as moved or changed.
///
/// Every range of `ranges` keeps its order; a run goes before the first range starting after it.
pub(crate) fn merged_by_position(
    ranges: Vec<RangeMatch>,
    extra: Vec<RangeMatch>,
) -> Vec<RangeMatch> {
    if extra.is_empty() {
        return ranges;
    }
    let start = |range: &TextRange| (range.start_row, range.start_column);
    let end = |range: &TextRange| (range.end_row, range.end_column);
    let overlaps = |a: &TextRange, b: &TextRange| start(a) < end(b) && start(b) < end(a);
    let extra: Vec<RangeMatch> = extra
        .into_iter()
        .filter(|run| {
            !ranges.iter().any(|range| {
                matches!(range.operation, TextOperation::Move | TextOperation::Update)
                    && overlaps(&range.source, &run.source)
            })
        })
        .collect();

    let mut result = Vec::with_capacity(ranges.len() + 2 * extra.len());
    let mut pending = extra.iter().peekable();
    for range in ranges {
        while pending
            .peek()
            .is_some_and(|next| start(&next.source) < start(&range.source))
        {
            result.extend(pending.next().cloned());
        }
        let inside: Vec<&RangeMatch> = extra
            .iter()
            .filter(|run| overlaps(&range.source, &run.source))
            .collect();
        if inside.is_empty() {
            result.push(range);
            continue;
        }
        // Cut around each run in turn, left to right; the pieces keep the range's destination.
        let mut from = start(&range.source);
        for run in inside {
            if from < start(&run.source) {
                result.push(piece(&range, from, start(&run.source)));
            }
            // The run itself between the two pieces, with any pending run before it.
            while let Some(next) = pending.next_if(|next| start(&next.source) <= start(&run.source))
            {
                result.push(next.clone());
            }
            from = from.max(end(&run.source));
        }
        if from < end(&range.source) {
            result.push(piece(&range, from, end(&range.source)));
        }
    }
    result.extend(pending.cloned());
    result
}

/// `range` narrowed to `from..to`.
fn piece(range: &RangeMatch, from: (usize, usize), to: (usize, usize)) -> RangeMatch {
    RangeMatch {
        source: TextRange::new(from.0, from.1, to.0, to.1),
        destination: range.destination.clone(),
        operation: range.operation.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(ranges: &[RangeMatch]) -> Vec<(TextOperation, (usize, usize, usize, usize))> {
        ranges
            .iter()
            .map(|range| {
                let s = &range.source;
                (
                    range.operation.clone(),
                    (s.start_row, s.start_column, s.end_row, s.end_column),
                )
            })
            .collect()
    }

    #[test]
    fn stripped_trailing_spaces_are_a_delete_of_the_whole_run() {
        let (before, after) = changed_trailing_whitespace("a = 1;   \nb\n", "a = 1;\nb\n");
        assert_eq!(spans(&before), vec![(TextOperation::Delete, (0, 6, 0, 9))]);
        assert!(after.is_empty(), "{after:?}");
    }

    #[test]
    fn spaces_turned_into_a_tab_paint_both_runs() {
        let (before, after) = changed_trailing_whitespace("x  \n", "x\t\n");
        assert_eq!(spans(&before), vec![(TextOperation::Delete, (0, 1, 0, 3))]);
        assert_eq!(spans(&after), vec![(TextOperation::Insert, (0, 1, 0, 2))]);
    }

    #[test]
    fn unchanged_trailing_whitespace_is_not_painted() {
        let (before, after) = changed_trailing_whitespace("x  \ny\n", "x  \nz\n");
        assert!(
            before.is_empty() && after.is_empty(),
            "{before:?} {after:?}"
        );
    }

    /// The `Maximal` paintings stop a new line at its last visible character.
    #[test]
    fn a_new_line_keeps_its_trailing_whitespace_unpainted() {
        let (before, after) = changed_trailing_whitespace("a\n", "a\nb  \n");
        assert!(
            before.is_empty() && after.is_empty(),
            "{before:?} {after:?}"
        );
    }

    /// `python-paddlepaddle-paddleocr-whitespace-only`: a blank row loses its spaces.
    #[test]
    fn a_blank_row_that_loses_its_spaces_is_painted() {
        let (before, after) = changed_trailing_whitespace("a\n    \nb\n", "a\n\nb\n");
        assert_eq!(spans(&before), vec![(TextOperation::Delete, (1, 0, 1, 4))]);
        assert!(after.is_empty());
    }

    /// A blank row at the edge of an inserted block lines up with an old blank row only by chance;
    /// its new indentation is the block's.
    #[test]
    fn a_blank_row_at_the_edge_of_an_inserted_block_is_left_alone() {
        let (before, after) =
            changed_trailing_whitespace("a\n\nz", "a\n    \n    if x {\n    }\nz");
        assert!(
            before.is_empty() && after.is_empty(),
            "{before:?} {after:?}"
        );
    }

    /// A blank line of indentation added between two kept lines is a new line, not a change to
    /// whitespace: the paintings leave it alone (`rust-algorithm-change`).
    #[test]
    fn an_added_blank_row_of_spaces_is_left_alone() {
        let (before, after) = changed_trailing_whitespace("a\nb\n", "a\n  \nb\n");
        assert!(
            before.is_empty() && after.is_empty(),
            "{before:?} {after:?}"
        );
    }

    #[test]
    fn a_crlf_break_is_not_trailing_whitespace() {
        let (before, after) = changed_trailing_whitespace("x\r\ny\n", "x\ny\n");
        assert!(
            before.is_empty() && after.is_empty(),
            "{before:?} {after:?}"
        );
        let (before, _) = changed_trailing_whitespace("x \r\n", "x\r\n");
        assert_eq!(spans(&before), vec![(TextOperation::Delete, (0, 1, 0, 2))]);
    }

    #[test]
    fn is_trailing_run_names_exactly_a_rows_trailing_whitespace() {
        let lines = ["a  ", "   ", "a b", "a \r"];
        assert!(is_trailing_run(&lines, &TextRange::new(0, 1, 0, 3)));
        assert!(
            !is_trailing_run(&lines, &TextRange::new(0, 2, 0, 3)),
            "part of the run"
        );
        assert!(is_trailing_run(&lines, &TextRange::new(1, 0, 1, 3)));
        assert!(
            !is_trailing_run(&lines, &TextRange::new(2, 1, 2, 2)),
            "a mid-row space"
        );
        assert!(
            is_trailing_run(&lines, &TextRange::new(3, 1, 3, 2)),
            "before the `\\r`"
        );
        assert!(
            !is_trailing_run(&lines, &TextRange::new(0, 1, 1, 0)),
            "two rows"
        );
    }

    fn at(row: usize, column: usize, end: usize, operation: TextOperation) -> RangeMatch {
        RangeMatch {
            source: TextRange::new(row, column, row, end),
            destination: TextRange::zero(),
            operation,
        }
    }

    #[test]
    fn merged_by_position_keeps_the_existing_order() {
        // Out of order on purpose: the merge must not sort what it was given.
        let ranges = vec![
            at(2, 0, 1, TextOperation::Insert),
            at(1, 0, 1, TextOperation::Insert),
        ];
        let extra = vec![
            at(0, 5, 6, TextOperation::Delete),
            at(3, 0, 1, TextOperation::Delete),
        ];
        let merged = merged_by_position(ranges, extra);
        assert_eq!(
            spans(&merged)
                .iter()
                .map(|(_, s)| (s.0, s.1))
                .collect::<Vec<_>>(),
            vec![(0, 5), (2, 0), (1, 0), (3, 0)]
        );
    }

    /// Only the runs built here may look like a trailing run: a whitespace piece the walk left in
    /// that shape goes, and a range that merely ends in trailing whitespace stays for the trim.
    #[test]
    fn without_trailing_runs_drops_only_the_exact_shape() {
        let source = "x = 1;   \n            \n";
        let ranges = vec![
            at(0, 6, 9, TextOperation::Insert),
            at(1, 0, 12, TextOperation::Insert),
            at(0, 0, 9, TextOperation::Insert),
            at(0, 6, 9, TextOperation::Move),
        ];
        assert_eq!(
            spans(&without_trailing_runs(ranges, source)),
            vec![
                (TextOperation::Insert, (0, 0, 0, 9)),
                (TextOperation::Move, (0, 6, 0, 9)),
            ]
        );
    }

    /// A whitespace-only file is one `Identical` range; each run cuts it, so no byte is claimed
    /// twice.
    #[test]
    fn a_run_cuts_the_identical_range_around_it() {
        let file = RangeMatch {
            source: TextRange::new(0, 0, 3, 0),
            destination: TextRange::new(0, 0, 3, 0),
            operation: TextOperation::Identical,
        };
        let merged = merged_by_position(vec![file], vec![at(1, 3, 5, TextOperation::Delete)]);
        assert_eq!(
            spans(&merged),
            vec![
                (TextOperation::Identical, (0, 0, 1, 3)),
                (TextOperation::Delete, (1, 3, 1, 5)),
                (TextOperation::Identical, (1, 5, 3, 0)),
            ]
        );
    }

    /// A moved row is never cut; it already shows as moved, so the run is left out.
    #[test]
    fn a_run_inside_a_move_is_dropped() {
        let moved = RangeMatch {
            source: TextRange::new(0, 0, 2, 0),
            destination: TextRange::new(5, 0, 7, 0),
            operation: TextOperation::Move,
        };
        let merged = merged_by_position(
            vec![moved.clone()],
            vec![at(0, 3, 5, TextOperation::Delete)],
        );
        assert_eq!(merged, vec![moved]);
    }
}
