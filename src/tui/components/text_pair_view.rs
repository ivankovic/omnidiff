/*  This file is part of the OmniDiff code diffing tool.
 *
 *  Copyright (C) 2026 Marko Ivankovic
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

//! Two texts that are not files on disk - an archive's member, a class file's listing - diffed by
//! OmniDiff and shown in a pane: the same diff and the same rendering as `--headless` (each side
//! numbered, unchanged runs elided, changed spans colored within the line, moved chunks boxed),
//! wrapped to the pane's width.
//!
//! Wrapping is done here, by screen row, because members are often one enormous line: an Office
//! document's XML is a single line of a hundred kilobytes, and scrolling by source line could not
//! move inside it. `J`/`K` jump to the next or previous row with a change on it, `PgDn`/`PgUp`
//! page.

use std::path::Path;

use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use unicode_width::UnicodeWidthChar;

use crate::diff::text::ranges_for_options;
use crate::tui::app::compute_diff_of_texts;
use crate::tui::headless::{CONTEXT_LINES, render_text_diff};

/// Texts larger than this are diffed by lines only: a pane is no place to wait for the AST diff of
/// a 20 MB file.
const SYNTAX_DIFF_BYTES: usize = 4 * 1024 * 1024;

pub struct TextPairView {
    /// The diff as headless renders it, one styled line per output line.
    lines: Vec<Line<'static>>,
    /// `lines` wrapped to a width: the rows, and which of them show a change.
    wrapped: Option<(u16, Vec<Line<'static>>, Vec<usize>)>,
    /// The first row shown.
    scroll: usize,
    /// How many rows the pane showed last, for paging.
    height: usize,
    /// What diffed it: the language OmniDiff detected, or why it fell back to lines.
    how: String,
    /// The two texts, for a caller that shows them elsewhere (the TUI's full diff view).
    before: String,
    after: String,
}

impl TextPairView {
    /// The diff of `before` and `after`, named `path` (whose extension picks the language, as a
    /// file's would).
    pub fn new(path: &Path, before: &str, after: &str) -> Self {
        let too_big = before.len().max(after.len()) > SYNTAX_DIFF_BYTES;
        let options = crate::tui::theme::load_render_options();
        let rendered = if too_big {
            None
        } else {
            compute_diff_of_texts(path, before, path, after, options)
                .ok()
                .map(|(mut data, _)| {
                    data.before_ranges =
                        ranges_for_options(&data.before_ranges, &data.before_contents, options);
                    data.after_ranges =
                        ranges_for_options(&data.after_ranges, &data.after_contents, options);
                    let how =
                        match crate::code::language::language_for_path_and_content(path, after) {
                            Some(language) if !data.plain_text_fallback => {
                                format!("diffed as {language}")
                            }
                            _ => "diffed by lines (no grammar)".to_string(),
                        };
                    (render_text_diff(&data, true, CONTEXT_LINES), how)
                })
        };
        let (text, how) = rendered.unwrap_or_else(|| {
            let why = if too_big {
                "too large to diff by syntax"
            } else {
                "the diff failed"
            };
            (
                plain_diff(before, after),
                format!("diffed by lines ({why})"),
            )
        });
        Self {
            lines: text.lines().map(|line| elide(ansi_line(line))).collect(),
            wrapped: None,
            scroll: 0,
            height: 1,
            how,
            before: before.to_string(),
            after: after.to_string(),
        }
    }

    /// The two texts diffed.
    pub fn texts(&self) -> (&str, &str) {
        (&self.before, &self.after)
    }

    /// One line for a status bar: how it was diffed, and where in it the view is.
    pub fn status(&self) -> String {
        let Some((_, rows, changes)) = &self.wrapped else {
            return self.how.clone();
        };
        let shown = changes
            .iter()
            .filter(|&&row| row < self.scroll + self.height)
            .count();
        format!(
            "{} · row {}/{} · change {shown}/{} (J/K next/previous change, PgDn/PgUp)",
            self.how,
            self.scroll + 1,
            rows.len(),
            changes.len()
        )
    }

    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        let Some((_, rows, changes)) = &self.wrapped else {
            return false;
        };
        let last = rows.len().saturating_sub(1);
        self.scroll = match code {
            // The change below the first row in view; the one above it.
            KeyCode::Char('J') => changes
                .iter()
                .copied()
                .find(|&row| row > self.scroll)
                .unwrap_or(self.scroll),
            KeyCode::Char('K') => changes
                .iter()
                .copied()
                .rev()
                .find(|&row| row < self.scroll)
                .unwrap_or(0),
            KeyCode::PageDown => (self.scroll + self.height.max(1)).min(last),
            KeyCode::PageUp => self.scroll.saturating_sub(self.height.max(1)),
            _ => return false,
        };
        true
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect, title: &str) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title.to_string());
        let inner = block.inner(area);
        frame.render_widget(block, area);
        if self.wrapped.as_ref().map(|(width, _, _)| *width) != Some(inner.width) {
            let (rows, changes) = wrap(&self.lines, usize::from(inner.width.max(1)));
            self.scroll = self.scroll.min(rows.len().saturating_sub(1));
            self.wrapped = Some((inner.width, rows, changes));
        }
        self.height = usize::from(inner.height);
        let (_, rows, _) = self.wrapped.as_ref().expect("wrapped above");
        let shown: Vec<Line> = rows
            .iter()
            .skip(self.scroll)
            .take(self.height)
            .cloned()
            .collect();
        frame.render_widget(Paragraph::new(shown), inner);
    }
}

/// A line of `render_text_diff`'s output as styled spans: its ANSI SGR codes (the handful headless
/// writes) become colors, everything else is text.
fn ansi_line(text: &str) -> Line<'static> {
    let mut spans = Vec::new();
    let mut style = Style::default();
    let mut rest = text;
    while let Some(start) = rest.find('\u{1b}') {
        if start > 0 {
            spans.push(Span::styled(rest[..start].to_string(), style));
        }
        let Some(end) = rest[start..].find('m') else {
            rest = &rest[start + 1..];
            continue;
        };
        for code in rest[start + 2..start + end].split(';') {
            style = match code {
                "0" | "" => Style::default(),
                "1" => style.add_modifier(Modifier::BOLD),
                "31" => style.fg(Color::Red),
                "32" => style.fg(Color::Green),
                "33" => style.fg(Color::Yellow),
                "90" => style.fg(Color::DarkGray),
                _ => style,
            };
        }
        rest = &rest[start + end + 1..];
    }
    if !rest.is_empty() {
        spans.push(Span::styled(rest.to_string(), style));
    }
    Line::from(spans)
}

/// Characters of unchanged text kept on each side of a change in a long line.
const KEPT_AROUND_CHANGES: usize = 120;

/// A line too long to read whole - an Office document's XML is one line - with each long run of
/// unchanged text cut to its ends: the changes, and [`KEPT_AROUND_CHANGES`] characters either side
/// of each, with "… N characters …" between.
fn elide(line: Line<'static>) -> Line<'static> {
    let length: usize = line
        .spans
        .iter()
        .map(|span| span.content.chars().count())
        .sum();
    if length < 8 * KEPT_AROUND_CHANGES {
        return line;
    }
    let count = line.spans.len();
    let mut spans = Vec::with_capacity(count);
    for (index, span) in line.spans.into_iter().enumerate() {
        let characters: Vec<char> = span.content.chars().collect();
        // A changed span is shown whole; the line's end needs no tail.
        let keep_head = KEPT_AROUND_CHANGES;
        let keep_tail = if index + 1 == count {
            0
        } else {
            KEPT_AROUND_CHANGES
        };
        if is_change(span.style) || characters.len() <= keep_head + keep_tail + 40 {
            spans.push(span);
            continue;
        }
        let head: String = characters[..keep_head].iter().collect();
        let tail: String = characters[characters.len() - keep_tail..].iter().collect();
        let cut = characters.len() - keep_head - keep_tail;
        spans.push(Span::styled(head, span.style));
        spans.push(Span::styled(
            format!(" … {cut} characters … "),
            Style::default().fg(Color::DarkGray),
        ));
        spans.push(Span::styled(tail, span.style));
    }
    Line::from(spans)
}

/// True if `style` marks a change: inserted, deleted or updated text (moves and chrome are grey).
fn is_change(style: Style) -> bool {
    matches!(style.fg, Some(Color::Red | Color::Green | Color::Yellow))
}

/// `lines` cut into rows of at most `width` columns, and the rows that show a change.
fn wrap(lines: &[Line<'static>], width: usize) -> (Vec<Line<'static>>, Vec<usize>) {
    let mut rows = Vec::new();
    let mut changes = Vec::new();
    for line in lines {
        let mut row: Vec<Span<'static>> = Vec::new();
        let (mut used, mut changed) = (0, false);
        let mut finish = |row: &mut Vec<Span<'static>>, changed: &mut bool| {
            if *changed {
                changes.push(rows.len());
            }
            rows.push(Line::from(std::mem::take(row)));
            *changed = false;
        };
        for span in &line.spans {
            let mut piece = String::new();
            for character in span.content.chars() {
                let cells = character.width().unwrap_or(0);
                if used + cells > width && used > 0 {
                    if !piece.is_empty() {
                        changed |= is_change(span.style);
                        row.push(Span::styled(std::mem::take(&mut piece), span.style));
                    }
                    finish(&mut row, &mut changed);
                    used = 0;
                }
                piece.push(character);
                used += cells;
            }
            if !piece.is_empty() {
                changed |= is_change(span.style);
                row.push(Span::styled(piece, span.style));
            }
        }
        finish(&mut row, &mut changed);
    }
    (rows, changes)
}

/// A plain line diff, rendered as `render_text_diff` renders one, for texts too large for the AST
/// diff: each side's lines, changed ones colored whole.
fn plain_diff(before: &str, after: &str) -> String {
    let (b, a): (Vec<&str>, Vec<&str>) = (before.lines().collect(), after.lines().collect());
    let pairs = crate::diff::text::line_diff_core(before, after, 10_000)
        .map(|core| core.pairs)
        .unwrap_or_default();
    let mut out = String::new();
    let mut side = |name: &str, lines: &[&str], kept: &[usize], color: &str| {
        out.push_str(&format!("=== {name} ===\n"));
        let mut kept = kept.iter().peekable();
        for (index, line) in lines.iter().enumerate() {
            if kept.peek() == Some(&&index) {
                kept.next();
                out.push_str(&format!("{:>6}   {line}\n", index + 1));
            } else {
                out.push_str(&format!(
                    "{:>6} \u{1b}[{color}m~ {line}\u{1b}[0m\n",
                    index + 1
                ));
            }
        }
    };
    let before_kept: Vec<usize> = pairs.iter().map(|(b, _)| *b).collect();
    let after_kept: Vec<usize> = pairs.iter().map(|(_, a)| *a).collect();
    side("before", &b, &before_kept, "31");
    side("after", &a, &after_kept, "32");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(rows: &[Line]) -> Vec<String> {
        rows.iter()
            .map(|row| row.spans.iter().map(|span| span.content.as_ref()).collect())
            .collect()
    }

    #[test]
    fn ansi_codes_become_styles() {
        let line = ansi_line("12 \u{1b}[31mgone\u{1b}[0m kept \u{1b}[1mbold\u{1b}[0m");
        assert_eq!(line.spans.len(), 4);
        assert_eq!(line.spans[1].content, "gone");
        assert_eq!(line.spans[1].style.fg, Some(Color::Red));
        assert!(line.spans[3].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn a_long_line_keeps_its_changes_and_cuts_the_rest() {
        let unchanged = "x".repeat(2000);
        // As headless writes it: the number grey, the marker yellow, then the text.
        let line = ansi_line(&format!(
            "\u{1b}[90m2 \u{1b}[0m \u{1b}[33m~\u{1b}[0m {unchanged}\u{1b}[33mNEW\u{1b}[0m{unchanged}"
        ));
        let elided = elide(line);
        let text: String = elided
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(text.starts_with("2  ~ "), "{text}");
        assert!(text.contains("NEW"));
        assert!(text.contains(" … 1761 characters … "), "{text}");
        assert!(text.chars().count() < 700, "{}", text.chars().count());
        let short = ansi_line("1    <a/>");
        assert_eq!(elide(short.clone()), short);
    }

    #[test]
    fn a_long_line_wraps_by_width_and_its_changed_rows_are_known() {
        let line = ansi_line("aaaaaaaaaa\u{1b}[32mbbb\u{1b}[0mcc");
        let (rows, changes) = wrap(&[line], 4);
        assert_eq!(text(&rows), vec!["aaaa", "aaaa", "aabb", "bcc"]);
        assert_eq!(changes, vec![2, 3]);
    }

    #[test]
    fn a_member_diffs_by_its_language_and_j_finds_the_change() {
        let before = "<a>\n<b x=\"1\"/>\n</a>\n";
        let after = "<a>\n<b x=\"2\"/>\n</a>\n";
        let mut view = TextPairView::new(Path::new("word/document.xml"), before, after);
        assert!(view.how.starts_with("diffed as"), "{}", view.how);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 8)).unwrap();
        terminal
            .draw(|frame| view.draw(frame, frame.area(), "document.xml"))
            .unwrap();
        let (_, rows, changes) = view.wrapped.clone().unwrap();
        assert!(!changes.is_empty(), "{:?}", text(&rows));
        assert!(text(&rows).iter().any(|row| row.contains("x=\"2\"")));
        assert!(view.handle_key(KeyCode::Char('J')));
        assert_eq!(
            view.scroll,
            changes.iter().copied().find(|&row| row > 0).unwrap_or(0)
        );
    }
}
