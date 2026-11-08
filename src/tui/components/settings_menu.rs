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
//! The settings menu: every viewer setting on one full-screen page, in three tabs.
//!
//! * **Highlighting** - what a diff paints ([`RenderOptionsDialog`]), with a live example, and the
//!   node highlight (the `H` key). The `M` key opens the menu here.
//! * **Layout** - the panel layout (the `v` key) and the width from which `auto` shows two panels
//!   ([`LayoutSettings`]).
//! * **Colors** - the overlay and syntax themes, syntax highlighting on or off (the `S` key), and
//!   the overlay colors ([`ThemeDialog`]). The `c` key opens the menu here.
//!
//! One rule for all of it: a change applies and is saved the moment it is made, so there is
//! nothing to accept or cancel, and `Enter` and `Esc` close the menu from any tab. (Syntax
//! highlighting on or off lasts for the run, as `S` does.) The one thing that can be abandoned is
//! a hex color being typed, whose own `Esc` cancels just that edit. `Tab` and `Shift-Tab` switch
//! tabs; inside a tab `↑`/`↓`, `←`/`→` and `Space` keep their meaning, which is why tabs are not on
//! the arrows.
//!
//! The menu replaced two screens, the `c` theme popup (which previewed and reverted on `Esc`) and
//! the `M` page; the tabs are those two components, unchanged in what they edit, plus the Layout
//! tab, which is new.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use super::{
    Component, move_selection, render_options_dialog::RenderOptionsDialog,
    theme_dialog::ThemeDialog,
};
use crate::tui::actions::Action;
use crate::tui::theme::{
    DEFAULT_WIDE_COLUMNS, PanelLayout, WIDE_COLUMNS_ENV, WIDE_COLUMNS_RANGE, WideColumnsSource,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Highlighting,
    Layout,
    Colors,
}

impl SettingsTab {
    const ALL: [SettingsTab; 3] = [
        SettingsTab::Highlighting,
        SettingsTab::Layout,
        SettingsTab::Colors,
    ];

    fn label(self) -> &'static str {
        match self {
            SettingsTab::Highlighting => "Highlighting",
            SettingsTab::Layout => "Layout",
            SettingsTab::Colors => "Colors",
        }
    }

    /// The tab `delta` steps along, wrapping.
    fn step(self, delta: isize) -> Self {
        let at = Self::ALL.iter().position(|&tab| tab == self).unwrap_or(0) as isize;
        let count = Self::ALL.len() as isize;
        Self::ALL[(at + delta).rem_euclid(count) as usize]
    }
}

pub struct SettingsMenu {
    tab: SettingsTab,
    highlighting: RenderOptionsDialog,
    layout: LayoutSettings,
    colors: ThemeDialog,
}

impl SettingsMenu {
    pub fn new(
        tab: SettingsTab,
        highlighting: RenderOptionsDialog,
        layout: LayoutSettings,
        colors: ThemeDialog,
    ) -> Self {
        Self {
            tab,
            highlighting,
            layout,
            colors,
        }
    }

    pub fn tab(&self) -> SettingsTab {
        self.tab
    }

    /// The Highlighting tab's example is drawn in the overlay theme, which the Colors tab changes.
    pub fn set_overlay_theme(&mut self, theme: crate::tui::theme::OverlayTheme) {
        self.highlighting.set_theme(theme);
    }

    fn tab_bar(&self) -> Line<'static> {
        let mut spans = vec![Span::raw(" Settings ").bold(), Span::raw(" ")];
        for tab in SettingsTab::ALL {
            let label = format!(" {} ", tab.label());
            spans.push(if tab == self.tab {
                Span::raw(label).reversed().bold()
            } else {
                Span::raw(label)
            });
            spans.push(Span::raw(" "));
        }
        spans.push(Span::raw("  Tab / Shift-Tab switch tabs").dim());
        Line::from(spans)
    }
}

impl Component for SettingsMenu {
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        // A hex value being typed takes every key, the menu's own included.
        if self.tab == SettingsTab::Colors && self.colors.is_editing() {
            return self.colors.handle_key_event(key);
        }
        match key.code {
            KeyCode::Tab => {
                self.tab = self.tab.step(1);
                Ok(Some(Action::Render))
            }
            KeyCode::BackTab => {
                self.tab = self.tab.step(-1);
                Ok(Some(Action::Render))
            }
            _ => match self.tab {
                SettingsTab::Highlighting => self.highlighting.handle_key_event(key),
                SettingsTab::Layout => self.layout.handle_key_event(key),
                SettingsTab::Colors => self.colors.handle_key_event(key),
            },
        }
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        frame.render_widget(Clear, area);
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(area);
        frame.render_widget(Paragraph::new(self.tab_bar()), rows[0]);
        let body = rows[1];
        match self.tab {
            SettingsTab::Highlighting => self.highlighting.draw(frame, body),
            SettingsTab::Layout => self.layout.draw(frame, body),
            SettingsTab::Colors => {
                let popup = self.colors.popup_area(body);
                self.colors.draw(frame, popup)
            }
        }
    }
}

/// How far one `←`/`→` moves the width row.
const WIDE_COLUMNS_STEP: u16 = 10;

const LAYOUT_ROW: usize = 0;
const WIDE_ROW: usize = 1;

/// The Layout tab: the panel layout (the `v` key) and the width from which `auto` shows two panels
/// (issue #70), each changed with `←`/`→` and saved at once.
///
/// A width from `$OMNIDIFF_WIDE_COLUMNS` overrides the config the row writes, so the row then only
/// says so rather than changing a value nothing would read.
pub struct LayoutSettings {
    layout: PanelLayout,
    wide_columns: u16,
    wide_source: WideColumnsSource,
    selected: usize,
    /// The terminal's width, from the last frame: the menu is the whole screen.
    terminal_width: u16,
}

impl LayoutSettings {
    pub fn new(layout: PanelLayout, wide_columns: u16, wide_source: WideColumnsSource) -> Self {
        Self {
            layout,
            wide_columns,
            wide_source,
            selected: LAYOUT_ROW,
            terminal_width: 0,
        }
    }

    fn step_layout(&mut self, forward: bool) -> Action {
        self.layout = if forward {
            self.layout.next()
        } else {
            self.layout.previous()
        };
        Action::PanelLayoutChanged(self.layout)
    }

    /// A step, held to [`WIDE_COLUMNS_RANGE`]; `None` where it would not move or the environment
    /// sets the width.
    fn step_wide_columns(&mut self, up: bool) -> Option<Action> {
        if self.wide_source == WideColumnsSource::Environment {
            return None;
        }
        let stepped = if up {
            self.wide_columns.saturating_add(WIDE_COLUMNS_STEP)
        } else {
            self.wide_columns.saturating_sub(WIDE_COLUMNS_STEP)
        };
        let clamped = stepped.clamp(*WIDE_COLUMNS_RANGE.start(), *WIDE_COLUMNS_RANGE.end());
        if clamped == self.wide_columns {
            return None;
        }
        self.wide_columns = clamped;
        self.wide_source = WideColumnsSource::Config;
        Some(Action::WideColumnsChanged(clamped))
    }

    /// What `auto` gives on this terminal.
    fn auto_gives(&self) -> &'static str {
        if self.terminal_width >= self.wide_columns {
            "two panels"
        } else {
            "one panel at a time"
        }
    }

    fn explanation(&self) -> Vec<Line<'static>> {
        if self.selected == LAYOUT_ROW {
            return vec![
                Line::from("Panel layout").bold(),
                Line::from(format!(
                    "auto: Before and After side by side from {} columns, one at a time below \
                     that - this terminal is {} columns wide, so {}. dual: always side by side. \
                     single: always one at a time, Tab switches.",
                    self.wide_columns,
                    self.terminal_width,
                    self.auto_gives()
                )),
                Line::from("Also the v key, anywhere in the diff.").italic(),
            ];
        }
        let source = match self.wide_source {
            WideColumnsSource::Environment => format!(
                "Set by {WIDE_COLUMNS_ENV}, which overrides this tab: unset it to change the \
                 width here."
            ),
            WideColumnsSource::Config => "Saved in the config file as `wide_columns`.".to_string(),
            WideColumnsSource::Default => format!(
                "The default, {DEFAULT_WIDE_COLUMNS}: two 80-column panels. {WIDE_COLUMNS_ENV} \
                 overrides it for one run."
            ),
        };
        vec![
            Line::from("Two panels from").bold(),
            Line::from(format!(
                "The auto layout shows Before and After side by side from {} columns, and one at \
                 a time below that. This terminal is {} columns wide: {}.",
                self.wide_columns,
                self.terminal_width,
                self.auto_gives()
            )),
            Line::from(source).italic(),
        ]
    }
}

impl Component for LayoutSettings {
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Up => {
                move_selection(&mut self.selected, -1, 2);
                Ok(Some(Action::Render))
            }
            KeyCode::Down => {
                move_selection(&mut self.selected, 1, 2);
                Ok(Some(Action::Render))
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') => {
                let forward = key.code != KeyCode::Left;
                Ok(match self.selected {
                    LAYOUT_ROW => Some(self.step_layout(forward)),
                    WIDE_ROW if key.code != KeyCode::Char(' ') => self.step_wide_columns(forward),
                    _ => None,
                })
            }
            KeyCode::Enter | KeyCode::Esc => Ok(Some(Action::SettingsClosed)),
            _ => Ok(None),
        }
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        self.terminal_width = area.width;
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Layout settings ");
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(4),
                Constraint::Length(1),
            ])
            .split(inner);
        let row = |index: usize, label: &str, value: String| {
            let line = Line::from(format!("{label:<20}< {value} >"));
            if index == self.selected {
                line.reversed()
            } else {
                line
            }
        };
        let list = vec![
            row(LAYOUT_ROW, "Panel layout", self.layout.label().to_string()),
            row(
                WIDE_ROW,
                "Two panels from",
                format!("{} columns", self.wide_columns),
            ),
        ];
        frame.render_widget(Paragraph::new(list), rows[0]);
        frame.render_widget(
            Paragraph::new(self.explanation())
                .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::TOP)),
            rows[1],
        );
        frame.render_widget(
            Paragraph::new("↑/↓ move  ←/→ change  Tab next tab  Enter or Esc close".dim()),
            rows[2],
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::text::RenderOptions;
    use crate::tui::theme::OverlayTheme;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn menu(tab: SettingsTab) -> SettingsMenu {
        SettingsMenu::new(
            tab,
            RenderOptionsDialog::new(RenderOptions::FULL),
            LayoutSettings::new(
                PanelLayout::Auto,
                DEFAULT_WIDE_COLUMNS,
                WideColumnsSource::Default,
            ),
            ThemeDialog::new(OverlayTheme::Dracula),
        )
    }

    fn screen(menu: &mut SettingsMenu, width: u16, height: u16) -> String {
        let backend = ratatui::backend::TestBackend::new(width, height);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| menu.draw(frame, frame.area()).unwrap())
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
    fn tab_and_shift_tab_walk_the_tabs_and_wrap() {
        let mut menu = menu(SettingsTab::Highlighting);
        menu.handle_key_event(key(KeyCode::Tab)).unwrap();
        assert_eq!(menu.tab(), SettingsTab::Layout);
        menu.handle_key_event(key(KeyCode::Tab)).unwrap();
        assert_eq!(menu.tab(), SettingsTab::Colors);
        menu.handle_key_event(key(KeyCode::Tab)).unwrap();
        assert_eq!(menu.tab(), SettingsTab::Highlighting);
        menu.handle_key_event(key(KeyCode::BackTab)).unwrap();
        assert_eq!(menu.tab(), SettingsTab::Colors);
    }

    #[test]
    fn enter_and_esc_close_the_menu_from_every_tab() {
        for tab in SettingsTab::ALL {
            for close in [KeyCode::Enter, KeyCode::Esc] {
                let mut menu = menu(tab);
                assert_eq!(
                    menu.handle_key_event(key(close)).unwrap(),
                    Some(Action::SettingsClosed),
                    "{close:?} on {tab:?}"
                );
            }
        }
    }

    /// Tab while typing a color is not the menu's: the edit keeps every key until it ends.
    #[test]
    fn a_color_being_typed_keeps_tab_and_esc() {
        let mut menu = menu(SettingsTab::Colors);
        for _ in 0..3 {
            menu.handle_key_event(key(KeyCode::Down)).unwrap();
        }
        menu.handle_key_event(key(KeyCode::Enter)).unwrap();
        assert!(menu.colors.is_editing());
        menu.handle_key_event(key(KeyCode::Tab)).unwrap();
        assert_eq!(menu.tab(), SettingsTab::Colors);
        assert_eq!(
            menu.handle_key_event(key(KeyCode::Esc)).unwrap(),
            Some(Action::Render),
            "Esc cancels the edit, not the menu"
        );
        assert!(!menu.colors.is_editing());
    }

    #[test]
    fn each_tab_draws_its_own_settings_under_the_tab_bar() {
        let mut highlighting = menu(SettingsTab::Highlighting);
        let text = screen(&mut highlighting, 160, 40);
        assert!(text.contains(" Highlighting "), "{text}");
        assert!(text.contains("Node highlight"), "{text}");

        let mut layout = menu(SettingsTab::Layout);
        let text = screen(&mut layout, 160, 40);
        assert!(text.contains("Panel layout"), "{text}");
        assert!(text.contains("Two panels from"), "{text}");

        let mut colors = menu(SettingsTab::Colors);
        let text = screen(&mut colors, 160, 40);
        assert!(text.contains("Syntax theme:"), "{text}");
        assert!(text.contains("Tab / Shift-Tab switch tabs"), "{text}");
    }

    fn layout(source: WideColumnsSource, columns: u16) -> LayoutSettings {
        LayoutSettings::new(PanelLayout::Auto, columns, source)
    }

    #[test]
    fn the_layout_row_cycles_both_ways() {
        let mut tab = layout(WideColumnsSource::Default, 160);
        assert_eq!(
            tab.handle_key_event(key(KeyCode::Right)).unwrap(),
            Some(Action::PanelLayoutChanged(PanelLayout::Dual))
        );
        assert_eq!(
            tab.handle_key_event(key(KeyCode::Left)).unwrap(),
            Some(Action::PanelLayoutChanged(PanelLayout::Auto))
        );
        assert_eq!(
            tab.handle_key_event(key(KeyCode::Left)).unwrap(),
            Some(Action::PanelLayoutChanged(PanelLayout::Single))
        );
    }

    #[test]
    fn the_width_row_steps_by_ten_within_range() {
        let mut tab = layout(WideColumnsSource::Default, 160);
        tab.handle_key_event(key(KeyCode::Down)).unwrap();
        assert_eq!(
            tab.handle_key_event(key(KeyCode::Right)).unwrap(),
            Some(Action::WideColumnsChanged(170))
        );
        assert_eq!(
            tab.handle_key_event(key(KeyCode::Left)).unwrap(),
            Some(Action::WideColumnsChanged(160))
        );
        let mut low = layout(WideColumnsSource::Config, 40);
        low.handle_key_event(key(KeyCode::Down)).unwrap();
        assert_eq!(low.handle_key_event(key(KeyCode::Left)).unwrap(), None);
        let mut high = layout(WideColumnsSource::Config, 1000);
        high.handle_key_event(key(KeyCode::Down)).unwrap();
        assert_eq!(high.handle_key_event(key(KeyCode::Right)).unwrap(), None);
    }

    /// The environment overrides the config, so the row the config backs cannot change it.
    #[test]
    fn a_width_from_the_environment_is_not_the_rows_to_change() {
        let mut tab = layout(WideColumnsSource::Environment, 200);
        tab.handle_key_event(key(KeyCode::Down)).unwrap();
        assert_eq!(tab.handle_key_event(key(KeyCode::Right)).unwrap(), None);
        let mut menu = menu(SettingsTab::Layout);
        menu.layout = tab;
        let text = screen(&mut menu, 160, 30);
        assert!(text.contains("200 columns"), "{text}");
        assert!(text.contains(WIDE_COLUMNS_ENV), "{text}");
    }

    #[test]
    fn the_width_row_says_what_this_terminal_gets() {
        let mut menu = menu(SettingsTab::Layout);
        menu.handle_key_event(key(KeyCode::Down)).unwrap();
        let wide = screen(&mut menu, 170, 30);
        assert!(
            wide.contains("This terminal is 170 columns wide: two panels"),
            "{wide}"
        );
        let narrow = screen(&mut menu, 120, 30);
        assert!(narrow.contains("one panel at a time"), "{narrow}");
    }
}
