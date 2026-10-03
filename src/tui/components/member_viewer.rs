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

//! A container pair on screen (see [`crate::diff::content::container`]): the members in a list,
//! changed ones only until `a` shows all, and the selected member beside it - a picture in the
//! picture view, text as a line diff, a nested container as a line saying `Enter` opens it.
//!
//! The list's first row is the pair itself, so a reader (or `human_solver`) can stand on the
//! whole file as well as on one member. A member is opened only when it is selected: a font's
//! six hundred glyphs are not drawn to show two.
//!
//! `Enter` on a member that is itself a container (a jar in a zip) opens it in place, and
//! `Backspace` comes back out.

use std::collections::HashMap;
use std::path::Path;

use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use ratatui_image::picker::Picker;

use super::content_viewer::ContentViewer;
use super::picture_viewer::{PictureColors, PictureViewer};
use crate::diff::content::container::{
    self, Aligned, Container, ContainerDiff, ContainerInfo, MemberContent, MemberStatus,
};
use crate::diff::content::{self, Engine, Family, Verdict};
use crate::diff::picture;

/// What the selected member looks like on screen.
enum MemberView {
    Picture(Box<PictureViewer>),
    /// A line diff: each line with its mark, and how far it is scrolled.
    Text {
        lines: Vec<(char, String)>,
        scroll: usize,
    },
    /// A member with nothing to draw: why, in a sentence.
    Message(String),
    /// A nested container, opened only on `Enter`.
    Nested {
        before: Vec<u8>,
        after: Vec<u8>,
    },
}

pub struct MemberViewer {
    before_name: String,
    after_name: String,
    family: Family,
    before: Option<Box<dyn Container>>,
    after: Option<Box<dyn Container>>,
    diff: ContainerDiff,
    rows: Vec<Aligned>,
    /// Show every member, not only the ones that changed, were added or were removed.
    show_all: bool,
    /// The selected row of the visible ones; 0 is the pair itself.
    selected: usize,
    view: Option<MemberView>,
    /// A member container opened with `Enter`, which takes the keys until `Backspace`.
    nested: Option<Box<ContentViewer>>,
    picker: Picker,
    annotating: bool,
    /// A short label per member key, shown after it in the list (`human_solver`'s verdicts).
    marks: HashMap<String, String>,
}

impl MemberViewer {
    /// The view of two decoded containers of `family`.
    pub fn new(
        names: (String, String),
        family: Family,
        before: Option<Box<dyn Container>>,
        after: Option<Box<dyn Container>>,
        picker: Picker,
        annotating: bool,
    ) -> Self {
        let diff = container::compare(family, before.as_deref(), after.as_deref(), 0);
        let rows = container::align(before.as_deref(), after.as_deref());
        Self {
            before_name: names.0,
            after_name: names.1,
            family,
            before,
            after,
            diff,
            rows,
            show_all: false,
            selected: 0,
            view: None,
            nested: None,
            picker,
            annotating,
            marks: HashMap::new(),
        }
    }

    /// The view of the container pair at `before` and `after`, if it decodes.
    pub fn open(
        before: &Path,
        after: &Path,
        family: Family,
        picker: Picker,
        annotating: bool,
    ) -> Option<Self> {
        let read = |path: &Path| std::fs::read(path).ok();
        let (before_bytes, after_bytes) = (read(before)?, read(after)?);
        let (before_side, after_side) =
            content::decode_container_pair(&before_bytes, &after_bytes).ok()?;
        Some(Self::new(
            (before.display().to_string(), after.display().to_string()),
            family,
            before_side,
            after_side,
            picker,
            annotating,
        ))
    }

    pub fn diff(&self) -> &ContainerDiff {
        &self.diff
    }

    pub fn family(&self) -> Family {
        self.family
    }

    pub fn set_picker(&mut self, picker: Picker) {
        self.picker = picker;
        self.view = None;
        if let Some(nested) = self.nested.as_mut() {
            nested.set_picker(self.picker.clone());
        }
    }

    pub fn annotating(&self) -> bool {
        self.annotating
    }

    pub fn set_annotating(&mut self, annotating: bool) {
        self.annotating = annotating;
        self.view = None;
        if let Some(nested) = self.nested.as_mut() {
            nested.set_annotating(annotating);
        }
    }

    pub fn is_playing(&self) -> bool {
        match (&self.nested, &self.view) {
            (Some(nested), _) => nested.is_playing(),
            (None, Some(MemberView::Picture(viewer))) => viewer.is_playing(),
            _ => false,
        }
    }

    /// Labels to show after members' keys in the list, by key.
    pub fn set_marks(&mut self, marks: HashMap<String, String>) {
        self.marks = marks;
    }

    /// The rows shown: every member with `a`, otherwise only those that differ.
    fn visible(&self) -> Vec<&Aligned> {
        self.rows
            .iter()
            .filter(|row| self.show_all || row.status != MemberStatus::Same)
            .collect()
    }

    fn key_of(&self, row: &Aligned) -> &str {
        let (side, index) = match (row.after, row.before) {
            (Some(index), _) => (self.after.as_deref(), index),
            (None, Some(index)) => (self.before.as_deref(), index),
            (None, None) => unreachable!("an aligned row has a side"),
        };
        &side.expect("a row's side is present").members()[index].key
    }

    /// The selected member's key; `None` while the pair itself is selected.
    pub fn selected_key(&self) -> Option<&str> {
        let visible = self.visible();
        let row = *visible.get(self.selected.checked_sub(1)?)?;
        Some(self.key_of(row))
    }

    /// The selected member's status; `None` while the pair itself is selected.
    pub fn selected_status(&self) -> Option<MemberStatus> {
        let visible = self.visible();
        Some(visible.get(self.selected.checked_sub(1)?)?.status)
    }

    /// The keys of the members that changed, in the list's order: the ones a verdict is given.
    pub fn changed_keys(&self) -> Vec<String> {
        self.rows
            .iter()
            .filter(|row| row.status == MemberStatus::Changed)
            .map(|row| self.key_of(row).to_string())
            .collect()
    }

    /// Selects the member called `key` (showing every member if it is unchanged), or the pair
    /// for `None`.
    pub fn select(&mut self, key: Option<&str>) {
        self.nested = None;
        self.view = None;
        let Some(key) = key else {
            self.selected = 0;
            return;
        };
        let position = |viewer: &Self| {
            viewer
                .visible()
                .iter()
                .position(|row| viewer.key_of(row) == key)
        };
        if position(self).is_none() {
            self.show_all = true;
        }
        if let Some(position) = position(self) {
            self.selected = position + 1;
        }
    }

    /// The engine's verdict on member `key`, if it changed.
    pub fn member_verdict(&self, key: &str) -> Option<Verdict> {
        self.diff.member(key)?.verdict()
    }

    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        if let Some(nested) = self.nested.as_mut() {
            if code == KeyCode::Backspace && !nested.has_nested() {
                self.nested = None;
                return true;
            }
            return nested.handle_key(code);
        }
        let count = self.visible().len() + 1;
        match code {
            KeyCode::Char('j') | KeyCode::Down => self.move_to((self.selected + 1).min(count - 1)),
            KeyCode::Char('k') | KeyCode::Up => self.move_to(self.selected.saturating_sub(1)),
            KeyCode::Home => self.move_to(0),
            KeyCode::End => self.move_to(count - 1),
            KeyCode::Char('a') => {
                let key = self.selected_key().map(str::to_string);
                self.show_all = !self.show_all;
                self.select(key.as_deref());
            }
            KeyCode::Enter => {
                self.ensure_view();
                let Some(MemberView::Nested { before, after }) = &self.view else {
                    return false;
                };
                let names = self
                    .selected_key()
                    .map(|key| {
                        (
                            format!("{} {key}", self.before_name),
                            format!("{} {key}", self.after_name),
                        )
                    })
                    .unwrap_or_default();
                self.nested = ContentViewer::from_bytes(
                    names,
                    before,
                    after,
                    self.picker.clone(),
                    self.annotating,
                )
                .map(Box::new);
            }
            KeyCode::PageDown | KeyCode::Char('J') => self.scroll(10),
            KeyCode::PageUp | KeyCode::Char('K') => self.scroll(-10),
            code => {
                self.ensure_view();
                return match self.view.as_mut() {
                    Some(MemberView::Picture(viewer)) => viewer.handle_key(code),
                    _ => false,
                };
            }
        }
        true
    }

    /// True while a member container opened with `Enter` is showing.
    pub fn has_nested(&self) -> bool {
        self.nested.is_some()
    }

    fn move_to(&mut self, selected: usize) {
        if selected != self.selected {
            self.selected = selected;
            self.view = None;
        }
    }

    fn scroll(&mut self, by: isize) {
        if let Some(MemberView::Text { lines, scroll }) = self.view.as_mut() {
            *scroll = scroll
                .saturating_add_signed(by)
                .min(lines.len().saturating_sub(1));
        }
    }

    /// Builds the selected member's view, if it is not built yet.
    fn ensure_view(&mut self) {
        if self.view.is_some() {
            return;
        }
        let visible = self.visible();
        let Some(row) = self
            .selected
            .checked_sub(1)
            .and_then(|index| visible.get(index))
            .map(|row| **row)
        else {
            self.view = Some(MemberView::Message(self.pair_summary()));
            return;
        };
        let key = self.key_of(&row).to_string();
        let open = |side: Option<&dyn Container>,
                    index: Option<usize>|
         -> Result<Option<MemberContent>, String> {
            match (side, index) {
                (Some(side), Some(index)) => side
                    .open(index)
                    .map(Some)
                    .map_err(|err| format!("{key} does not open: {err:#}")),
                _ => Ok(None),
            }
        };
        let opened = open(self.before.as_deref(), row.before)
            .and_then(|before| Ok((before, open(self.after.as_deref(), row.after)?)));
        self.view = Some(match opened {
            Ok((before, after)) => self.member_view(&key, before, after),
            Err(message) => MemberView::Message(message),
        });
    }

    /// The view of one member, from what each side opened to.
    fn member_view(
        &self,
        key: &str,
        before: Option<MemberContent>,
        after: Option<MemberContent>,
    ) -> MemberView {
        let names = (format!("before: {key}"), format!("after: {key}"));
        match (before, after) {
            (before, after)
                if [&before, &after]
                    .iter()
                    .all(|side| matches!(side, None | Some(MemberContent::Picture(_)))) =>
            {
                let decoded = |side: Option<MemberContent>| match side {
                    Some(MemberContent::Picture(picture)) => Some(*picture),
                    _ => None,
                };
                MemberView::Picture(Box::new(PictureViewer::from_decoded(
                    names.0,
                    names.1,
                    decoded(before),
                    decoded(after),
                    self.picker.clone(),
                    self.annotating,
                )))
            }
            (before, after) => {
                let bytes = |side: &Option<MemberContent>| -> Option<Vec<u8>> {
                    match side {
                        Some(MemberContent::Bytes(bytes)) => Some(bytes.clone()),
                        Some(MemberContent::Text(text)) => Some(text.clone().into_bytes()),
                        _ => None,
                    }
                };
                let (b, a) = (
                    bytes(&before).unwrap_or_default(),
                    bytes(&after).unwrap_or_default(),
                );
                match content::pair_kind(&b, &a) {
                    Some(Engine::Picture) => match picture::decode_pair(&b, &a) {
                        Ok((before, after)) => {
                            MemberView::Picture(Box::new(PictureViewer::from_decoded(
                                names.0,
                                names.1,
                                before,
                                after,
                                self.picker.clone(),
                                self.annotating,
                            )))
                        }
                        Err(err) => MemberView::Message(format!("{key} does not decode: {err:#}")),
                    },
                    Some(Engine::Container(_)) => MemberView::Nested {
                        before: b,
                        after: a,
                    },
                    None => match (
                        before.is_none() || crate::code::decode_text(&b).is_some(),
                        after.is_none() || crate::code::decode_text(&a).is_some(),
                    ) {
                        (true, true) => MemberView::Text {
                            lines: text_lines(
                                &crate::code::decode_text(&b).unwrap_or_default(),
                                &crate::code::decode_text(&a).unwrap_or_default(),
                            ),
                            scroll: 0,
                        },
                        _ => MemberView::Message(format!(
                            "{key}: binary, {} -> {} bytes",
                            b.len(),
                            a.len()
                        )),
                    },
                }
            }
        }
    }

    /// What the pair row shows: each side, and how many members changed.
    fn pair_summary(&self) -> String {
        let side = |info: &Option<ContainerInfo>| match info {
            Some(info) => format!(
                "{}, {} members, {} bytes",
                info.format, info.members, info.bytes
            ),
            None => "nothing".to_string(),
        };
        format!(
            "{} -> {}\n\n{}",
            side(&self.diff.before),
            side(&self.diff.after),
            self.counts()
        )
    }

    fn counts(&self) -> String {
        let count = |status| self.rows.iter().filter(|row| row.status == status).count();
        format!(
            "{} changed, {} added, {} removed, {} unchanged",
            count(MemberStatus::Changed),
            count(MemberStatus::Added),
            count(MemberStatus::Removed),
            count(MemberStatus::Same)
        )
    }

    /// One line for the status bar.
    pub fn status(&self) -> String {
        if let Some(nested) = &self.nested {
            return format!("{} (Backspace: back)", nested.status());
        }
        let shown = if self.show_all {
            "all (a: changed)"
        } else {
            "changed (a: all)"
        };
        let member = match (self.selected_key(), &self.view) {
            (None, _) => String::new(),
            (Some(key), Some(MemberView::Picture(viewer))) => {
                format!(" · {key}: {}", viewer.status())
            }
            (Some(key), Some(MemberView::Nested { .. })) => {
                format!(" · {key}: Enter opens it")
            }
            (Some(key), _) => match (self.annotating, self.member_verdict(key)) {
                (false, Some(verdict)) => format!(" · {key}: omnidiff says {}", verdict.label()),
                _ => format!(" · {key}"),
            },
        };
        format!(
            "{}: {} · showing {shown}, j/k move{member}",
            self.family.noun(true),
            self.counts()
        )
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect, colors: PictureColors) {
        if let Some(nested) = self.nested.as_mut() {
            nested.draw(frame, area, colors);
            return;
        }
        self.ensure_view();
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);
        self.draw_list(frame, columns[0]);
        let title = self.selected_key().unwrap_or_default().to_string();
        match self.view.as_mut() {
            Some(MemberView::Picture(viewer)) => viewer.draw(frame, columns[1], colors),
            Some(MemberView::Text { lines, scroll }) => {
                let block = Block::default().borders(Borders::ALL).title(title);
                let inner = block.inner(columns[1]);
                frame.render_widget(block, columns[1]);
                let shown: Vec<Line> = lines
                    .iter()
                    .skip(*scroll)
                    .take(inner.height as usize)
                    .map(|(mark, text)| {
                        let line = format!("{mark} {text}");
                        match mark {
                            '-' => Line::from(line.red()),
                            '+' => Line::from(line.green()),
                            _ => Line::from(line),
                        }
                    })
                    .collect();
                frame.render_widget(Paragraph::new(shown), inner);
            }
            Some(MemberView::Message(message)) => {
                let block = Block::default().borders(Borders::ALL);
                let inner = block.inner(columns[1]);
                frame.render_widget(block, columns[1]);
                frame.render_widget(Paragraph::new(message.as_str()), inner);
            }
            Some(MemberView::Nested { .. }) => {
                let block = Block::default().borders(Borders::ALL);
                let inner = block.inner(columns[1]);
                frame.render_widget(block, columns[1]);
                frame.render_widget(
                    Paragraph::new("A container: Enter opens it, Backspace comes back."),
                    inner,
                );
            }
            None => {}
        }
    }

    fn draw_list(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default().borders(Borders::ALL).title(format!(
            "{} -> {}",
            short(&self.before_name),
            short(&self.after_name)
        ));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let visible = self.visible();
        let height = inner.height as usize;
        // Keep the selection on screen, a third of the way down once the list scrolls.
        let first = self
            .selected
            .saturating_sub(height / 3)
            .min((visible.len() + 1).saturating_sub(height));
        let mut lines = Vec::with_capacity(height);
        for index in first..(visible.len() + 1).min(first + height) {
            let (mark, key) = match index {
                0 => (' ', "(the whole file)".to_string()),
                _ => {
                    let row = visible[index - 1];
                    let mark = match row.status {
                        MemberStatus::Same => ' ',
                        MemberStatus::Changed => '~',
                        MemberStatus::Added => '+',
                        MemberStatus::Removed => '-',
                    };
                    (mark, self.key_of(row).to_string())
                }
            };
            let label = if index == 0 {
                self.marks.get("").cloned()
            } else {
                self.marks.get(&key).cloned()
            };
            let color = match mark {
                '~' => Color::Yellow,
                '+' => Color::Green,
                '-' => Color::Red,
                _ => Color::Reset,
            };
            let mut spans = vec![
                Span::styled(format!("{mark} "), Style::new().fg(color)),
                Span::from(key),
            ];
            if let Some(label) = label {
                spans.push(format!("  {label}").cyan());
            }
            let mut line = Line::from(spans);
            if index == self.selected {
                line = line.style(Style::new().add_modifier(Modifier::REVERSED));
            }
            lines.push(line);
        }
        frame.render_widget(Paragraph::new(lines), inner);
    }
}

/// A path's file name, for a list title.
fn short(name: &str) -> String {
    Path::new(name)
        .file_name()
        .map_or(name.to_string(), |file| file.to_string_lossy().into_owned())
}

/// Two texts as one line diff: each line marked ' ' (both), '-' (before only) or '+' (after
/// only), in order.
fn text_lines(before: &str, after: &str) -> Vec<(char, String)> {
    let (b, a): (Vec<&str>, Vec<&str>) = (before.lines().collect(), after.lines().collect());
    let pairs = crate::diff::text::line_diff_core(before, after, 10_000)
        .map(|core| core.pairs)
        .unwrap_or_default();
    let mut lines = Vec::with_capacity(b.len().max(a.len()));
    let (mut i, mut j) = (0, 0);
    for (pb, pa) in pairs.into_iter().chain([(b.len(), a.len())]) {
        lines.extend(b[i..pb].iter().map(|line| ('-', line.to_string())));
        lines.extend(a[j..pa].iter().map(|line| ('+', line.to_string())));
        if pb < b.len() {
            lines.push((' ', b[pb].to_string()));
        }
        (i, j) = (pb + 1, pa + 1);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::content::archive::tests::zip;

    fn viewer(before: &[(&str, &[u8])], after: &[(&str, &[u8])]) -> MemberViewer {
        let decode = |files: &[(&str, &[u8])]| content::decode_container(&zip(files)).unwrap();
        MemberViewer::new(
            ("a.zip".to_string(), "b.zip".to_string()),
            Family::Archives,
            Some(decode(before)),
            Some(decode(after)),
            Picker::halfblocks(),
            false,
        )
    }

    fn screen(viewer: &mut MemberViewer) -> String {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 12)).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                viewer.draw(
                    frame,
                    area,
                    PictureColors::from_theme(Color::Red, Color::Green, Color::Yellow),
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn text_lines_mark_what_each_side_has() {
        assert_eq!(
            text_lines("a\nb\nc\n", "a\nB\nc\nd\n"),
            vec![
                (' ', "a".to_string()),
                ('-', "b".to_string()),
                ('+', "B".to_string()),
                (' ', "c".to_string()),
                ('+', "d".to_string()),
            ]
        );
    }

    #[test]
    fn the_list_shows_changed_members_and_a_shows_them_all() {
        let mut viewer = viewer(
            &[
                ("same.txt", b"x\n"),
                ("edit.txt", b"one\n"),
                ("gone.txt", b"g\n"),
            ],
            &[
                ("same.txt", b"x\n"),
                ("edit.txt", b"two\n"),
                ("new.txt", b"n\n"),
            ],
        );
        let text = screen(&mut viewer);
        assert!(text.contains("~ edit.txt"), "{text}");
        assert!(text.contains("- gone.txt"), "{text}");
        assert!(text.contains("+ new.txt"), "{text}");
        assert!(!text.contains("same.txt"), "{text}");
        assert!(
            text.contains("1 changed, 1 added, 1 removed, 1 unchanged"),
            "{text}"
        );

        viewer.handle_key(KeyCode::Char('a'));
        assert!(screen(&mut viewer).contains("same.txt"));
        assert_eq!(viewer.changed_keys(), vec!["edit.txt".to_string()]);
    }

    #[test]
    fn a_changed_text_member_shows_as_a_line_diff() {
        let mut viewer = viewer(
            &[("edit.txt", b"zero\none\nthree\n")],
            &[("edit.txt", b"zero\ntwo\nthree\n")],
        );
        viewer.handle_key(KeyCode::Char('j'));
        assert_eq!(viewer.selected_key(), Some("edit.txt"));
        let text = screen(&mut viewer);
        assert!(text.contains("- one"), "{text}");
        assert!(text.contains("+ two"), "{text}");
        assert!(
            viewer.status().contains("omnidiff says content change"),
            "{}",
            viewer.status()
        );
        viewer.set_annotating(true);
        assert!(
            !viewer.status().contains("omnidiff says"),
            "{}",
            viewer.status()
        );
    }

    #[test]
    fn enter_opens_a_nested_archive_and_backspace_comes_back() {
        let inner_before = zip(&[("deep.txt", b"1\n")]);
        let inner_after = zip(&[("deep.txt", b"2\n")]);
        let mut viewer = viewer(
            &[("inner.zip", inner_before.as_slice())],
            &[("inner.zip", inner_after.as_slice())],
        );
        viewer.handle_key(KeyCode::Char('j'));
        assert!(viewer.handle_key(KeyCode::Enter));
        assert!(viewer.has_nested());
        assert!(screen(&mut viewer).contains("~ deep.txt"));
        assert!(viewer.handle_key(KeyCode::Backspace));
        assert!(!viewer.has_nested());
    }

    #[test]
    fn a_picture_member_is_shown_as_a_picture() {
        let png = |pixel: u8| {
            let mut bytes = Vec::new();
            image::RgbaImage::from_pixel(4, 4, image::Rgba([pixel, 0, 0, 255]))
                .write_to(
                    &mut std::io::Cursor::new(&mut bytes),
                    image::ImageFormat::Png,
                )
                .unwrap();
            bytes
        };
        let (a, b) = (png(0), png(255));
        let mut viewer = viewer(&[("icon.png", a.as_slice())], &[("icon.png", b.as_slice())]);
        viewer.select(Some("icon.png"));
        screen(&mut viewer);
        assert!(viewer.status().contains("PNG 4x4"), "{}", viewer.status());
        assert_eq!(viewer.member_verdict("icon.png"), Some(Verdict::Replaced));
    }
}
