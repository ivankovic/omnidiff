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

const HINT: &str = "↑/↓ move  Space toggle  1 Minimal  2 Full  Enter or Esc close";

/// What the page is about: an example in the open diff's language, or non-code content, which
/// these settings do not shape.
#[derive(Debug, Clone, Copy)]
pub enum PageContent {
    Example(&'static SettingsExample),
    NonCode,
}

/// The `M` key's settings page, full screen: one checkbox row per [`RenderOptions`] field, an
/// example diff in the open diff's language painted under the current settings, and what the
/// selected setting does - with the bytes it changes in the example underlined and marked in the
/// gutter, so a setting is seen rather than described.
///
/// Every toggle applies and persists the moment it is pressed, and the diff behind the page shows
/// it. There is nothing to accept or cancel, so `Enter` and `Esc` both just close. The presets are
/// on `1`/`2`, not `m`/`f`: the page opens on `M`, and a doubled opening key must not wipe every
/// field and persist that.
pub struct RenderOptionsDialog {
    options: RenderOptions,
    selected: usize,
    content: PageContent,
    theme: OverlayTheme,
    /// The example painted for `(options, selected)`; rebuilt when either changes.
    view: Option<ExampleView>,
}

/// The example under one choice of settings, and what the selected setting changes in it.
struct ExampleView {
    options: RenderOptions,
    selected: usize,
    painted: Painted,
    texts: [String; 2],
    /// Per side, the bytes the selected setting repaints.
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

    pub fn for_content(options: RenderOptions, content: PageContent, theme: OverlayTheme) -> Self {
        Self {
            options,
            selected: 0,
            content,
            theme,
            view: None,
        }
    }

    fn row_count(&self) -> usize {
        self.options.options().len()
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
        let fresh = self
            .view
            .as_ref()
            .is_some_and(|view| view.options == self.options && view.selected == self.selected);
        if !fresh {
            self.view = build_view(example, self.options, self.selected);
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

/// `example` painted under `options`, the bytes `selected` repaints when flipped, and what to
/// say when it repaints nothing: the example's own reason, or another setting it needs.
fn build_view(
    example: &'static SettingsExample,
    options: RenderOptions,
    selected: usize,
) -> Option<ExampleView> {
    let (now, texts) = painted(example, options)?;
    let flipped = |mut options: RenderOptions, index: usize| {
        options.toggle(index);
        options
    };
    let (other, _) = painted(example, flipped(options, selected))?;
    let marked: [Vec<bool>; 2] = [0, 1].map(|side| {
        now[side]
            .iter()
            .zip(&other[side])
            .map(|(a, b)| a != b)
            .collect()
    });
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
        selected,
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
    } else if options == RenderOptions::FULL {
        "Full"
    } else {
        "custom"
    }
}

/// One side of the example as lines: each byte styled by the operation painting it, the bytes
/// the selected setting changes underlined, and a `◆` in the gutter of every row holding one.
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
            style = style.add_modifier(Modifier::UNDERLINED | Modifier::BOLD);
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
                KeyCode::Enter | KeyCode::Esc => Some(Action::RenderOptionsAccepted),
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
            KeyCode::Char(' ') => {
                self.options.toggle(self.selected);
                Ok(Some(Action::RenderOptionsChanged(self.options)))
            }
            KeyCode::Enter | KeyCode::Esc => Ok(Some(Action::RenderOptionsAccepted)),
            KeyCode::Char('1') => {
                self.options = RenderOptions::MINIMAL;
                Ok(Some(Action::RenderOptionsChanged(self.options)))
            }
            KeyCode::Char('2') => {
                self.options = RenderOptions::FULL;
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
        let (options, selected) = (self.options, self.selected);
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
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(6),
                Constraint::Length(6),
                Constraint::Length(1),
            ])
            .split(inner);
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(46.min(inner.width / 3)),
                Constraint::Min(20),
            ])
            .split(rows[0]);

        let list: Vec<Line> = options
            .options()
            .into_iter()
            .enumerate()
            .map(|(index, (name, on))| {
                let text = format!("[{}] {name}", if on { 'x' } else { ' ' });
                if index == selected {
                    Line::from(text).reversed()
                } else {
                    Line::from(text)
                }
            })
            .collect();
        frame.render_widget(
            Paragraph::new(list)
                .wrap(Wrap { trim: false })
                .block(Block::default().borders(Borders::RIGHT)),
            columns[0],
        );

        let view = self.view();
        let (description, note) = (
            OPTION_INFO[selected].description,
            view.and_then(|view| view.note.clone()),
        );
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

        let mut text = vec![
            Line::from(OPTION_INFO[selected].name).bold(),
            Line::from(description),
        ];
        text.push(match note {
            Some(note) => Line::from(note).italic(),
            None => Line::from("◆ Underlined: what this setting changes in the example.").dim(),
        });
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::TOP)),
            rows[1],
        );
        frame.render_widget(Paragraph::new(HINT.dim()), rows[2]);
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

    #[test]
    fn space_toggles_the_selected_option_and_reports_it() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::MINIMAL);

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
            }))
        );
    }

    #[test]
    fn down_then_space_toggles_the_second_option() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::MINIMAL);
        dialog.handle_key_event(key(KeyCode::Down)).unwrap();

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
            }))
        );
    }

    #[test]
    fn digits_jump_straight_to_the_named_presets() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::FULL);

        let to_minimal = dialog.handle_key_event(key(KeyCode::Char('1'))).unwrap();
        assert_eq!(
            to_minimal,
            Some(Action::RenderOptionsChanged(RenderOptions::MINIMAL))
        );

        let to_full = dialog.handle_key_event(key(KeyCode::Char('2'))).unwrap();
        assert_eq!(
            to_full,
            Some(Action::RenderOptionsChanged(RenderOptions::FULL))
        );
    }

    /// Every toggle has already applied and persisted itself, so closing has nothing left to
    /// change - and nothing to revert: `Esc` is not a cancel here.
    #[test]
    fn enter_and_esc_both_close_and_keep_every_toggle() {
        for close in [KeyCode::Enter, KeyCode::Esc] {
            let mut dialog = RenderOptionsDialog::new(RenderOptions::MINIMAL);
            dialog.handle_key_event(key(KeyCode::Char(' '))).unwrap();
            let toggled = dialog.options;

            let action = dialog.handle_key_event(key(close)).unwrap();

            assert_eq!(action, Some(Action::RenderOptionsAccepted), "{close:?}");
            assert_eq!(
                dialog.options, toggled,
                "{close:?} must not edit the options it closes on"
            );
        }
    }

    /// Bound to MINIMAL, a doubled `M`/`m` would turn every field off and persist it.
    #[test]
    fn the_key_that_opens_the_panel_is_inert_inside_it() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::FULL);

        assert_eq!(
            dialog.handle_key_event(key(KeyCode::Char('m'))).unwrap(),
            None
        );
        assert_eq!(
            dialog.handle_key_event(key(KeyCode::Char('f'))).unwrap(),
            None
        );
        assert_eq!(dialog.options, RenderOptions::FULL);
    }

    /// A preset pressed and then `Esc`: the preset stands, since it was applied and saved when
    /// it was pressed. `Esc` never sends a cancel from this panel.
    #[test]
    fn esc_after_a_preset_keeps_the_preset() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::FULL);

        dialog.handle_key_event(key(KeyCode::Char('1'))).unwrap();
        assert_eq!(dialog.options, RenderOptions::MINIMAL);

        let action = dialog.handle_key_event(key(KeyCode::Esc)).unwrap();

        assert_eq!(action, Some(Action::RenderOptionsAccepted));
        assert_eq!(dialog.options, RenderOptions::MINIMAL);
    }

    #[test]
    fn selection_does_not_move_past_the_last_row() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::FULL);
        for _ in 0..10 {
            dialog.handle_key_event(key(KeyCode::Down)).unwrap();
        }
        assert_eq!(dialog.selected, dialog.row_count() - 1);
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
        let dialog = RenderOptionsDialog::new(RenderOptions::FULL);
        let area = Rect::new(0, 0, 200, 50);
        assert_eq!(dialog.popup_area(area), area);
    }

    /// The selected setting's effect is marked in the example, and named in words below it.
    #[test]
    fn the_selected_settings_effect_is_marked_in_the_example() {
        let mut dialog = RenderOptionsDialog::new(RenderOptions::FULL);
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
        for _ in 0..3 {
            dialog.handle_key_event(key(KeyCode::Down)).unwrap();
        }
        let text = screen(&mut dialog, 160, 40);
        assert!(text.contains("turn on"), "{text}");
    }

    #[test]
    fn non_code_content_says_there_is_nothing_to_set() {
        let mut dialog = RenderOptionsDialog::for_content(
            RenderOptions::FULL,
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
            Some(Action::RenderOptionsAccepted)
        );
    }
}
