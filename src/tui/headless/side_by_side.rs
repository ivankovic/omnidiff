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

//! The side-by-side layout of headless output (`--layout side-by-side|auto`): the before side on
//! the left, the after side on the right, the lines omnidiff matched on one row.
//!
//! **Which layout.** `stacked`, the default, is the output as it always was. `side-by-side` needs
//! a known width (see [`resolve_width`]) and at least [`MIN_CODE_COLUMNS`] display columns of code
//! in each column; `auto` needs [`AUTO_MIN_CODE_COLUMNS`] and [`AUTO_FIT_PERCENT`] of the rows it
//! will show fitting their columns without wrapping. Otherwise the output is stacked, silently: a
//! warning line would land in lazygit's view or in `git diff`'s output.
//!
//! **Width**, first known wins: `--width`, `$COLUMNS`, stdout's size if it is a terminal (lazygit
//! gives its renderer a pty sized to the pane), the controlling terminal's through `/dev/tty`
//! (under `git diff` stdout is the pager's pipe and git sets no `COLUMNS`, but the terminal is
//! still there). Zero counts as unknown at every step. Not on Windows past `$COLUMNS`.
//!
//! **Rows** come from [`super::alignment`]: the engine's node mapping (or, without a tree, the
//! painting's matched ranges), not git's patch, and never from the render options. A row
//! whose line sits alone although it matches a line elsewhere (a move) ends its half with that
//! line's number, `→ N` on the left and `← N` on the right; the stacked layout's `Moved to/from`
//! box headers are not drawn, the `|` marker of a moved line is.
//!
//! **A half** is the line number (as wide as the side's largest), the stacked layout's two marker
//! columns and a space, and the code. Code is measured in display columns (`unicode-width`, tabs
//! expanded to `display_columns::TAB_WIDTH` stops from the line's start) and wraps inside its
//! column onto continuation rows with no number: a wide character that would straddle the edge
//! moves to the next row and the gap is padded, a zero-width one stays with its base. Every
//! colored piece is opened and reset within its row, so no color crosses the `│` divider.
//!
//! **Folding** counts rows: `--context N` rows around each changed one, a changed row being one
//! with a line omnidiff paints or, with OSC 1717 negotiated, a line git's patch changes. An elided
//! run is one `... N unchanged lines ...` row across both columns (`N before, M after` when the
//! sides differ); a region whose enclosing declaration is out of view is headed by the *after*
//! side's `@` breadcrumb.
//!
//! **OSC 1717** (`tui::osc1717`, spec §6.2 and §6.3): the records are classified in patch space
//! exactly as in the stacked layout, whatever the row pairing. Each half starts with its line's
//! record (left `d`/`c`, right `a`/`c`; the right one right after the divider), a blank half has
//! none, and a continuation row repeats the record of its half. The header row carries one `f`;
//! a fold or breadcrumb row heading a region one `h`, naming the first new line of the region.

use std::ffi::OsStr;

use super::alignment::{self, RowLines};
use super::{
    Breadcrumbs, CHROME_COLOR, Piece, SideLines, keep_around, markers, paint_pieces,
    printable_path, push_chrome_row, summary_header,
};
use crate::diff::text_range::cell_width_of;
use crate::tui::actions::DiffSessionData;
use crate::tui::display_columns;
use crate::tui::osc1717::{self, Annotation, Patch, Section, SectionRecords};

/// How headless output lays out the two sides of a text diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    /// The before side, then the after side.
    #[default]
    Stacked,
    /// Side by side when it fits well, else stacked.
    Auto,
    /// Side by side unless the columns are too narrow for any code.
    SideBySide,
}

/// A [`Layout`] and the width to lay it out in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LayoutRequest {
    pub layout: Layout,
    /// The output's width in columns; `None` when unknown, which is always stacked.
    pub width: Option<usize>,
}

/// Display columns of code each column needs for `auto` to pick side by side.
pub const AUTO_MIN_CODE_COLUMNS: usize = 40;

/// Display columns of code each column needs for side by side at all.
pub const MIN_CODE_COLUMNS: usize = 20;

/// The share of shown rows, in percent, that must fit without wrapping for `auto`.
pub const AUTO_FIT_PERCENT: usize = 90;

/// The output width, first known wins (see the module doc): `flag`, `columns` (the `COLUMNS`
/// variable), then `stdout` and `tty`, each asked only if needed. Zero is unknown.
pub fn resolve_width(
    flag: Option<usize>,
    columns: Option<&OsStr>,
    stdout: impl FnOnce() -> Option<usize>,
    tty: impl FnOnce() -> Option<usize>,
) -> Option<usize> {
    let known = |width: Option<usize>| width.filter(|&width| width > 0);
    known(flag)
        .or_else(|| {
            known(
                columns
                    .and_then(OsStr::to_str)
                    .and_then(|text| text.trim().parse().ok()),
            )
        })
        .or_else(|| known(stdout()))
        .or_else(|| known(tty()))
}

/// [`resolve_width`] of this process.
pub fn terminal_width(flag: Option<usize>) -> Option<usize> {
    resolve_width(
        flag,
        std::env::var_os("COLUMNS").as_deref(),
        stdout_columns,
        tty_columns,
    )
}

/// The columns of the terminal `fd` is, by `TIOCGWINSZ`.
#[cfg(unix)]
fn window_columns(fd: std::os::fd::RawFd) -> Option<usize> {
    let mut size = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: `TIOCGWINSZ` writes one `winsize` through the pointer, valid and exclusive for the
    // call; on a descriptor that is no terminal it fails and writes nothing.
    let status = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut size) };
    (status == 0).then_some(usize::from(size.ws_col))
}

#[cfg(unix)]
fn stdout_columns() -> Option<usize> {
    use std::io::IsTerminal;
    use std::os::fd::AsRawFd;
    let stdout = std::io::stdout();
    if !stdout.is_terminal() {
        return None;
    }
    window_columns(stdout.as_raw_fd())
}

#[cfg(unix)]
fn tty_columns() -> Option<usize> {
    use std::os::fd::AsRawFd;
    let tty = std::fs::File::open("/dev/tty").ok()?;
    window_columns(tty.as_raw_fd())
}

#[cfg(not(unix))]
fn stdout_columns() -> Option<usize> {
    None
}

#[cfg(not(unix))]
fn tty_columns() -> Option<usize> {
    None
}

/// How many decimal digits `n` has.
fn digits(n: usize) -> usize {
    n.max(1).to_string().len()
}

/// The row of `pieces` wrapped to `width` display columns, each row with its own pieces and the
/// columns they take (see the module doc). Always at least one row; a character wider than
/// `width` gets a row of its own.
fn wrap(pieces: &[Piece], width: usize) -> Vec<(Vec<Piece>, usize)> {
    let mut rows: Vec<(Vec<Piece>, usize)> = vec![(Vec::new(), 0)];
    for piece in pieces {
        let mut text = String::new();
        for ch in piece.text.chars() {
            let char_width = cell_width_of(ch).get();
            let used = rows.last().map_or(0, |(_, used)| *used);
            if char_width > 0 && used > 0 && used + char_width > width {
                if !text.is_empty() {
                    rows.last_mut().unwrap().0.push(Piece {
                        text: std::mem::take(&mut text),
                        color: piece.color,
                    });
                }
                rows.push((Vec::new(), 0));
            }
            text.push(ch);
            rows.last_mut().unwrap().1 += char_width;
        }
        if !text.is_empty() {
            rows.last_mut().unwrap().0.push(Piece {
                text,
                color: piece.color,
            });
        }
    }
    rows
}

/// `text`'s first `width` display columns.
fn truncate(text: &str, width: usize) -> String {
    let piece = Piece {
        text: text.to_string(),
        color: None,
    };
    wrap(&[piece], width)
        .swap_remove(0)
        .0
        .into_iter()
        .map(|piece| piece.text)
        .collect()
}

/// `text` if it fits `width` display columns, else `…` and as much of its end as fits.
fn fit_tail(text: &str, width: usize) -> String {
    if display_columns::display_width(text) <= width {
        return text.to_string();
    }
    let mut tail = Vec::new();
    let mut used = 1;
    for ch in text.chars().rev() {
        let char_width = cell_width_of(ch).get();
        if used + char_width > width {
            break;
        }
        used += char_width;
        tail.push(ch);
    }
    std::iter::once('…').chain(tail.into_iter().rev()).collect()
}

/// The widths of the layout: `left` columns, the divider, a space, `right` columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Geometry {
    width: usize,
    left: usize,
    right: usize,
    old_number: usize,
    new_number: usize,
}

impl Geometry {
    fn new(width: usize, old_lines: usize, new_lines: usize) -> Self {
        let halves = width.saturating_sub(2);
        Self {
            width,
            left: halves / 2,
            right: halves - halves / 2,
            old_number: digits(old_lines),
            new_number: digits(new_lines),
        }
    }

    /// Display columns of code in the left column: the half less the number, its space, and the
    /// two marker columns and their space.
    fn old_code(&self) -> usize {
        self.left.saturating_sub(self.old_number + 4)
    }

    fn new_code(&self) -> usize {
        self.right.saturating_sub(self.new_number + 4)
    }
}

/// One side's column.
struct Half<'a> {
    section: Section,
    side: &'a SideLines<'a>,
    records: Option<&'a SectionRecords<'a>>,
    /// Per line on a one-sided row, the line it matches on the other side.
    counterpart: &'a [Option<usize>],
    number_width: usize,
    code_width: usize,
    /// Pad every row to the column's full width: the left column, so the divider lines up.
    pad: bool,
    /// What comes between the record and the number: the space after the divider.
    lead: &'static str,
}

impl Half<'_> {
    /// The `→ N`/`← N` naming line `line`'s counterpart, if it has one.
    fn arrow(&self, line: usize) -> Option<String> {
        let other = self.counterpart.get(line).copied().flatten()?;
        Some(match self.section {
            Section::Before => format!("→ {}", other + 1),
            Section::After => format!("← {}", other + 1),
        })
    }

    /// The columns line `line`'s code wraps at: the column's, less its arrow and a space.
    fn code_width(&self, line: usize) -> usize {
        match self.arrow(line) {
            Some(arrow) => self
                .code_width
                .saturating_sub(display_columns::display_width(&arrow) + 1)
                .max(1),
            None => self.code_width,
        }
    }

    /// Whether line `line` fits its column without wrapping.
    fn fits(&self, line: usize) -> bool {
        display_columns::display_width(self.side.lines[line]) <= self.code_width(line)
    }

    /// Line `line`'s rows in this column, the record first on each.
    fn rows(&self, line: usize, use_color: bool) -> Vec<String> {
        let code_width = self.code_width(line);
        let arrow = self.arrow(line);
        let record = self.records.and_then(|records| records.content(line));
        let wrapped = wrap(&self.side.pieces(line, use_color), code_width);
        let mut rows = Vec::with_capacity(wrapped.len());
        for (index, (pieces, used)) in wrapped.iter().enumerate() {
            let mut out = String::new();
            if let Some(record) = &record {
                out.push_str(record);
            }
            out.push_str(self.lead);
            if index == 0 {
                let number = format!("{:>width$} ", line + 1, width = self.number_width);
                push_chrome(&mut out, &number, use_color);
                out.push_str(&markers(self.side.flags[line], use_color));
            } else {
                out.push_str(&" ".repeat(self.number_width + 4));
            }
            paint_pieces(&mut out, pieces);
            let arrow = arrow.as_ref().filter(|_| index == 0 || self.pad);
            let Some(arrow) = arrow else {
                if self.pad {
                    out.push_str(&" ".repeat(code_width.saturating_sub(*used)));
                }
                rows.push(out);
                continue;
            };
            out.push_str(&" ".repeat(code_width.saturating_sub(*used) + 1));
            if index == 0 {
                push_chrome(&mut out, arrow, use_color);
            } else if self.pad {
                out.push_str(&" ".repeat(display_columns::display_width(arrow)));
            }
            rows.push(out);
        }
        rows
    }
}

/// `text` in the chrome grey when `use_color`.
fn push_chrome(out: &mut String, text: &str, use_color: bool) {
    if use_color {
        out.push_str(&format!("\u{1b}[{CHROME_COLOR}m{text}\u{1b}[0m"));
    } else {
        out.push_str(text);
    }
}

/// The header row: `── before: P ──…┬─ after: Q ──…`, a path too long for its column cut to its
/// end.
fn header_row(before: &str, after: &str, geometry: &Geometry) -> String {
    let side = |rule: &str, label: &str, path: &str, width: usize| {
        let prefix = format!("{rule} {label}: ");
        // A space and at least one rule after the path.
        let room = width.saturating_sub(display_columns::display_width(&prefix) + 2);
        let text = format!("{prefix}{} ", fit_tail(path, room));
        let fill = width.saturating_sub(display_columns::display_width(&text));
        format!("{text}{}", "─".repeat(fill))
    };
    format!(
        "{}┬{}",
        side("──", "before", before, geometry.left),
        side("─", "after", after, geometry.right + 1)
    )
}

/// The fold row's text for an elided run of `old` and `new` lines.
fn fold_text(old: usize, new: usize) -> String {
    let lines = |n: usize| format!("{n} unchanged line{}", if n == 1 { "" } else { "s" });
    if old == new {
        format!("      ... {} ...", lines(old))
    } else {
        format!("      ... {} before, {new} after ...", lines(old))
    }
}

/// The rows of `sides`, the first `lines` of each: by the node mapping's `node_lines` when the
/// pair has them; else by the painting's ranges, or by git's `patch` above `cap` pairs; `None`
/// above the cap without a patch.
fn aligned_rows(
    data: &DiffSessionData,
    (old_side, new_side): (&SideLines, &SideLines),
    (old_lines, new_lines): (usize, usize),
    (patch, node_lines): (Option<&Patch>, Option<&alignment::NodeLines>),
    cap: usize,
) -> Option<alignment::Alignment> {
    if let Some(node_lines) = node_lines {
        return Some(alignment::align_by_nodes(
            &old_side.lines[..old_lines],
            &new_side.lines[..new_lines],
            node_lines,
        ));
    }
    let pairs = alignment::line_pairs(
        &data.before_ranges,
        &data.after_ranges,
        old_lines,
        new_lines,
    );
    let updated = |pair: &alignment::LinePair| {
        u64::from(old_side.flags[pair.old].updated) + u64::from(new_side.flags[pair.new].updated)
    };
    if let Some(aligned) = alignment::align(
        &old_side.lines[..old_lines],
        &new_side.lines[..new_lines],
        &pairs,
        updated,
        cap,
    ) {
        return Some(aligned);
    }
    let patch = patch?;
    Some(alignment::rows_by_patch(
        old_lines,
        new_lines,
        |line| patch.is_deleted(line + 1),
        |line| patch.is_added(line + 1),
    ))
}

/// The diff laid out side by side, or `None` when `request` asks for the stacked layout or it does
/// not fit (see the module doc). `fallback_patch` is git's patch of the pair when `annotation`
/// has none, for rows by the patch above the alignment's cap; `node_lines` what the node mapping
/// says of each line, which the rows follow when given.
pub(super) fn render(
    data: &DiffSessionData,
    use_color: bool,
    context: usize,
    annotation: Option<&Annotation>,
    (fallback_patch, node_lines): (Option<&Patch>, Option<&alignment::NodeLines>),
    request: LayoutRequest,
) -> Option<String> {
    let minimum = match request.layout {
        Layout::Stacked => return None,
        Layout::Auto => AUTO_MIN_CODE_COLUMNS,
        Layout::SideBySide => MIN_CODE_COLUMNS,
    };
    let (old_lines, new_lines) = (
        osc1717::line_count(&data.before_contents),
        osc1717::line_count(&data.after_contents),
    );
    let geometry = Geometry::new(request.width?, old_lines, new_lines);
    if geometry.old_code().min(geometry.new_code()) < minimum {
        return None;
    }

    let records = |section| annotation.map(|a| a.section(section, old_lines, new_lines));
    let (before_records, after_records) = (records(Section::Before), records(Section::After));
    let old_side = SideLines::new(
        &data.before_contents,
        &data.before_ranges,
        before_records.as_ref(),
    );
    let new_side = SideLines::new(
        &data.after_contents,
        &data.after_ranges,
        after_records.as_ref(),
    );

    let patch = annotation
        .and_then(|a| a.fitting_patch(old_lines, new_lines))
        .or(fallback_patch.filter(|patch| patch.fits(old_lines, new_lines)));
    let aligned = aligned_rows(
        data,
        (&old_side, &new_side),
        (old_lines, new_lines),
        (patch, node_lines),
        alignment::PAIR_CAP,
    )?;
    let rows = &aligned.rows;
    let changed: Vec<bool> = rows
        .iter()
        .map(|row| {
            row.old.is_some_and(|line| old_side.changed(line))
                || row.new.is_some_and(|line| new_side.changed(line))
        })
        .collect();
    let keep = keep_around(&changed, context);

    let left = Half {
        section: Section::Before,
        side: &old_side,
        records: before_records.as_ref(),
        counterpart: &aligned.old_counterpart,
        number_width: geometry.old_number,
        code_width: geometry.old_code(),
        pad: true,
        lead: "",
    };
    let right = Half {
        section: Section::After,
        side: &new_side,
        records: after_records.as_ref(),
        counterpart: &aligned.new_counterpart,
        number_width: geometry.new_number,
        code_width: geometry.new_code(),
        pad: false,
        lead: " ",
    };

    if request.layout == Layout::Auto {
        let shown: Vec<&RowLines> = rows
            .iter()
            .zip(&keep)
            .filter_map(|(row, keep)| keep.then_some(row))
            .collect();
        let fitting = shown
            .iter()
            .filter(|row| {
                row.old.is_none_or(|line| left.fits(line))
                    && row.new.is_none_or(|line| right.fits(line))
            })
            .count();
        if fitting * 100 < shown.len() * AUTO_FIT_PERCENT {
            return None;
        }
    }

    let mut out = String::new();
    if let Some(header) = summary_header(data, use_color) {
        out.push_str(&header);
    }
    if let Some(annotation) = annotation {
        out.push_str(&osc1717::file_record(&annotation.file));
    }
    out.push_str(&header_row(
        &printable_path(&data.before_path),
        &printable_path(&data.after_path),
        &geometry,
    ));
    out.push('\n');

    let breadcrumbs = Breadcrumbs::of(&data.after_path, &data.after_contents);
    let mut row_of_new = vec![0; new_lines];
    for (index, row) in rows.iter().enumerate() {
        if let Some(line) = row.new {
            row_of_new[line] = index;
        }
    }
    let region_header = |line: usize| {
        after_records
            .as_ref()
            .and_then(|records| records.region_header(line))
    };
    let divider = if use_color {
        format!("\u{1b}[{CHROME_COLOR}m│\u{1b}[0m")
    } else {
        "│".to_string()
    };

    let mut index = 0;
    let mut previous_shown = false;
    while index < rows.len() {
        if !keep[index] {
            let start = index;
            while index < rows.len() && !keep[index] {
                index += 1;
            }
            let count = |side: fn(&RowLines) -> Option<usize>| {
                rows[start..index]
                    .iter()
                    .filter(|row| side(row).is_some())
                    .count()
            };
            if index < rows.len()
                && let Some(record) = rows[index..]
                    .iter()
                    .find_map(|row| row.new)
                    .and_then(region_header)
            {
                out.push_str(&record);
            }
            push_chrome_row(
                &mut out,
                &fold_text(count(|row| row.old), count(|row| row.new)),
                use_color,
            );
            previous_shown = false;
            continue;
        }

        if !previous_shown {
            let region_end = (index..rows.len())
                .find(|&at| !keep[at])
                .unwrap_or(rows.len());
            if let Some(first_new) = rows[index..region_end].iter().find_map(|row| row.new)
                && let Some(reference) = breadcrumbs
                    .as_ref()
                    .and_then(|breadcrumbs| breadcrumbs.enclosing(first_new))
                && reference < new_lines
                && !keep[row_of_new[reference]]
            {
                if let Some(record) = region_header(first_new) {
                    out.push_str(&record);
                }
                let breadcrumb = format!(
                    "{:>width$} @ {}",
                    reference + 1,
                    display_columns::expand_tabs(new_side.lines[reference]),
                    width = geometry.new_number
                );
                push_chrome_row(&mut out, &truncate(&breadcrumb, geometry.width), use_color);
            }
        }

        let row = rows[index];
        let left_rows = row
            .old
            .map_or(Vec::new(), |line| left.rows(line, use_color));
        let right_rows = row
            .new
            .map_or(Vec::new(), |line| right.rows(line, use_color));
        for at in 0..left_rows.len().max(right_rows.len()) {
            match left_rows.get(at) {
                Some(text) => out.push_str(text),
                None => out.push_str(&" ".repeat(geometry.left)),
            }
            out.push_str(&divider);
            if let Some(text) = right_rows.get(at) {
                out.push_str(text);
            }
            out.push('\n');
        }
        previous_shown = true;
        index += 1;
    }
    Some(out)
}

#[cfg(test)]
mod tests;
