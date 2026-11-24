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
//! One file's diff as the viewer shows it, for another ratatui application to host: the two
//! panels (or one, on a narrow area), the minimap, the change navigation and the search, drawn
//! into whatever [`Rect`] the host gives it and driven by the keys the host passes on.
//!
//! The host keeps its own event loop, its own quit and its own idea of the next file: a key the
//! view has no use for comes back as [`Outcome::Ignored`]. Nothing here owns the terminal.
//!
//! ```no_run
//! use omnidiff::tui::embed::{DiffView, Outcome};
//!
//! let mut view = DiffView::from_texts(
//!     "src/lib.rs".as_ref(), "fn a() {}\n",
//!     "src/lib.rs".as_ref(), "fn a() { b() }\n",
//! ).unwrap();
//! // in the host's draw:   view.render(frame, area);
//! // in the host's input:  if view.handle_key(key) == Outcome::Ignored { /* the host's own keys */ }
//! ```

use std::path::Path;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::diff::text::RenderOptions;
use crate::library::Side;
use crate::tui::components::Component;
use crate::tui::components::diff_viewer::{DiffViewer, Panel};
pub use crate::tui::theme::OverlayTheme;

/// What became of a key passed to [`DiffView::handle_key`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The view used it; draw again.
    Handled,
    /// The view has no use for it; it is the host's.
    Ignored,
}

/// One file's diff, ready to draw. See the [module](self).
pub struct DiffView {
    viewer: DiffViewer,
    structural: bool,
}

impl DiffView {
    /// Diffs two texts, each named by a path that picks its language and its title; the paths
    /// need not exist. The cursor starts on the first change.
    pub fn from_texts(
        before_path: &Path,
        before_text: &str,
        after_path: &Path,
        after_text: &str,
    ) -> Result<Self> {
        let (data, _) = crate::tui::app::compute_diff_of_texts(
            before_path,
            before_text,
            after_path,
            after_text,
            RenderOptions::MINIMAL,
        )?;
        let mut viewer = DiffViewer::new();
        viewer.set_render_options(RenderOptions::MINIMAL);
        viewer.load_diff(&data);
        Ok(Self {
            viewer,
            structural: !data.plain_text_fallback,
        })
    }

    /// Whether both sides were parsed and matched node by node, rather than line by line.
    pub fn is_structural(&self) -> bool {
        self.structural
    }

    /// Draws the view into `area`, and puts the terminal's cursor on the view's.
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Drawing cannot fail but for the trait's signature; nothing is left half drawn.
        let _ = self.viewer.draw(frame, area);
    }

    /// Moves, searches or switches layout by the viewer's own keys: `j`/`k` and the arrows,
    /// `n`/`p` from change to change, Enter to a change's counterpart, `v` for the layout, Tab
    /// for the other panel, Ctrl-d/u/e/y, Page Up/Down, Home/End. Anything else, `q` and `/`
    /// among them, is [`Outcome::Ignored`] and the host's.
    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
            return Outcome::Ignored;
        }
        match self.viewer.handle_key_event(key) {
            Ok(Some(_)) => Outcome::Handled,
            _ => Outcome::Ignored,
        }
    }

    /// Searches the focused panel for `query`, as `/` does in the viewer, and goes to the first
    /// match; returns how many there are.
    pub fn search(&mut self, query: &str) -> usize {
        self.viewer.search(query);
        self.viewer
            .focused_search_match_count_and_index()
            .map_or(0, |(count, _)| count)
    }

    /// The next or previous match of the last search.
    pub fn next_match(&mut self, forward: bool) {
        self.viewer.jump_to_search_match(forward);
    }

    /// The side the cursor is on, and its 0-based row there: where a comment the host takes
    /// would go.
    pub fn cursor(&self) -> Option<(Side, usize)> {
        let (row, _) = self.viewer.focused_cursor_position()?;
        let side = match self.viewer.active_panel() {
            Panel::Before => Side::Before,
            Panel::After => Side::After,
        };
        Some((side, row))
    }

    /// Gives `side` the cursor: the panel shown, when only one fits.
    pub fn focus_side(&mut self, side: Side) {
        let wanted = match side {
            Side::Before => Panel::Before,
            Side::After => Panel::After,
        };
        if self.viewer.active_panel() != wanted {
            self.viewer.toggle_active_panel();
        }
    }

    /// Puts the cursor on `row` of the after side, 0-based, scrolled into view.
    pub fn go_to_after_row(&mut self, row: usize) {
        self.viewer.restore_cursor(Panel::After, row, 0);
    }

    /// The palette of the change overlay and the cursor, to sit with the host's colours.
    pub fn set_overlay_theme(&mut self, theme: OverlayTheme) {
        self.viewer.set_overlay_theme(theme);
    }

    /// The syntax highlighting scheme, by its syntect name.
    pub fn set_syntax_theme(&mut self, name: &str) {
        self.viewer.set_syntax_theme(name.to_string());
    }

    /// Where the change it is on is among all of them, 1-based, and how many there are.
    pub fn change_position(&self) -> Option<(usize, usize)> {
        // Despite its name, it is `(1-based index, total)`.
        self.viewer.merged_change_count_and_index()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use ratatui::{Terminal, backend::TestBackend};

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn a_hosted_view_draws_into_its_area_and_leaves_the_host_its_keys() {
        let mut view = DiffView::from_texts(
            "lib.rs".as_ref(),
            "fn a() {}\n\nfn b() {}\n",
            "lib.rs".as_ref(),
            "fn a() { c() }\n\nfn b() {}\nfn d() {}\n",
        )
        .unwrap();
        assert!(view.is_structural());
        let mut terminal = Terminal::new(TestBackend::new(100, 12)).unwrap();
        // Drawn into the lower part only, as a host would below its own header.
        terminal
            .draw(|f| view.render(f, Rect::new(0, 2, 100, 10)))
            .unwrap();
        let screen: String = (0..12)
            .map(|y| {
                (0..100)
                    .map(|x| terminal.backend().buffer()[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        let rows: Vec<&str> = screen.lines().collect();
        assert!(
            rows[0].trim().is_empty() && rows[1].trim().is_empty(),
            "{screen}"
        );
        // A hundred columns is one panel at a time: the before side, to begin with.
        assert!(
            rows[2].contains("Before") && screen.contains("fn a() {}"),
            "{screen}"
        );

        assert_eq!(view.handle_key(key('n')), Outcome::Handled);
        assert_eq!(
            view.handle_key(key('q')),
            Outcome::Ignored,
            "q is the host's"
        );
        assert_eq!(
            view.handle_key(key('/')),
            Outcome::Ignored,
            "and so is search's prompt"
        );
        let (at, of) = view.change_position().expect("a change");
        assert!(1 <= at && at <= of, "change {at} of {of}");
        view.go_to_after_row(3);
        assert_eq!(view.cursor(), Some((Side::After, 3)));
    }
}
