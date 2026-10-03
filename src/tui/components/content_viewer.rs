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

//! A pair diffed by content (see [`crate::diff::content`]) on screen: the view its engine's diff
//! gets, behind one interface for the TUI and `human_solver`.

use std::path::Path;

use crossterm::event::KeyCode;
use ratatui::{Frame, layout::Rect};
use ratatui_image::picker::Picker;

use super::picture_viewer::{PictureColors, PictureViewer};
use crate::diff::content::{self, Engine, Verdict};

pub enum ContentViewer {
    Picture(PictureViewer),
}

impl ContentViewer {
    /// The view of `before` and `after` if they are a pair diffed by content that decodes; `None`
    /// otherwise, and the caller reports them as any other binary pair.
    pub fn open(before: &Path, after: &Path, picker: Picker) -> Option<Self> {
        Self::open_with(before, after, picker, false)
    }

    /// [`Self::open`] for a human recording a verdict: nothing the engine decided is shown.
    pub fn open_for_annotation(before: &Path, after: &Path, picker: Picker) -> Option<Self> {
        Self::open_with(before, after, picker, true)
    }

    fn open_with(before: &Path, after: &Path, picker: Picker, annotating: bool) -> Option<Self> {
        let read = |path: &Path| std::fs::read(path).ok();
        match content::pair_kind(&read(before)?, &read(after)?)? {
            Engine::Picture => {
                let viewer = if annotating {
                    PictureViewer::open_for_annotation(before, after, picker)
                } else {
                    PictureViewer::open(before, after, picker)
                };
                viewer.map(ContentViewer::Picture)
            }
        }
    }

    /// Switches to a different drawing protocol (the terminal answered the graphics query).
    pub fn set_picker(&mut self, picker: Picker) {
        match self {
            ContentViewer::Picture(viewer) => viewer.set_picker(picker),
        }
    }

    /// The engine's verdict on the pair.
    pub fn verdict(&self) -> Verdict {
        match self {
            ContentViewer::Picture(viewer) => viewer.verdict(),
        }
    }

    /// Whether the annotation mode hides what the engine decided.
    pub fn annotating(&self) -> bool {
        match self {
            ContentViewer::Picture(viewer) => viewer.annotating(),
        }
    }

    pub fn set_annotating(&mut self, annotating: bool) {
        match self {
            ContentViewer::Picture(viewer) => viewer.set_annotating(annotating),
        }
    }

    /// True while something animates: the caller should draw again soon, not only on a key.
    pub fn is_playing(&self) -> bool {
        match self {
            ContentViewer::Picture(viewer) => viewer.is_playing(),
        }
    }

    /// Handles a key of the view; false for any other key, which it leaves to the caller.
    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        match self {
            ContentViewer::Picture(viewer) => viewer.handle_key(code),
        }
    }

    /// One line for the status bar.
    pub fn status(&self) -> String {
        match self {
            ContentViewer::Picture(viewer) => viewer.status(),
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect, colors: PictureColors) {
        match self {
            ContentViewer::Picture(viewer) => viewer.draw(frame, area, colors),
        }
    }
}
