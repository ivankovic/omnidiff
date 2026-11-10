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

//! The spacing an edit inside a line brings along.
//!
//! `pub fn` -> `pub async fn` adds a word and a blank; `a` -> `a + b` adds two tokens and two
//! blanks. A node's range never holds the blank in front of it, so the walk alone paints `async`
//! and leaves a line that gained six characters with five highlighted. The rule here (ground-truth
//! invariant 30, `spaces_before_edits_follow_their_count`): **the blanks in front of an inserted or
//! deleted piece go with it exactly when the line has more blanks around the piece than the other
//! file has at that place**. `= foo` -> `= bar baz` paints ` baz`; `f(a)` -> `f(x, a)` paints `x,`
//! alone, because the one blank between `,` and `a` stands where no blank was needed before and
//! follows the piece instead of leading it.
//!
//! Both halves of the rule read the other file: [`widen_over_added_spacing`] finds the place
//! through the range's anchor, [`slide_to_leading_spacing`] through the unchanged text around a
//! changed middle. The widened range starts with its blanks, which
//! [`RenderOptions::leading_whitespace`](super::RenderOptions::leading_whitespace) keeps or trims
//! like the indentation of a whole inserted line, so the ranges are built the same way under every
//! option.
//!
//! Indentation is not this rule's: a piece that starts its row is left as the walk built it.

use super::{RangeMatch, TextOperation};
use crate::diff::text_range::{SourceColumn, SourceRow, SourceText, TextRange};

/// A blank inside a row. Line breaks are not spacing: this rule never crosses a row.
fn is_spacing(byte: u8) -> bool {
    byte == b' ' || byte == b'\t'
}

/// Whether `offset` is where a row's text ends: at a line break or at the end of the file.
fn is_row_end(text: &[u8], offset: usize) -> bool {
    matches!(text.get(offset), None | Some(b'\n' | b'\r'))
}

fn offset_of(text: &SourceText, row: usize, column: usize) -> Option<usize> {
    text.byte_index(SourceRow::from_raw(row), SourceColumn::from_raw(column))
        .map(|offset| offset.get())
}

fn start_offset(text: &SourceText, range: &TextRange) -> Option<usize> {
    offset_of(text, range.start_row, range.start_column)
}

fn end_offset(text: &SourceText, range: &TextRange) -> Option<usize> {
    offset_of(text, range.end_row, range.end_column)
}

/// Grows each in-line `Insert`/`Delete` of one side's walk over the blanks in front of it that the
/// edit added to the line (see the module doc).
///
/// A run of such ranges with only blanks between them is one edit: its first range is the one
/// that grows. The edit sits between the visible character before its leading blanks and the one
/// after its trailing blanks; its anchor names the same place in `destination`, and the rule
/// applies only where that is verifiably so - the anchor follows the same character and the same
/// character (or the row's end on both sides) comes next. There the edit takes
/// `ours - theirs` of its leading blanks, where `ours` counts the blanks before and after it and
/// `theirs` the blanks at the anchor. Everywhere else - a neighbour that moved, a replacement
/// whose other half sits at the anchor, a piece at the start of its row - nothing changes.
pub(super) fn widen_over_added_spacing(
    ranges: &mut [RangeMatch],
    source: &SourceText,
    destination: &SourceText,
) {
    let source_bytes = source.text().as_bytes();
    let destination_bytes = destination.text().as_bytes();
    let is_edit = |range: &RangeMatch| {
        matches!(
            range.operation,
            TextOperation::Insert | TextOperation::Delete
        ) && !range.source.is_empty()
    };

    let mut index = 0;
    while index < ranges.len() {
        let first = index;
        index += 1;
        if !is_edit(&ranges[first]) {
            continue;
        }
        let (Some(start), Some(mut end)) = (
            start_offset(source, &ranges[first].source),
            end_offset(source, &ranges[first].source),
        ) else {
            continue;
        };
        // The rest of the run: the same operation, with only blanks in between.
        while let Some(next) = ranges.get(index) {
            if !is_edit(next) || next.operation != ranges[first].operation {
                break;
            }
            let (Some(next_start), Some(next_end)) = (
                start_offset(source, &next.source),
                end_offset(source, &next.source),
            ) else {
                break;
            };
            if next_start < end
                || !source_bytes[end..next_start]
                    .iter()
                    .all(|&byte| is_spacing(byte))
            {
                break;
            }
            end = next_end;
            index += 1;
        }

        let leading = source_bytes[..start]
            .iter()
            .rev()
            .take_while(|&&byte| is_spacing(byte))
            .count();
        let left = start - leading;
        // No blank to take, the range already starts with one, or the blanks are indentation.
        if leading == 0
            || source_bytes.get(start).is_none_or(|&byte| is_spacing(byte))
            || left == 0
            || source_bytes[left - 1] == b'\n'
        {
            continue;
        }
        // A range through the end of its row ends at `(row + 1, 0)`, and one built from text may
        // end in blanks: the edit's last visible character is what the trailing blanks follow.
        while end > start && source_bytes[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        let trailing = source_bytes[end..]
            .iter()
            .take_while(|&&byte| is_spacing(byte))
            .count();
        let right = end + trailing;

        let Some(anchor) = start_offset(destination, &ranges[first].destination) else {
            continue;
        };
        if anchor == 0 || destination_bytes[anchor - 1] != source_bytes[left - 1] {
            continue;
        }
        let theirs = destination_bytes[anchor..]
            .iter()
            .take_while(|&&byte| is_spacing(byte))
            .count();
        let same_right_neighbour = match (
            is_row_end(source_bytes, right),
            is_row_end(destination_bytes, anchor + theirs),
        ) {
            (true, true) => true,
            (false, false) => source_bytes[right] == destination_bytes[anchor + theirs],
            _ => false,
        };
        if !same_right_neighbour {
            continue;
        }
        let taken = (leading + trailing).saturating_sub(theirs).min(leading);
        let grown_start = (
            ranges[first].source.start_row,
            ranges[first].source.start_column - taken,
        );
        // A text range before it that holds those blanks keeps them: ranges never overlap.
        let overlaps_previous = first > 0 && {
            let previous = &ranges[first - 1].source;
            !previous.is_empty() && (previous.end_row, previous.end_column) > grown_start
        };
        if !overlaps_previous {
            ranges[first].source.start_column = grown_start.1;
        }
    }
}

/// How far to slide the changed middle of a text pair left, so that a middle found as `word␠`
/// after a common prefix ending in a blank is read as `␠word` instead: the same edit, with the
/// blank it adds in front of it (see the module doc).
///
/// `text` is the side that has the middle, `prefix_len..middle_end` its extent; the other side
/// has nothing there, which is what makes every slide of one byte another exact account of the
/// edit. The result is how many bytes to take from the prefix and give to the suffix, on both
/// sides.
pub(super) fn slide_to_leading_spacing(text: &str, prefix_len: usize, middle_end: usize) -> usize {
    let bytes = text.as_bytes();
    let mut slid = 0;
    while slid < prefix_len
        && middle_end - slid > prefix_len - slid
        && is_spacing(bytes[middle_end - slid - 1])
        && bytes[prefix_len - slid - 1] == bytes[middle_end - slid - 1]
    {
        slid += 1;
    }
    slid
}
