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
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::{Component, move_selection};
use crate::diff::text::{OPTION_INFO, RenderOptions, TextOperation};
use crate::tui::actions::Action;
use crate::tui::settings_examples::{Painted, SettingsExample, example_for, painted};
use crate::tui::theme::OverlayTheme;

const HINT: &str =
    "↑/↓ move  Space choose  1 Minimal  2 Extended  Tab next tab  Enter or Esc close";

/// The settings column's width: the longest [`OPTION_INFO`] name, its checkbox and the border.
const LIST_WIDTH: u16 = 40;

/// The page's first line: what every row below it is about.
const INTRO: &str = "These settings decide how much of each change is colored.";

/// The presets the page lists first, with what each is for.
const PRESETS: [(&str, RenderOptions, &str); 2] = [
    (
        "Minimal",
        RenderOptions::MINIMAL,
        "Color only the characters that changed, avoiding whitespace. Least to look at.",
    ),
    (
        "Extended",
        RenderOptions::EXTENDED,
        "Color all changed characters, and color some moved or surrounding code as well. The \
         shape of a change is easier to see, but more characters will be highlighted.",
    ),
];

/// The settings under the presets, grouped by what they decide, as [`OPTION_INFO`] indexes. The
/// grouping lives here rather than in [`RenderOptions::options`]'s order, which the examples'
/// declared no-effect indexes and the saved badge order depend on.
const GROUPS: [(&str, [usize; 3]); 3] = [
    ("How much to color", [6, 2, 1]),
    ("Whitespace", [0, 7, 8]),
    ("Moves", [3, 4, 5]),
];

/// One selectable row of the page, top to bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    /// A [`PRESETS`] index.
    Preset(usize),
    /// An [`OPTION_INFO`] index.
    Setting(usize),
    NodeHighlight,
}

/// Every selectable row, in the order the page shows them.
fn rows() -> Vec<Row> {
    (0..PRESETS.len())
        .map(Row::Preset)
        .chain(
            GROUPS
                .iter()
                .flat_map(|(_, members)| members.map(Row::Setting)),
        )
        .chain([Row::NodeHighlight])
        .collect()
}

/// What the page is about: an example in the open diff's language, or non-code content, which
/// these settings do not shape.
#[derive(Debug, Clone, Copy)]
pub enum PageContent {
    Example(&'static SettingsExample),
    NonCode,
}

/// The settings menu's Highlighting tab (the `M` key opens it): the two presets first, then one
/// checkbox row per [`RenderOptions`] field in [`GROUPS`], an example diff in the open diff's
/// language painted under the current settings, and what the selected row does - with the bytes
/// it would change in the example in bold and marked in the gutter, so a setting is seen rather
/// than described. The presets lead because they are the choice that matters: against the
/// hand-painted corpus, flipping any one setting away from a preset almost always agrees with
/// fewer paintings.
///
/// Every toggle applies and persists the moment it is pressed, like the whole menu: there is
/// nothing to accept or cancel, so `Enter` and `Esc` both just close it. The presets are on
/// `1`/`2`, not `m`/`f`: the menu opens here on `M`, and a doubled opening key must not wipe every
/// field and persist that.
///
/// Below the checkboxes, one row that is not a rendering option: the node highlight (the `H` key),
/// which paints over the diff rather than changing it. It has no example to mark, so selecting it
/// explains itself instead.
pub struct RenderOptionsDialog {
    options: RenderOptions,
    /// A [`rows`] index.
    selected: usize,
    content: PageContent,
    theme: OverlayTheme,
    /// The node highlight's state (the `H` key), as the menu opened.
    node_highlight: bool,
    /// The example painted for `(options, selected)`; rebuilt when either changes.
    view: Option<ExampleView>,
}

/// The example under one choice of settings, and what the selected setting changes in it.
struct ExampleView {
    options: RenderOptions,
    selected: Row,
    painted: Painted,
    texts: [String; 2],
    /// Per side, the bytes choosing the selected row repaints.
    marked: [Vec<bool>; 2],
    /// Said instead of an effect when the selected setting changes nothing here.
    note: Option<String>,
}

impl RenderOptionsDialog {
    /// The page with the Rust example and the default colors.
    pub fn new(options: RenderOptions) -> Self {
        Self::for_content(
            options,
            PageContent::Example(example_for(None)),
            OverlayTheme::default(),
        )
    }

    /// Opens on the preset the options are, else the first one.
    pub fn for_content(options: RenderOptions, content: PageContent, theme: OverlayTheme) -> Self {
        Self {
            options,
            selected: PRESETS
                .iter()
                .position(|&(_, preset, _)| preset == options)
                .unwrap_or(0),
            content,
            theme,
            node_highlight: false,
            view: None,
        }
    }

    /// The overlay theme the example is drawn in, changed live from the Colors tab.
    pub fn set_theme(&mut self, theme: OverlayTheme) {
        self.theme = theme;
    }

    pub fn with_node_highlight(mut self, enabled: bool) -> Self {
        self.node_highlight = enabled;
        self
    }

    fn row_count(&self) -> usize {
        rows().len()
    }

    fn row(&self) -> Row {
        rows()[self.selected]
    }

    fn node_row_text() -> Vec<Line<'static>> {
        vec![
            Line::from("Node highlight").bold(),
            Line::from(
                "The range under the cursor, and its match on the other panel, highlight when \
                 part of a real change. Off by default: it repaints on every move and covers the \
                 diff color.",
            ),
            Line::from("Also the H key, anywhere in the diff.").italic(),
        ]
    }

    /// The whole screen: the page replaces the diff while it is open.
    pub fn popup_area(&self, area: Rect) -> Rect {
        area
    }

    /// The example view for the current settings and selection, built if stale.
    fn view(&mut self) -> Option<&ExampleView> {
        let PageContent::Example(example) = self.content else {
            return None;
        };
        let selected = self.row();
        let fresh = self
            .view
            .as_ref()
            .is_some_and(|view| view.options == self.options && view.selected == selected);
        if !fresh {
            self.view = build_view(example, self.options, selected);
        }
        self.view.as_ref()
    }

    fn draw_non_code(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Highlighting settings ");
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let middle = Rect::new(
            inner.x,
            inner.y + inner.height / 2,
            inner.width,
            1.min(inner.height),
        );
        frame.render_widget(
            Paragraph::new("No settings at this time for non-code content.")
                .alignment(ratatui::layout::Alignment::Center),
            middle,
        );
        let hint = Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(1),
            inner.width,
            1.min(inner.height),
        );
        frame.render_widget(Paragraph::new("Enter or Esc close".dim()), hint);
    }
}

/// `example` painted under `options`, the bytes choosing `selected` repaints (a preset picked, a
/// setting flipped), and what to say when it repaints nothing: the example's own reason, or
/// another setting it needs. On the node-highlight row, the example as it is, nothing marked.
fn build_view(
    example: &'static SettingsExample,
    options: RenderOptions,
    selected: Row,
) -> Option<ExampleView> {
    let (now, texts) = painted(example, options)?;
    let flipped = |mut options: RenderOptions, index: usize| {
        options.toggle(index);
        options
    };
    let chosen = match selected {
        Row::Preset(index) => PRESETS[index].1,
        Row::Setting(index) => flipped(options, index),
        Row::NodeHighlight => options,
    };
    let (other, _) = painted(example, chosen)?;
    let marked: [Vec<bool>; 2] = [0, 1].map(|side| {
        now[side]
            .iter()
            .zip(&other[side])
            .map(|(a, b)| a != b)
            .collect()
    });
    let Row::Setting(selected) = selected else {
        let note = match selected {
            Row::Preset(_) if chosen == options => Some("This is the current choice."),
            Row::Preset(_) if marked.iter().flatten().all(|&m| !m) => {
                Some("This example looks the same either way.")
            }
            _ => None,
        };
        return Some(ExampleView {
            options,
            selected,
            painted: now,
            texts,
            marked,
            note: note.map(str::to_string),
        });
    };
    let note = if marked.iter().flatten().any(|&m| m) {
        None
    } else if let Some((_, reason)) = example.no_effect.iter().find(|&&(i, _)| i == selected) {
        Some((*reason).to_string())
    } else {
        // Another setting whose change would give this one something to show.
        (0..options.options().len())
            .filter(|&other| other != selected)
            .find(|&other| {
                let with = flipped(options, other);
                painted(example, with).map(|(p, _)| p)
                    != painted(example, flipped(with, selected)).map(|(p, _)| p)
            })
            .map(|other| {
                let turn = if options.options()[other].1 {
                    "off"
                } else {
                    "on"
                };
                format!(
                    "Nothing to show with the other settings as they are: turn {turn} \"{}\" to \
                     see what this one changes.",
                    OPTION_INFO[other].name
                )
            })
            .or_else(|| Some("This example has nothing for this setting to change.".to_string()))
    };
    Some(ExampleView {
        options,
        selected: Row::Setting(selected),
        painted: now,
        texts,
        marked,
        note,
    })
}

/// The row of `text` to keep in view: the first one the selected setting changes, else the first
/// painted one, else the top.
fn focus_row(text: &str, painted: &[Option<TextOperation>], marked: &[bool]) -> usize {
    let first = |hit: &dyn Fn(usize) -> bool| (0..text.len()).find(|&at| hit(at));
    first(&|at| marked[at])
        .or_else(|| first(&|at| painted[at].is_some()))
        .map_or(0, |at| text[..at].matches('\n').count())
}

/// Which preset `options` is, for the page's header.
fn preset_name(options: RenderOptions) -> &'static str {
    if options == RenderOptions::MINIMAL {
        "Minimal"
    } else if options == RenderOptions::EXTENDED {
        "Extended"
    } else {
        "custom"
    }
}

/// One side of the example as lines: each byte styled by the operation painting it, the bytes
/// the selected setting changes in bold, and a `◆` in the gutter of every row holding one.
fn example_lines(
    text: &str,
    painted: &[Option<TextOperation>],
    marked: &[bool],
    theme: OverlayTheme,
) -> Vec<Line<'static>> {
    let palette = theme.palette();
    let style_of = |label: &Option<TextOperation>, mark: bool| {
        let mut style = match label {
            Some(TextOperation::Insert) => {
                Style::new().bg(palette.insert_bg).fg(palette.overlay_fg)
            }
            Some(TextOperation::Delete) => {
                Style::new().bg(palette.delete_bg).fg(palette.overlay_fg)
            }
            Some(TextOperation::Move) => Style::new().bg(palette.move_bg).fg(palette.overlay_fg),
            Some(TextOperation::Update) => {
                Style::new().bg(palette.update_bg).fg(palette.overlay_fg)
            }
            _ => Style::new(),
        };
        if mark {
            style = style.add_modifier(Modifier::BOLD);
        }
        style
    };
    let mut lines = Vec::new();
    let mut start = 0;
    for row in text.split('\n') {
        let end = start + row.len();
        let row_marked = marked[start..end].iter().any(|&m| m);
        let mut spans = vec![if row_marked {
            Span::from("◆ ").bold()
        } else {
            Span::from("  ")
        }];
        let mut run_start = start;
        let mut chars = row.char_indices().peekable();
        while let Some((offset, _)) = chars.next() {
            let at = start + offset;
            let next = chars.peek().map_or(end, |&(o, _)| start + o);
            let key = (&painted[at], marked[at]);
            let next_key = (next < end).then(|| (&painted[next], marked[next]));
            if next_key != Some(key) {
                spans.push(Span::styled(
                    text[run_start..next].to_string(),
                    style_of(key.0, key.1),
                ));
                run_start = next;
            }
        }
        lines.push(Line::from(spans));
        start = end + 1;
    }
    lines
}

impl Component for RenderOptionsDialog {
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        if matches!(self.content, PageContent::NonCode) {
            return Ok(match key.code {
                KeyCode::Enter | KeyCode::Esc => Some(Action::SettingsClosed),
                _ => None,
            });
        }
        match key.code {
            KeyCode::Up => {
                let len = self.row_count();
                move_selection(&mut self.selected, -1, len);
                Ok(Some(Action::Render))
            }
            KeyCode::Down => {
                let len = self.row_count();
                move_selection(&mut self.selected, 1, len);
                Ok(Some(Action::Render))
            }
            KeyCode::Char(' ') => match self.row() {
                Row::Preset(index) => {
                    self.options = PRESETS[index].1;
                    Ok(Some(Action::RenderOptionsChanged(self.options)))
                }
                Row::Setting(index) => {
                    self.options.toggle(index);
                    Ok(Some(Action::RenderOptionsChanged(self.options)))
                }
                Row::NodeHighlight => {
                    self.node_highlight = !self.node_highlight;
                    Ok(Some(Action::NodeHighlightChanged(self.node_highlight)))
                }
            },
            KeyCode::Enter | KeyCode::Esc => Ok(Some(Action::SettingsClosed)),
            KeyCode::Char('1') => {
                self.options = RenderOptions::MINIMAL;
                Ok(Some(Action::RenderOptionsChanged(self.options)))
            }
            KeyCode::Char('2') => {
                self.options = RenderOptions::EXTENDED;
                Ok(Some(Action::RenderOptionsChanged(self.options)))
            }
            _ => Ok(None),
        }
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let PageContent::Example(example) = self.content else {
            self.draw_non_code(frame, area);
            return Ok(());
        };
        let theme = self.theme;
        let options = self.options;
        let title = format!(
            " Highlighting settings · {} · example in {} ",
            preset_name(options),
            if example.language == crate::code::Language::Unknown {
                "plain text".to_string()
            } else {
                crate::code::language::human_name(example.language)
            }
        );
        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let bands = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(6),
                Constraint::Length(6),
                Constraint::Length(1),
            ])
            .split(inner);
        frame.render_widget(Paragraph::new(INTRO), bands[0]);
        // Wide enough for the longest name on one row, so a list line is a screen line and the
        // selection can be scrolled into view.
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(LIST_WIDTH.min(inner.width / 2)),
                Constraint::Min(20),
            ])
            .split(bands[1]);

        let row = self.row();
        let mark = |line: Line<'static>, this: Row| {
            if this == row { line.reversed() } else { line }
        };
        let values = options.options();
        let mut list: Vec<Line> = vec!["Preset".bold().into()];
        let mut selected_line = 0;
        let mut push = |list: &mut Vec<Line>, text: String, this: Row| {
            if this == row {
                selected_line = list.len();
            }
            list.push(mark(text.into(), this));
        };
        for (index, &(name, preset, _)) in PRESETS.iter().enumerate() {
            let dot = if preset == options { '•' } else { ' ' };
            push(&mut list, format!("({dot}) {name}"), Row::Preset(index));
        }
        for (title, members) in GROUPS {
            list.push(Line::default());
            list.push(title.bold().into());
            for index in members {
                let check = if values[index].1 { 'x' } else { ' ' };
                let text = format!("[{check}] {}", OPTION_INFO[index].name);
                push(&mut list, text, Row::Setting(index));
            }
        }
        // The node-highlight row in a fixed area at the bottom of the column, not after the
        // checkboxes: on a short terminal they would push it out of sight.
        let node = format!(
            "[{}] Node highlight",
            if self.node_highlight { 'x' } else { ' ' }
        );
        let node_line = mark(node.into(), Row::NodeHighlight);
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(columns[0]);
        let list_scroll = (selected_line + 1).saturating_sub(left[0].height as usize) as u16;
        frame.render_widget(
            Paragraph::new(list)
                .scroll((list_scroll, 0))
                .block(Block::default().borders(Borders::RIGHT)),
            left[0],
        );
        frame.render_widget(
            Paragraph::new(node_line).block(
                Block::default()
                    .borders(Borders::TOP | Borders::RIGHT)
                    .title(" Cursor ".dim()),
            ),
            left[1],
        );

        let view = self.view();
        let note = view.and_then(|view| view.note.clone());
        if let Some(view) = view {
            let sides = Layout::default()
                .direction(if columns[1].width >= 100 {
                    Direction::Horizontal
                } else {
                    Direction::Vertical
                })
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(columns[1]);
            for (side, title) in [(0, " Before "), (1, " After ")] {
                let lines = example_lines(
                    &view.texts[side],
                    &view.painted[side],
                    &view.marked[side],
                    theme,
                );
                // Unwrapped, so a source row is a screen row, scrolled so the row the selected
                // setting changes sits a third of the way down.
                let height = sides[side].height.saturating_sub(2) as usize;
                let row = focus_row(&view.texts[side], &view.painted[side], &view.marked[side]);
                let scroll = row.saturating_sub(height / 3) as u16;
                frame.render_widget(
                    Paragraph::new(lines)
                        .scroll((scroll, 0))
                        .block(Block::default().borders(Borders::BOTTOM).title(title)),
                    sides[side],
                );
            }
        }

        let (name, description, mark_note) = match row {
            Row::NodeHighlight => {
                let text = Self::node_row_text();
                frame.render_widget(
                    Paragraph::new(text)
                        .wrap(Wrap { trim: true })
                        .block(Block::default().borders(Borders::TOP)),
                    bands[2],
                );
                frame.render_widget(Paragraph::new(HINT.dim()), bands[3]);
                return Ok(());
            }
            Row::Preset(index) => (
                PRESETS[index].0,
                PRESETS[index].2,
                "◆ marks the rows that change if you choose it.",
            ),
            Row::Setting(index) => (
                OPTION_INFO[index].name,
                OPTION_INFO[index].description,
                "◆ marks the rows this setting changes in the example.",
            ),
        };
        let text = vec![
            Line::from(name).bold(),
            Line::from(description),
            match note {
                Some(note) => Line::from(note).italic(),
                None => Line::from(mark_note).dim(),
            },
        ];
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::TOP)),
            bands[2],
        );
        frame.render_widget(Paragraph::new(HINT.dim()), bands[3]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    /// Moves the selection down to `row`.
    fn select(dialog: &mut RenderOptionsDialog, row: Row) {
        while dialog.row() != row {
            assert!(
                dialog.selected + 1 < dialog.row_count(),
                "{row:?} is below the cursor"
            );
            dialog.handle_key_event(key(KeyCode::Down)).unwrap();
        }
    }

    #[test]
    fn space_toggles_the_selected_option_and_reports_it() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::MINIMAL);
        select(&mut dialog, Row::Setting(0));

        let action = dialog.handle_key_event(key(KeyCode::Char(' '))).unwrap();

        assert_eq!(
            action,
            Some(Action::RenderOptionsChanged(RenderOptions {
                leading_whitespace: true,
                structural_punctuation: false,
                whole_pair_updates: false,
                paint_reindent_only_moves: false,
                paint_displaced_moves: false,
                paint_resized_moves: false,
                whole_identifier_updates: false,
                interior_whitespace: false,
                changed_trailing_whitespace: false,
            }))
        );
    }

    #[test]
    fn space_toggles_a_setting_in_a_later_group() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::MINIMAL);
        select(&mut dialog, Row::Setting(1));

        let action = dialog.handle_key_event(key(KeyCode::Char(' '))).unwrap();

        assert_eq!(
            action,
            Some(Action::RenderOptionsChanged(RenderOptions {
                leading_whitespace: false,
                structural_punctuation: true,
                whole_pair_updates: false,
                paint_reindent_only_moves: false,
                paint_displaced_moves: false,
                paint_resized_moves: false,
                whole_identifier_updates: false,
                interior_whitespace: false,
                changed_trailing_whitespace: false,
            }))
        );
    }

    #[test]
    fn digits_jump_straight_to_the_named_presets() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::EXTENDED);

        let to_minimal = dialog.handle_key_event(key(KeyCode::Char('1'))).unwrap();
        assert_eq!(
            to_minimal,
            Some(Action::RenderOptionsChanged(RenderOptions::MINIMAL))
        );

        let to_extended = dialog.handle_key_event(key(KeyCode::Char('2'))).unwrap();
        assert_eq!(
            to_extended,
            Some(Action::RenderOptionsChanged(RenderOptions::EXTENDED))
        );
    }

    /// Every toggle has already applied and persisted itself, so closing has nothing left to
    /// change - and nothing to revert: `Esc` is not a cancel here.
    #[test]
    fn enter_and_esc_both_close_and_keep_every_toggle() {
        for close in [KeyCode::Enter, KeyCode::Esc] {
            let mut dialog = RenderOptionsDialog::new(RenderOptions::MINIMAL);
            select(&mut dialog, Row::Setting(0));
            dialog.handle_key_event(key(KeyCode::Char(' '))).unwrap();
            let toggled = dialog.options;

            let action = dialog.handle_key_event(key(close)).unwrap();

            assert_eq!(action, Some(Action::SettingsClosed), "{close:?}");
            assert_eq!(
                dialog.options, toggled,
                "{close:?} must not edit the options it closes on"
            );
        }
    }

    /// Bound to MINIMAL, a doubled `M`/`m` would turn every field off and persist it.
    #[test]
    fn the_key_that_opens_the_panel_is_inert_inside_it() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::EXTENDED);

        assert_eq!(
            dialog.handle_key_event(key(KeyCode::Char('m'))).unwrap(),
            None
        );
        assert_eq!(
            dialog.handle_key_event(key(KeyCode::Char('f'))).unwrap(),
            None
        );
        assert_eq!(dialog.options, RenderOptions::EXTENDED);
    }

    /// A preset pressed and then `Esc`: the preset stands, since it was applied and saved when
    /// it was pressed. `Esc` never sends a cancel from this panel.
    #[test]
    fn esc_after_a_preset_keeps_the_preset() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::EXTENDED);

        dialog.handle_key_event(key(KeyCode::Char('1'))).unwrap();
        assert_eq!(dialog.options, RenderOptions::MINIMAL);

        let action = dialog.handle_key_event(key(KeyCode::Esc)).unwrap();

        assert_eq!(action, Some(Action::SettingsClosed));
        assert_eq!(dialog.options, RenderOptions::MINIMAL);
    }

    #[test]
    fn selection_does_not_move_past_the_last_row() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::EXTENDED);
        for _ in 0..dialog.row_count() + 5 {
            dialog.handle_key_event(key(KeyCode::Down)).unwrap();
        }
        assert_eq!(dialog.selected, dialog.row_count() - 1);
    }

    /// The node-highlight row, the last one.
    fn on_the_node_row(dialog: &mut RenderOptionsDialog) {
        for _ in 0..dialog.row_count() {
            dialog.handle_key_event(key(KeyCode::Down)).unwrap();
        }
        assert_eq!(dialog.row(), Row::NodeHighlight);
    }

    #[test]
    fn space_on_the_last_row_switches_the_node_highlight() {
        let mut dialog =
            RenderOptionsDialog::new(RenderOptions::EXTENDED).with_node_highlight(false);
        on_the_node_row(&mut dialog);
        assert_eq!(
            dialog.handle_key_event(key(KeyCode::Char(' '))).unwrap(),
            Some(Action::NodeHighlightChanged(true))
        );
        assert_eq!(
            dialog.handle_key_event(key(KeyCode::Char(' '))).unwrap(),
            Some(Action::NodeHighlightChanged(false))
        );
        assert_eq!(
            dialog.options,
            RenderOptions::EXTENDED,
            "no rendering option moved"
        );
    }

    /// Selected, the row explains itself and marks nothing in the example.
    #[test]
    fn the_node_row_explains_itself() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::EXTENDED);
        on_the_node_row(&mut dialog);
        let text = screen(&mut dialog, 160, 40);
        assert!(text.contains("[ ] Node highlight"), "{text}");
        assert!(text.contains("Also the H key"), "{text}");
        assert!(!text.contains("◆"), "{text}");
    }

    /// The page as text, drawn at `width` by `height`.
    fn screen(dialog: &mut RenderOptionsDialog, width: u16, height: u16) -> String {
        let backend = ratatui::backend::TestBackend::new(width, height);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| dialog.draw(frame, frame.area()).unwrap())
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol().to_string())
                    .collect::<String>()
                    + "\n"
            })
            .collect()
    }

    #[test]
    fn the_page_is_the_whole_screen() {
        let dialog = RenderOptionsDialog::new(RenderOptions::EXTENDED);
        let area = Rect::new(0, 0, 200, 50);
        assert_eq!(dialog.popup_area(area), area);
    }

    /// The selected setting's effect is marked in the example, and named in words below it.
    #[test]
    fn the_selected_settings_effect_is_marked_in_the_example() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::EXTENDED);
        select(&mut dialog, Row::Setting(0));
        let text = screen(&mut dialog, 160, 40);
        assert!(text.contains("example in Rust"), "{text}");
        assert!(text.contains("◆"), "{text}");
        assert!(text.contains(OPTION_INFO[0].name), "{text}");
        assert!(text.contains("fn perimeter"), "{text}");
    }

    /// From Minimal, a re-indented block shows as moved only once displaced code does: the page
    /// says which setting to turn on rather than showing nothing.
    #[test]
    fn a_setting_with_nothing_to_show_names_the_one_it_needs() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::MINIMAL);
        select(&mut dialog, Row::Setting(3));
        let text = screen(&mut dialog, 160, 40);
        assert!(text.contains("turn on"), "{text}");
    }

    /// The page opens on the preset in use, says what the page is for, and lists the presets
    /// before any setting.
    #[test]
    fn the_presets_come_first_and_the_page_opens_on_the_one_in_use() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::EXTENDED);
        assert_eq!(dialog.row(), Row::Preset(1));
        let text = screen(&mut dialog, 160, 40);
        assert!(text.contains(INTRO), "{text}");
        assert!(text.contains("(•) Extended"), "{text}");
        assert!(text.contains("This is the current choice."), "{text}");
        let minimal = text.find("( ) Minimal").expect("the Minimal row");
        assert!(minimal < text.find(OPTION_INFO[0].name).unwrap(), "{text}");

        let mut custom = RenderOptions::EXTENDED;
        custom.toggle(0);
        let dialog = RenderOptionsDialog::new(custom);
        assert_eq!(dialog.row(), Row::Preset(0), "no preset in use: the first");
    }

    /// Space on a preset row applies it, and the example marks what choosing it would change.
    #[test]
    fn space_on_a_preset_row_applies_it() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::EXTENDED);
        dialog.handle_key_event(key(KeyCode::Up)).unwrap();
        assert_eq!(dialog.row(), Row::Preset(0));
        let text = screen(&mut dialog, 160, 40);
        assert!(text.contains("◆"), "{text}");
        assert_eq!(
            dialog.handle_key_event(key(KeyCode::Char(' '))).unwrap(),
            Some(Action::RenderOptionsChanged(RenderOptions::MINIMAL))
        );
    }

    /// Every setting is listed once, under one group.
    #[test]
    fn the_groups_list_every_setting_once() {
        let mut listed: Vec<usize> = GROUPS.iter().flat_map(|(_, members)| *members).collect();
        listed.sort_unstable();
        assert_eq!(listed, (0..OPTION_INFO.len()).collect::<Vec<_>>());
    }

    /// A name with its checkbox fits the column on one row, so the list can scroll by rows.
    #[test]
    fn every_name_fits_the_settings_column() {
        for info in OPTION_INFO {
            let row = format!("[x] {}", info.name);
            // One column for the border.
            assert!(row.chars().count() < LIST_WIDTH as usize, "{row}");
        }
    }

    /// On a short terminal the list scrolls to keep the selected row in sight.
    #[test]
    fn the_selected_row_stays_in_sight_on_a_short_terminal() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::EXTENDED);
        select(&mut dialog, Row::Setting(5));
        let text = screen(&mut dialog, 120, 24);
        // Its checkbox row, not the description below, which names it too.
        assert!(
            text.contains(&format!("[x] {}", OPTION_INFO[5].name)),
            "{text}"
        );
    }

    #[test]
    fn non_code_content_says_there_is_nothing_to_set() {
        let mut dialog = RenderOptionsDialog::for_content(
            RenderOptions::EXTENDED,
            PageContent::NonCode,
            OverlayTheme::default(),
        );
        let text = screen(&mut dialog, 120, 30);
        assert!(
            text.contains("No settings at this time for non-code content."),
            "{text}"
        );
        assert_eq!(
            dialog.handle_key_event(key(KeyCode::Char(' '))).unwrap(),
            None
        );
        assert_eq!(
            dialog.handle_key_event(key(KeyCode::Esc)).unwrap(),
            Some(Action::SettingsClosed)
        );
    }
}
