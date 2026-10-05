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

use super::member_viewer::MemberViewer;
use super::picture_viewer::{self, PictureColors, PictureViewer};
use crate::diff::content::{self, Engine, Verdict};
use crate::diff::picture;

pub enum ContentViewer {
    Picture(PictureViewer),
    Members(MemberViewer),
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
        Self::from_bytes(
            (before.display().to_string(), after.display().to_string()),
            &read(before)?,
            &read(after)?,
            picker,
            annotating,
        )
    }

    /// The view of a pair already read (a container's member), each side titled with its name.
    pub fn from_bytes(
        names: (String, String),
        before: &[u8],
        after: &[u8],
        picker: Picker,
        annotating: bool,
    ) -> Option<Self> {
        match content::pair_kind(before, after)? {
            Engine::Picture => {
                let (before, after) = picture::decode_pair(before, after).ok()?;
                Some(ContentViewer::Picture(PictureViewer::from_decoded(
                    names.0, names.1, before, after, picker, annotating,
                )))
            }
            Engine::Container(family) => {
                let (before, after) = content::decode_container_pair(before, after).ok()?;
                Some(ContentViewer::Members(MemberViewer::new(
                    names, family, before, after, picker, annotating,
                )))
            }
        }
    }

    /// The member view, for a container pair.
    pub fn members(&self) -> Option<&MemberViewer> {
        match self {
            ContentViewer::Members(viewer) => Some(viewer),
            ContentViewer::Picture(_) => None,
        }
    }

    pub fn members_mut(&mut self) -> Option<&mut MemberViewer> {
        match self {
            ContentViewer::Members(viewer) => Some(viewer),
            ContentViewer::Picture(_) => None,
        }
    }

    /// The text member `Enter` asked to see in a full diff view (see
    /// [`MemberViewer::take_open_request`]).
    pub fn take_open_request(&mut self) -> Option<(String, String, String)> {
        self.members_mut().and_then(MemberViewer::take_open_request)
    }

    /// True while a container opened inside this one is showing (see [`MemberViewer`]).
    pub fn has_nested(&self) -> bool {
        self.members().is_some_and(MemberViewer::has_nested)
    }

    /// Switches to a different drawing protocol (the terminal answered the graphics query).
    pub fn set_picker(&mut self, picker: Picker) {
        match self {
            ContentViewer::Picture(viewer) => viewer.set_picker(picker),
            ContentViewer::Members(viewer) => viewer.set_picker(picker),
        }
    }

    /// The engine's verdict on the pair.
    pub fn verdict(&self) -> Verdict {
        match self {
            ContentViewer::Picture(viewer) => viewer.verdict(),
            ContentViewer::Members(viewer) => viewer.diff().verdict(),
        }
    }

    /// Whether the annotation mode hides what the engine decided.
    pub fn annotating(&self) -> bool {
        match self {
            ContentViewer::Picture(viewer) => viewer.annotating(),
            ContentViewer::Members(viewer) => viewer.annotating(),
        }
    }

    pub fn set_annotating(&mut self, annotating: bool) {
        match self {
            ContentViewer::Picture(viewer) => viewer.set_annotating(annotating),
            ContentViewer::Members(viewer) => viewer.set_annotating(annotating),
        }
    }

    /// True while something animates: the caller should draw again soon, not only on a key.
    pub fn is_playing(&self) -> bool {
        match self {
            ContentViewer::Picture(viewer) => viewer.is_playing(),
            ContentViewer::Members(viewer) => viewer.is_playing(),
        }
    }

    /// Handles a key of the view; false for any other key, which it leaves to the caller. `b`
    /// draws every picture in half blocks, or with the terminal's protocol again (see
    /// [`picture_viewer::half_blocks_only`]).
    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        if code == KeyCode::Char('b') {
            picture_viewer::set_half_blocks_only(!picture_viewer::half_blocks_only());
            return true;
        }
        match self {
            ContentViewer::Picture(viewer) => viewer.handle_key(code),
            ContentViewer::Members(viewer) => viewer.handle_key(code),
        }
    }

    /// One line for the status bar.
    pub fn status(&self) -> String {
        match self {
            ContentViewer::Picture(viewer) => viewer.status(),
            ContentViewer::Members(viewer) => viewer.status(),
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect, colors: PictureColors) {
        match self {
            ContentViewer::Picture(viewer) => viewer.draw(frame, area, colors),
            ContentViewer::Members(viewer) => viewer.draw(frame, area, colors),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn b_draws_every_picture_in_half_blocks_and_back() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.png"), dir.path().join("b.png"));
        for (path, pixel) in [(&a, 0u8), (&b, 255)] {
            image::RgbaImage::from_pixel(4, 4, image::Rgba([pixel, 0, 0, 255]))
                .save_with_format(path, image::ImageFormat::Png)
                .unwrap();
        }
        let mut picker = Picker::halfblocks();
        picker.set_protocol_type(ratatui_image::picker::ProtocolType::Kitty);
        let mut viewer = ContentViewer::open(&a, &b, picker).unwrap();
        let was = picture_viewer::half_blocks_only();
        picture_viewer::set_half_blocks_only(false);
        assert!(
            viewer.status().ends_with(" · kitty (b: half blocks)"),
            "{}",
            viewer.status()
        );
        assert!(viewer.handle_key(KeyCode::Char('b')));
        assert!(
            viewer.status().ends_with(" · half blocks (b: pixels)"),
            "{}",
            viewer.status()
        );
        assert!(viewer.handle_key(KeyCode::Char('b')));
        assert!(!picture_viewer::half_blocks_only());
        picture_viewer::set_half_blocks_only(was);
    }
}
