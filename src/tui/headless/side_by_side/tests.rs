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

use std::path::PathBuf;

use super::*;
use crate::diff::text::{RenderOptions, ranges_for_options};
use crate::tui::headless::alignment::NodeLines;
use crate::tui::headless::{
    CONTEXT_LINES, Pair, render_annotated_text_diff, render_laid_out_text_diff,
};

/// A diff as `headless::run` computes it for a side-by-side layout: the session data, and what the
/// node mapping says of the lines when the pair has an AST.
struct Diffed {
    data: DiffSessionData,
    lines: Option<NodeLines>,
}

impl std::ops::Deref for Diffed {
    type Target = DiffSessionData;

    fn deref(&self) -> &DiffSessionData {
        &self.data
    }
}

/// The diff of `before` and `after` as `headless::run` paints it under `options`, its sides named
/// `name`.
fn diff_of(before: &str, after: &str, name: &str, options: RenderOptions) -> Diffed {
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = (dir.path().join(format!("a-{name}")), dir.path().join(name));
    std::fs::write(&old, before).unwrap();
    std::fs::write(&new, after).unwrap();
    let (mut data, _, lines) = Pair::read(&old, &new)
        .unwrap()
        .diff_and_node_lines(options, true)
        .unwrap();
    data.before_path = PathBuf::from(name);
    data.after_path = PathBuf::from(name);
    data.before_ranges = ranges_for_options(&data.before_ranges, &data.before_contents, options);
    data.after_ranges = ranges_for_options(&data.after_ranges, &data.after_contents, options);
    Diffed { data, lines }
}

fn laid_out_with(diffed: &Diffed, layout: Layout, width: Option<usize>, context: usize) -> String {
    render_laid_out_text_diff(
        diffed,
        false,
        context,
        None,
        LayoutRequest { layout, width },
        None,
        diffed.lines.as_ref(),
    )
}

fn laid_out(diffed: &Diffed, layout: Layout, width: Option<usize>) -> String {
    laid_out_with(diffed, layout, width, CONTEXT_LINES)
}

/// The rows after the header row, trailing blanks trimmed.
fn body(text: &str) -> Vec<String> {
    after_header(text)
        .map(|row| row.trim_end().to_string())
        .collect()
}

/// The rows after the header row (after the summary label, if any).
fn after_header(text: &str) -> impl Iterator<Item = &str> {
    text.lines().skip_while(|row| !row.contains('┬')).skip(1)
}

/// The display column of the first `needle` in `row`.
fn column_of(row: &str, needle: char) -> Option<usize> {
    Some(display_columns::display_width(&row[..row.find(needle)?]))
}

const MOVE_BEFORE: &str = "def total(numbers):\n    s = 0\n    for n in numbers:\n        s += n\n    return s\n\n\ndef mean(numbers):\n    return total(numbers) / len(numbers)\n";
const MOVE_AFTER: &str = "def mean(numbers):\n    return sum(numbers) / len(numbers)\n\n\ndef total(numbers):\n    return sum(numbers)\n";

/// The design's first example: `def mean` keeps its rows because the node mapping holds more
/// pairs inside it (25, its `total` -> `sum` an update) than inside `def total` (12: the header
/// and the `return`, its loop deleted); the moved `total` names its other copy's lines.
#[test]
fn a_moved_function_sits_alone_and_names_its_other_copy() {
    let data = diff_of(MOVE_BEFORE, MOVE_AFTER, "move.py", RenderOptions::MINIMAL);
    let text = laid_out(&data, Layout::SideBySide, Some(96));
    let header = text.lines().next().unwrap();
    assert!(
        header.starts_with("── before: move.py ──") && header.contains("┬─ after: move.py ──"),
        "{header}"
    );
    assert_eq!(display_columns::display_width(header), 96);
    assert_eq!(
        column_of(header, '┬'),
        column_of(text.lines().nth(1).unwrap(), '│')
    );
    let expected = [
        "1    def total(numbers):                    → 5│",
        "2  -     s = 0                                 │",
        "3  -     for n in numbers:                     │",
        "4  -         s += n                            │",
        "5        return s                           → 6│",
        "6                                              │",
        "7                                              │",
        "8    def mean(numbers):                        │ 1    def mean(numbers):",
        "9  ~     return total(numbers) / len(numbers)  │ 2  ~     return sum(numbers) / len(numbers)",
        "                                               │ 3",
        "                                               │ 4",
        "                                               │ 5    def total(numbers):                    ← 1",
        "                                               │ 6  ~     return sum(numbers)                ← 5",
    ];
    assert_eq!(body(&text), expected, "\n{text}");
}

/// Under Full the painting differs (`return s` is `~`, the long line wraps at this width), the rows
/// do not: they come from the mapping, which the render options never touch.
#[test]
fn the_moved_function_keeps_the_same_rows_under_full() {
    let data = diff_of(MOVE_BEFORE, MOVE_AFTER, "move.py", RenderOptions::FULL);
    let expected = [
        "1    def total(numbers):                 → 5│",
        "2  -     s = 0                              │",
        "3  -     for n in numbers:                  │",
        "4  -         s += n                         │",
        "5  ~     return s                        → 6│",
        "6                                           │",
        "7                                           │",
        "8    def mean(numbers):                     │ 1    def mean(numbers):",
        "9  ~     return total(numbers) / len(numbers│ 2  ~     return sum(numbers) / len(numbers)",
        "     )                                      │",
        "                                            │ 3",
        "                                            │ 4",
        "                                            │ 5    def total(numbers):                 ← 1",
        "                                            │ 6  ~     return sum(numbers)             ← 5",
    ];
    let text = laid_out(&data, Layout::SideBySide, Some(90));
    assert_eq!(body(&text), expected, "\n{text}");
}

const REINDENT_BEFORE: &str = "def f(x):\n    if x:\n        return 1\n    return 0\n";
const REINDENT_AFTER: &str =
    "def f(x):\n    if x:\n        if x > 1:\n            return 1\n    return 0\n";

/// The design's second example: a nested `if` added around a line that is then the same line.
#[test]
fn a_line_reindented_into_a_new_block_stays_beside_itself() {
    let data = diff_of(REINDENT_BEFORE, REINDENT_AFTER, "f.py", RenderOptions::FULL);
    let text = laid_out(&data, Layout::SideBySide, Some(80));
    let expected = [
        "1    def f(x):                         │ 1    def f(x):",
        "2        if x:                         │ 2        if x:",
        "                                       │ 3  +         if x > 1:",
        "3 |          return 1                  │ 4 |              return 1",
        "4        return 0                      │ 5        return 0",
    ];
    assert_eq!(body(&text), expected, "\n{text}");
}

#[test]
fn the_width_comes_from_the_first_source_that_knows_it() {
    let never = || -> Option<usize> { panic!("asked although an earlier source knew") };
    let columns = |text: &str| Some(OsStr::new(text).to_os_string());
    assert_eq!(
        resolve_width(Some(100), columns("80").as_deref(), never, never),
        Some(100)
    );
    assert_eq!(
        resolve_width(None, columns(" 80 ").as_deref(), never, never),
        Some(80)
    );
    assert_eq!(
        resolve_width(None, columns("wide").as_deref(), || Some(120), never),
        Some(120),
        "a COLUMNS that is no number is skipped"
    );
    assert_eq!(
        resolve_width(None, None, || None, || Some(132)),
        Some(132),
        "git's pager: stdout a pipe, the terminal still there"
    );
    assert_eq!(
        resolve_width(Some(0), columns("0").as_deref(), || Some(0), || Some(90)),
        Some(90),
        "zero is unknown at every step"
    );
    assert_eq!(resolve_width(None, None, || None, || None), None);
}

#[test]
fn wrapping_never_splits_a_wide_character_and_keeps_combining_marks_with_their_base() {
    let piece = |text: &str| Piece {
        text: text.to_string(),
        color: Some("33"),
    };
    let rows = |text: &str, width: usize| -> Vec<(String, usize)> {
        wrap(&[piece(text)], width)
            .into_iter()
            .map(|(pieces, used)| (pieces.into_iter().map(|p| p.text).collect(), used))
            .collect()
    };
    assert_eq!(
        rows("a漢字", 2),
        vec![("a".into(), 1), ("漢".into(), 2), ("字".into(), 2)],
        "漢 would straddle the edge after `a`: it moves on and leaves a gap"
    );
    assert_eq!(rows("ab😀c", 3), vec![("ab".into(), 2), ("😀c".into(), 3)]);
    assert_eq!(
        rows("abe\u{301}cd", 3),
        vec![("abe\u{301}".into(), 3), ("cd".into(), 2)]
    );
    assert_eq!(rows("abc", 3), vec![("abc".into(), 3)], "exactly the width");
    assert_eq!(rows("abcd", 3), vec![("abc".into(), 3), ("d".into(), 1)]);
    assert_eq!(rows("", 3), vec![(String::new(), 0)]);
    // Every piece keeps its color on every row it reaches.
    let wrapped = wrap(&[piece("abcd"), piece("ef")], 3);
    assert!(
        wrapped
            .iter()
            .all(|(pieces, _)| pieces.iter().all(|p| p.color == Some("33")))
    );
}

/// The display column at which each row's divider sits, for rows that have one.
fn divider_columns(text: &str) -> Vec<usize> {
    after_header(text)
        .filter_map(|row| column_of(row, '│'))
        .collect()
}

#[test]
fn wide_characters_tabs_and_long_lines_keep_the_divider_straight() {
    let before = "fn a() {\n\tlet s = \"漢字漢字漢字漢字漢字漢字漢字漢字漢字漢字漢字漢字\";\n}\n";
    let after =
        "fn a() {\n\tlet s = \"漢字漢字漢字漢字漢字漢字漢字漢字漢字😀漢字漢字e\u{301}\";\n}\n";
    let data = diff_of(before, after, "w.rs", RenderOptions::FULL);
    for width in [60, 61, 80] {
        let text = laid_out(&data, Layout::SideBySide, Some(width));
        let geometry = Geometry::new(width, 3, 3);
        let columns = divider_columns(&text);
        assert!(columns.len() > 3, "the long line wraps: {text}");
        assert!(
            columns.iter().all(|&c| c == geometry.left),
            "{width}:\n{text}"
        );
        assert!(!text.contains('\t'), "{text}");
        assert!(
            text.contains("│ 2  ~     let s"),
            "the tab is four columns: {text}"
        );
        for row in text.lines() {
            assert!(
                display_columns::display_width(row) <= width,
                "{width}: {row:?} is wider"
            );
        }
    }
}

#[test]
fn colors_never_cross_the_divider() {
    let data = diff_of(
        "x = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n",
        "x = \"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\"\n",
        "c.py",
        RenderOptions::FULL,
    );
    let text = render_laid_out_text_diff(
        &data,
        true,
        CONTEXT_LINES,
        None,
        LayoutRequest {
            layout: Layout::SideBySide,
            width: Some(60),
        },
        None,
        data.lines.as_ref(),
    );
    let rows: Vec<&str> = text.lines().skip(1).collect();
    assert!(rows.len() >= 2, "{text}");
    for row in rows {
        let (left, right) = row.split_once("\u{1b}[90m│\u{1b}[0m").expect("a divider");
        for half in [left, right] {
            // Every SGR opened in a half is reset in it.
            let opened = half.matches("\u{1b}[3").count() + half.matches("\u{1b}[90m").count();
            assert_eq!(opened, half.matches("\u{1b}[0m").count(), "{half:?}");
        }
        // The continuation rows of the painted strings are painted again.
        assert!(
            right.contains("\u{1b}[33m") || right.contains("\u{1b}[32m"),
            "{row:?}"
        );
    }
}

#[test]
fn a_line_exactly_the_column_wide_does_not_wrap_and_one_more_does() {
    let geometry = Geometry::new(70, 1, 1);
    let code = geometry.old_code();
    let exact = "x".repeat(code);
    let data = diff_of(
        &format!("{exact}\n"),
        &format!("{exact}y\n"),
        "e.txt",
        RenderOptions::FULL,
    );
    let rows = body(&laid_out(&data, Layout::SideBySide, Some(70)));
    assert_eq!(rows.len(), 2, "{rows:#?}");
    assert!(
        rows[0].starts_with(&format!("1    {exact}│ 1  ~ x")),
        "{rows:#?}"
    );
    assert!(rows[1].ends_with("│      y"), "{rows:#?}");
}

#[test]
fn an_unknown_or_too_narrow_width_is_stacked() {
    let data = diff_of(MOVE_BEFORE, MOVE_AFTER, "move.py", RenderOptions::FULL);
    let stacked = render_annotated_text_diff(&data, false, CONTEXT_LINES, None);
    // 9 and 6 lines: one digit a side, so code is (width - 2) / 2 - 5 on the left.
    let narrowest = 2 * (MIN_CODE_COLUMNS + 5) + 2;
    for (layout, width) in [
        (Layout::SideBySide, None),
        (Layout::SideBySide, Some(0)),
        (Layout::SideBySide, Some(narrowest - 1)),
        (Layout::Auto, None),
        (Layout::Auto, Some(2 * (AUTO_MIN_CODE_COLUMNS + 5) + 1)),
        (Layout::Stacked, Some(200)),
    ] {
        assert_eq!(
            laid_out(&data, layout, width),
            stacked,
            "{layout:?} at {width:?}"
        );
    }
    let side_by_side = laid_out(&data, Layout::SideBySide, Some(narrowest));
    assert_ne!(side_by_side, stacked);
    assert!(side_by_side.contains('┬'));
    // At the narrowest width auto allows, every row fits: the longest line is exactly the column,
    // and the lines naming a counterpart are short enough for their `→ N`.
    assert!(
        laid_out(
            &data,
            Layout::Auto,
            Some(2 * (AUTO_MIN_CODE_COLUMNS + 5) + 2)
        )
        .contains('┬')
    );
    assert!(laid_out(&data, Layout::Auto, Some(120)).contains('┬'));
}

#[test]
fn auto_stays_stacked_when_too_many_shown_lines_would_wrap() {
    let long = "y".repeat(60);
    let before: String = (0..10).map(|i| format!("x{i} = '{long}'\n")).collect();
    let after: String = (0..10).map(|i| format!("x{i} = '{long}z'\n")).collect();
    let data = diff_of(&before, &after, "l.py", RenderOptions::FULL);
    assert!(!laid_out(&data, Layout::Auto, Some(120)).contains('┬'));
    assert!(laid_out(&data, Layout::SideBySide, Some(120)).contains('┬'));
    assert!(laid_out(&data, Layout::Auto, Some(200)).contains('┬'));
}

/// The left and the right line number of each row, `None` for a blank half or a continuation.
fn numbers(text: &str, geometry: &Geometry) -> Vec<(Option<usize>, Option<usize>)> {
    let number = |gutter: &[char]| -> Option<usize> {
        gutter.iter().collect::<String>().trim().parse().ok()
    };
    after_header(text)
        .filter(|row| !row.starts_with("      ... "))
        .filter_map(|row| {
            let chars: Vec<char> = row.chars().collect();
            let mut column = 0;
            let at = chars.iter().position(|&ch| {
                let here = column;
                column += cell_width_of(ch).get();
                here == geometry.left
            })?;
            if chars[at] != '│' {
                return None;
            }
            let right_gutter = &chars[(at + 2).min(chars.len())..];
            Some((
                number(&chars[..geometry.old_number.min(at)]),
                number(&right_gutter[..geometry.new_number.min(right_gutter.len())]),
            ))
        })
        .collect()
}

/// A fixture's before and after texts and its file extension.
fn fixture(name: &str) -> (String, String, String) {
    let dir = crate::test::helper::diffs_case_dir(name).unwrap();
    let side = |prefix: &str| {
        let entry = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .find(|file| file.starts_with(prefix) && file.ends_with(".test"))
            .unwrap();
        let extension = entry
            .trim_start_matches(prefix)
            .trim_end_matches(".test")
            .to_string();
        (
            std::fs::read_to_string(dir.join(&entry)).unwrap(),
            extension,
        )
    };
    let ((before, extension), (after, _)) = (side("before"), side("after"));
    (before, after, extension)
}

/// The rows of `before` -> `after` named `name` under `options`, and the line each one-sided row
/// names.
fn rows_under(
    before: &str,
    after: &str,
    name: &str,
    options: RenderOptions,
) -> alignment::Alignment {
    let data = diff_of(before, after, name, options);
    let (old_lines, new_lines) = (osc1717::line_count(before), osc1717::line_count(after));
    let old_side = SideLines::new(&data.before_contents, &data.before_ranges, None);
    let new_side = SideLines::new(&data.after_contents, &data.after_ranges, None);
    aligned_rows(
        &data,
        (&old_side, &new_side),
        (old_lines, new_lines),
        (None, data.lines.as_ref()),
        alignment::PAIR_CAP,
    )
    .unwrap()
}

/// The `M` panel's options change the painting and never the rows: the rows come from the node
/// mapping. Both design examples and a sample of the fixtures.
#[test]
fn the_render_options_never_change_the_rows() {
    let examples = [
        (
            MOVE_BEFORE.to_string(),
            MOVE_AFTER.to_string(),
            ".py".to_string(),
        ),
        (
            REINDENT_BEFORE.to_string(),
            REINDENT_AFTER.to_string(),
            ".py".to_string(),
        ),
    ];
    let fixtures = crate::test::helper::UNIT_TEST_FIXTURES
        .iter()
        .step_by(5)
        .map(|name| fixture(name));
    let mut with_ast = 0;
    for (index, (before, after, extension)) in examples.into_iter().chain(fixtures).enumerate() {
        let name = format!("f{extension}");
        let full = rows_under(&before, &after, &name, RenderOptions::FULL);
        let minimal = rows_under(&before, &after, &name, RenderOptions::MINIMAL);
        assert_eq!(full, minimal, "case {index} ({extension})");
        with_ast += usize::from(
            diff_of(&before, &after, &name, RenderOptions::FULL)
                .lines
                .is_some(),
        );
    }
    assert!(with_ast > 2, "the sample reaches the node mapping");
}

/// Over a sample of the fixtures, with no folding: every line of each side on exactly one row,
/// top to bottom, and every divider in its column.
#[test]
fn every_line_of_a_fixture_is_on_one_row_in_order() {
    for name in crate::test::helper::UNIT_TEST_FIXTURES.iter().step_by(7) {
        let (before, after, extension) = fixture(name);
        let data = diff_of(
            &before,
            &after,
            &format!("f{extension}"),
            RenderOptions::FULL,
        );
        let (old_lines, new_lines) = (osc1717::line_count(&before), osc1717::line_count(&after));
        let width = 160;
        let text = laid_out_with(&data, Layout::SideBySide, Some(width), usize::MAX / 2);
        let geometry = Geometry::new(width, old_lines, new_lines);
        if !data
            .before_ranges
            .iter()
            .chain(&data.after_ranges)
            .any(|range| !matches!(range.operation, crate::diff::text::TextOperation::Identical))
            || text.contains("      ... ")
        {
            // Nothing painted: one fold row says how many lines each side has.
            assert!(
                text.contains(&fold_text(old_lines, new_lines)),
                "{name}\n{text}"
            );
            continue;
        }
        let rows = numbers(&text, &geometry);
        let left: Vec<usize> = rows.iter().filter_map(|row| row.0).collect();
        let right: Vec<usize> = rows.iter().filter_map(|row| row.1).collect();
        assert_eq!(left, (1..=old_lines).collect::<Vec<_>>(), "{name}\n{text}");
        assert_eq!(right, (1..=new_lines).collect::<Vec<_>>(), "{name}\n{text}");
        assert!(
            after_header(&text)
                .all(|row| row.starts_with("      ... ")
                    || display_columns::display_width(row) <= width),
            "{name}"
        );
    }
}

/// Folding counts rows; a run whose sides differ in length says both.
#[test]
fn folded_rows_name_each_sides_count_when_they_differ() {
    let lines = |range: std::ops::Range<usize>| -> String {
        range.map(|i| format!("line {i}\n")).collect()
    };
    let before = format!("{}\n\n{}", lines(0..20), lines(20..40));
    let after = format!("{}{}changed\n", lines(0..20), lines(20..39));
    let data = diff_of(&before, &after, "fold.txt", RenderOptions::FULL);
    let text = laid_out(&data, Layout::SideBySide, Some(100));
    assert!(
        text.contains("... 20 unchanged lines ...") || text.contains("unchanged lines before,"),
        "{text}"
    );
    let rows = body(&text);
    assert!(
        rows.len() < 20,
        "40 lines with changes in two places fold: {text}"
    );
    assert_eq!(fold_text(3, 3), "      ... 3 unchanged lines ...");
    assert_eq!(
        fold_text(1, 2),
        "      ... 1 unchanged line before, 2 after ..."
    );
}

#[test]
fn a_long_path_is_cut_to_its_end_in_the_header() {
    let geometry = Geometry::new(60, 9, 9);
    let header = header_row(
        "a/very/long/directory/name/that/goes/on/before.rs",
        "b.rs",
        &geometry,
    );
    assert_eq!(display_columns::display_width(&header), 60, "{header}");
    assert!(header.contains("…"), "{header}");
    assert!(header.contains("before.rs ─┬"), "{header}");
    assert_eq!(column_of(&header, '┬'), Some(geometry.left));
}

/// Above the cap the rows follow git's patch: the reindented `return 1` is a deletion and an
/// addition there, so it no longer sits beside itself; without a patch there are no rows at all
/// (stacked).
#[test]
fn above_the_cap_the_rows_follow_gits_patch() {
    let (before, after) = (
        "def f(x):\n    if x:\n        return 1\n    return 0\n",
        "def f(x):\n    if x:\n        if x > 1:\n            return 1\n    return 0\n",
    );
    let data = diff_of(before, after, "f.py", RenderOptions::FULL);
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = (dir.path().join("a.py"), dir.path().join("b.py"));
    std::fs::write(&old, before).unwrap();
    std::fs::write(&new, after).unwrap();
    let Some(patch) = Patch::start(&old, &new).finish() else {
        eprintln!("no git on PATH; skipped");
        return;
    };
    let old_side = SideLines::new(&data.before_contents, &data.before_ranges, None);
    let new_side = SideLines::new(&data.after_contents, &data.after_ranges, None);
    let rows = |cap: usize, patch: Option<&Patch>| {
        aligned_rows(&data, (&old_side, &new_side), (4, 5), (patch, None), cap).map(|a| a.rows)
    };
    assert_eq!(
        rows(alignment::PAIR_CAP, None).unwrap()[2..4],
        [RowLines::new(2), RowLines::pair(2, 3)],
        "under the cap, omnidiff's matching"
    );
    assert_eq!(
        rows(0, Some(&patch)).unwrap(),
        [
            RowLines::pair(0, 0),
            RowLines::pair(1, 1),
            RowLines::pair(2, 2),
            RowLines::new(3),
            RowLines::pair(3, 4),
        ]
    );
    assert_eq!(rows(0, None), None);
}
