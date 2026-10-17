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

//! A container pair on screen (see [`crate::diff::content::container`]): the members in a list,
//! changed ones only until `a` shows all, and the selected member beside it - a picture in the
//! picture view, text (a Java class file as its listing) diffed by OmniDiff in a
//! [`TextPairView`], a nested container as a line saying `Enter` opens it. `Enter` on a text
//! member asks the caller to show it in a full diff view ([`MemberViewer::take_open_request`]),
//! which the TUI does.
//!
//! The list's first row is the pair itself, so a reader (or `human_solver`) can stand on the
//! whole file as well as on one member. A member is opened only when it is selected: a font's
//! six hundred glyphs are not drawn to show two.
//!
//! `Enter` on a member that is itself a container (a jar in a zip) opens it in place, and
//! `Backspace` comes back out. `g` shows every changed member that is a picture at once - a
//! font's changed glyphs, a theme's changed cursors - as a grid on each side, in the picture view.
//!
//! A removed and an added member can be paired ([`MemberViewer::pair`]), for `human_solver` to say
//! a file was moved or renamed: the two become one changed row, `> old -> new`, opened and diffed
//! like any changed member. The engine's diff ([`MemberViewer::diff`]) is left as the engine made
//! it; the pair's own detail is [`MemberViewer::member_detail`].

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
use super::text_pair_view::TextPairView;
use crate::diff::content::container::{
    self, Aligned, Container, ContainerDiff, ContainerInfo, MemberContent, MemberDetail,
    MemberStatus,
};
use crate::diff::content::{self, Engine, Family, Verdict};
use crate::diff::picture::{self, Frames, PictureInfo};
use image::RgbaImage;

/// One member's place in the grid: its picture on each side that has one.
type Tile = (Option<RgbaImage>, Option<RgbaImage>);

/// How many changed members the grid (`g`) shows at most.
const GRID_MEMBERS: usize = 400;
/// The largest a grid tile is drawn, in pixels: bigger pictures are scaled down to it.
const GRID_TILE_PIXELS: u32 = 128;

/// What the selected member looks like on screen.
enum MemberView {
    Picture(Box<PictureViewer>),
    /// Text (or a class file's listing), diffed by OmniDiff.
    Text(Box<TextPairView>),
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
    /// Members paired by hand: a removed member's index to the added member's it became.
    pairs: Vec<(usize, usize)>,
    /// What changed in each pair, by after's key: `None` if a side does not open.
    pair_details: HashMap<String, Option<MemberDetail>>,
    /// Show every member, not only the ones that changed, were added or were removed.
    show_all: bool,
    /// Show every changed picture member at once, in a grid (`g`), instead of the selected one.
    grid: bool,
    /// The selected row of the visible ones; 0 is the pair itself.
    selected: usize,
    view: Option<MemberView>,
    /// A member container opened with `Enter`, which takes the keys until `Backspace`.
    nested: Option<Box<ContentViewer>>,
    picker: Picker,
    annotating: bool,
    /// A short label per member key, shown after it in the list (`human_solver`'s verdicts).
    marks: HashMap<String, String>,
    /// A text member `Enter` asked to see in a full diff view: its key and both texts, for the
    /// caller to take ([`Self::take_open_request`]).
    open_request: Option<(String, String, String)>,
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
            pairs: Vec::new(),
            pair_details: HashMap::new(),
            show_all: false,
            grid: false,
            selected: 0,
            view: None,
            nested: None,
            picker,
            annotating,
            marks: HashMap::new(),
            open_request: None,
        }
    }

    /// The text member `Enter` asked to open in a full diff view, if it did: (key, before,
    /// after). A caller with no such view leaves it.
    pub fn take_open_request(&mut self) -> Option<(String, String, String)> {
        if let Some(nested) = self.nested.as_mut() {
            return nested.take_open_request();
        }
        self.open_request.take()
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

    /// The key before has for a paired row, which differs from after's; `None` for any other.
    fn renamed_key_of(&self, row: &Aligned) -> Option<&str> {
        let (before, after) = (row.before?, row.after?);
        let before = &self.before.as_deref()?.members()[before].key;
        (*before != self.after.as_deref()?.members()[after].key).then_some(before.as_str())
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

    /// What changed in member `key`: the engine's detail, or a pair's.
    pub fn member_detail(&self, key: &str) -> Option<&MemberDetail> {
        match self.pair_details.get(key) {
            Some(detail) => detail.as_ref(),
            None => self.diff.member(key)?.detail.as_ref(),
        }
    }

    /// The key member `key` had before, if it is after's side of a pair.
    pub fn renamed_from(&self, key: &str) -> Option<&str> {
        self.rows
            .iter()
            .find(|row| row.after.is_some() && self.key_of(row) == key)
            .and_then(|row| self.renamed_key_of(row))
    }

    /// Every pair, as (before's key, after's key).
    pub fn pairs(&self) -> Vec<(String, String)> {
        let key = |side: Option<&dyn Container>, index: usize| {
            side.expect("a paired side is present").members()[index]
                .key
                .clone()
        };
        self.pairs
            .iter()
            .map(|&(before, after)| {
                (
                    key(self.before.as_deref(), before),
                    key(self.after.as_deref(), after),
                )
            })
            .collect()
    }

    /// Pairs the removed member `before` with the added member `after`: one changed row, under
    /// after's key, in the added member's place.
    pub fn pair(&mut self, before: &str, after: &str) -> Result<(), String> {
        let index = |status: MemberStatus, key: &str| {
            self.rows
                .iter()
                .find(|row| row.status == status && self.key_of(row) == key)
                .and_then(|row| match status {
                    MemberStatus::Removed => row.before,
                    _ => row.after,
                })
        };
        let before_index = index(MemberStatus::Removed, before)
            .ok_or_else(|| format!("{before} is not a removed member"))?;
        let after_index = index(MemberStatus::Added, after)
            .ok_or_else(|| format!("{after} is not an added member"))?;
        let open = |side: Option<&dyn Container>, index: usize| side?.open(index).ok();
        let detail = open(self.before.as_deref(), before_index)
            .zip(open(self.after.as_deref(), after_index))
            .map(|(before, after)| container::compare_members(before, after, 0));
        self.pair_details.insert(after.to_string(), detail);
        self.pairs.push((before_index, after_index));
        self.realign(after);
        Ok(())
    }

    /// Undoes the pair whose after side is `after`, returning before's key.
    pub fn unpair(&mut self, after: &str) -> Option<String> {
        let before = self.renamed_from(after)?.to_string();
        self.pair_details.remove(after);
        let after_members = self.after.as_deref().map_or(&[][..], |side| side.members());
        self.pairs
            .retain(|&(_, index)| after_members[index].key != after);
        self.realign(after);
        Some(before)
    }

    /// The rows again, with every pair applied, keeping `key` selected.
    fn realign(&mut self, key: &str) {
        let mut rows = container::align(self.before.as_deref(), self.after.as_deref());
        for &(before, after) in &self.pairs {
            rows.retain(|row| !(row.status == MemberStatus::Removed && row.before == Some(before)));
            if let Some(row) = rows
                .iter_mut()
                .find(|row| row.status == MemberStatus::Added && row.after == Some(after))
            {
                *row = Aligned {
                    before: Some(before),
                    after: Some(after),
                    status: MemberStatus::Changed,
                };
            }
        }
        self.rows = rows;
        self.select(Some(key));
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
            KeyCode::Char('g') => {
                self.grid = !self.grid;
                self.view = None;
            }
            KeyCode::Enter => {
                self.ensure_view();
                if let (Some(MemberView::Text(view)), Some(key)) = (&self.view, self.selected_key())
                {
                    let (before, after) = view.texts();
                    self.open_request =
                        Some((key.to_string(), before.to_string(), after.to_string()));
                    return true;
                }
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
            code => {
                self.ensure_view();
                return match self.view.as_mut() {
                    Some(MemberView::Picture(viewer)) => viewer.handle_key(code),
                    Some(MemberView::Text(view)) => view.handle_key(code),
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
            // The grid shows every changed member whichever is selected.
            if !self.grid {
                self.view = None;
            }
        }
    }

    /// Builds the selected member's view, if it is not built yet.
    fn ensure_view(&mut self) {
        if self.view.is_some() {
            return;
        }
        if self.grid {
            self.view = Some(self.grid_view());
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
        let before_key = self.renamed_key_of(&row).unwrap_or(&key).to_string();
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
            Ok((before, after)) => self.member_view((&before_key, &key), before, after),
            Err(message) => MemberView::Message(message),
        });
    }

    /// The view of one member, from what each side opened to; `keys` are its key on each side,
    /// which differ only for a pair.
    fn member_view(
        &self,
        keys: (&str, &str),
        before: Option<MemberContent>,
        after: Option<MemberContent>,
    ) -> MemberView {
        let key = keys.1;
        // The picture viewer titles each pane "before: <name>" itself.
        let names = (keys.0.to_string(), keys.1.to_string());
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
                    // Text, or a class file as its listing (`container::as_text`).
                    None => match (container::as_text(&b), container::as_text(&a)) {
                        (Some(before_text), Some(after_text)) => MemberView::Text(Box::new(
                            TextPairView::new(Path::new(key), &before_text, &after_text),
                        )),
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

    /// Every changed member that is a picture (a glyph, a cursor, a picture in an archive), as
    /// two grids of tiles - before's and after's, each member in the same place on both - shown as
    /// one picture pair. The first [`GRID_MEMBERS`] of them.
    fn grid_view(&self) -> MemberView {
        let first_frame = |content: Option<MemberContent>| -> Option<RgbaImage> {
            match content? {
                MemberContent::Picture(picture) => Some(picture.1.pixels(0).into_owned()),
                MemberContent::Bytes(bytes) if picture::is_picture(&bytes) => {
                    Some(picture::decode(&bytes).ok()?.1.pixels(0).into_owned())
                }
                _ => None,
            }
        };
        let open = |side: Option<&dyn Container>, index: Option<usize>| {
            side.zip(index)
                .and_then(|(side, index)| side.open(index).ok())
        };
        let tiles: Vec<Tile> = self
            .rows
            .iter()
            .filter(|row| row.status == MemberStatus::Changed)
            .take(GRID_MEMBERS)
            .map(|row| {
                (
                    first_frame(open(self.before.as_deref(), row.before)),
                    first_frame(open(self.after.as_deref(), row.after)),
                )
            })
            .filter(|(before, after)| before.is_some() || after.is_some())
            .collect();
        if tiles.is_empty() {
            return MemberView::Message(
                "No changed member is a picture: g shows the selected member again".to_string(),
            );
        }
        let side = tiles
            .iter()
            .flat_map(|(before, after)| [before, after])
            .flatten()
            .map(|tile| tile.width().max(tile.height()))
            .max()
            .unwrap_or(1)
            .min(GRID_TILE_PIXELS);
        let columns = (tiles.len() as f64).sqrt().ceil() as u32;
        let rows = (tiles.len() as u32).div_ceil(columns);
        let grid = |pick: fn(&Tile) -> &Option<RgbaImage>| {
            let mut canvas = RgbaImage::new(columns * side, rows * side);
            for (index, tile) in tiles.iter().enumerate() {
                let Some(tile) = pick(tile) else {
                    continue;
                };
                let tile = if tile.width() > side || tile.height() > side {
                    image::imageops::thumbnail(tile, side, side)
                } else {
                    tile.clone()
                };
                let (x, y) = (index as u32 % columns * side, index as u32 / columns * side);
                image::imageops::overlay(&mut canvas, &tile, i64::from(x), i64::from(y));
            }
            let frames = Frames::from_frames(vec![(canvas, 0)]).ok()?;
            let (width, height) = frames.dimensions();
            let info = PictureInfo {
                format: "GRID".to_string(),
                width,
                height,
                color: "RGBA8".to_string(),
                bytes: 0,
                frames: 1,
                duration_ms: 0,
            };
            Some((info, frames))
        };
        let count = tiles.len();
        MemberView::Picture(Box::new(PictureViewer::from_decoded(
            format!("{count} changed"),
            format!("{count} changed"),
            grid(|tile| &tile.0),
            grid(|tile| &tile.1),
            self.picker.clone(),
            self.annotating,
        )))
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
        let paired = match self.pairs.len() {
            0 => String::new(),
            pairs => format!(" ({pairs} paired)"),
        };
        format!(
            "{} changed{paired}, {} added, {} removed, {} unchanged",
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
        if self.grid {
            return format!(
                "{}: {} · every changed picture member (g: one at a time) · {}",
                self.family.noun(true),
                self.counts(),
                match &self.view {
                    Some(MemberView::Picture(viewer)) => viewer.status(),
                    _ => String::new(),
                }
            );
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
            (Some(key), Some(MemberView::Text(view))) => {
                match (self.annotating, self.member_verdict(key)) {
                    (false, Some(verdict)) => {
                        format!(
                            " · {key}: omnidiff says {} · {}",
                            verdict.label_in(content::Vocabulary::Text),
                            view.status()
                        )
                    }
                    _ => format!(" · {key} · {}", view.status()),
                }
            }
            (Some(key), _) => match (self.annotating, self.member_verdict(key)) {
                (false, Some(verdict)) => {
                    let binary = self
                        .diff
                        .member(key)
                        .and_then(|member| member.detail.as_ref());
                    let label = match binary {
                        Some(container::MemberDetail::Binary { .. }) => {
                            verdict.label_in(content::Vocabulary::Binary)
                        }
                        _ => verdict.label(),
                    };
                    format!(" · {key}: omnidiff says {label}")
                }
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
            Some(MemberView::Text(view)) => view.draw(frame, columns[1], &title),
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
                    let key = self.key_of(row).to_string();
                    match (row.status, self.renamed_key_of(row)) {
                        (_, Some(before)) => ('>', format!("{before} -> {key}")),
                        (MemberStatus::Same, None) => (' ', key),
                        (MemberStatus::Changed, None) => ('~', key),
                        (MemberStatus::Added, None) => ('+', key),
                        (MemberStatus::Removed, None) => ('-', key),
                    }
                }
            };
            let label = match index {
                0 => self.marks.get("").cloned(),
                _ => self.marks.get(self.key_of(visible[index - 1])).cloned(),
            };
            let color = match mark {
                '~' | '>' => Color::Yellow,
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
                viewer.draw(frame, area, PictureColors::from_theme(Color::Yellow));
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
            viewer.status().contains("omnidiff says edited"),
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
    fn a_removed_and_an_added_member_pair_into_one_changed_row() {
        let mut viewer = viewer(
            &[("old.txt", b"one\ntwo\n"), ("gone.txt", b"g\n")],
            &[("dir/new.txt", b"one\n2\n"), ("added.txt", b"a\n")],
        );
        assert_eq!(
            viewer.pair("old.txt", "gone.txt"),
            Err("gone.txt is not an added member".to_string())
        );
        viewer.pair("old.txt", "dir/new.txt").unwrap();
        assert_eq!(viewer.selected_key(), Some("dir/new.txt"));
        assert_eq!(viewer.selected_status(), Some(MemberStatus::Changed));
        assert_eq!(viewer.changed_keys(), vec!["dir/new.txt".to_string()]);
        assert_eq!(viewer.renamed_from("dir/new.txt"), Some("old.txt"));
        assert_eq!(
            viewer.pairs(),
            vec![("old.txt".to_string(), "dir/new.txt".to_string())]
        );
        assert!(matches!(
            viewer.member_detail("dir/new.txt"),
            Some(MemberDetail::Text {
                removed: 1,
                added: 1,
                ..
            })
        ));
        assert_eq!(
            viewer.member_verdict("dir/new.txt"),
            None,
            "the engine's diff is its own: it sees one added member"
        );
        let text = screen(&mut viewer);
        assert!(text.contains("> old.txt -> dir/new.txt"), "{text}");
        assert!(!text.contains("- old.txt"), "{text}");
        assert!(text.contains("- two"), "the pair is diffed: {text}");
        assert!(text.contains("+ 2"), "{text}");
        let status = viewer.status();
        assert!(
            status.contains("1 changed (1 paired), 1 added, 1 removed"),
            "{status}"
        );

        assert_eq!(viewer.unpair("dir/new.txt"), Some("old.txt".to_string()));
        assert!(viewer.pairs().is_empty());
        assert_eq!(viewer.member_detail("dir/new.txt"), None);
        let text = screen(&mut viewer);
        assert!(text.contains("- old.txt"), "{text}");
        assert!(text.contains("+ dir/new.txt"), "{text}");
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
        assert_eq!(
            viewer
                .member_verdict("icon.png")
                .map(|verdict| verdict.to_string())
                .as_deref(),
            Some("redrawn"),
            "a flat black square turned flat red"
        );
    }

    #[test]
    fn g_shows_every_changed_glyph_in_a_grid() {
        let read = |name: &str| {
            std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("src/test/data/content")
                    .join(name),
            )
            .unwrap()
        };
        let (before, after) =
            content::decode_container_pair(&read("before.ttf"), &read("after.ttf")).unwrap();
        let mut viewer = MemberViewer::new(
            ("before.ttf".to_string(), "after.ttf".to_string()),
            Family::Fonts,
            before,
            after,
            Picker::halfblocks(),
            true,
        );
        viewer.handle_key(KeyCode::Char('g'));
        let text = screen(&mut viewer);
        assert!(text.contains("before: 1 changed"), "{text}");
        assert!(
            !text.contains("before: before"),
            "one side label, not two: {text}"
        );
        let status = viewer.status();
        assert!(status.contains("every changed picture member"), "{status}");
        // B is the one changed glyph: a one-tile grid, a glyph's cell wide.
        assert!(status.contains("GRID 96x96"), "{status}");
        viewer.handle_key(KeyCode::Char('g'));
        assert!(!viewer.status().contains("GRID"));
    }
}
