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

//! A picture pair on screen (see [`crate::diff::picture`]): the two pictures with their changed
//! regions outlined, or one composite of them, in four views `t` cycles through - GitHub's image
//! diff modes:
//!
//! * **side by side**: both, each region outlined in the delete (before) or insert (after) color;
//! * **difference**: the after picture faded out, its changed pixels in the update color;
//! * **blend**: the two mixed half and half, so a shifted element shows twice;
//! * **swipe**: before left of a divider and after right of it, `h`/`l` moving the divider.
//!
//! Drawn by `ratatui-image`: in a terminal that speaks the kitty, sixel or iTerm2 graphics protocol
//! (`tui::app` asks it at startup) as real pixels, and everywhere else as Unicode half blocks - two
//! pixels a cell, coarse but enough to see the layout and where the outlines are. The color-depth
//! pass fits half blocks to a 256-color terminal like any other cell.
//!
//! An animation is shown one **moment** at a time: a frame of each side, as the frame diff pairs
//! them (`FrameStep`), `,`/`.` stepping and space playing at the after side's frame times. When
//! annotating, the frames pair by position instead, since the engine's pairing is part of its
//! answer.
//!
//! Composites are rebuilt only when the view, the divider, the moment, the pane size or the colors
//! change: encoding a picture for a graphics protocol is the slow part, not drawing it.

use std::path::Path;
use std::time::{Duration, Instant};

use crate::diff::picture::{self, Comparison, FrameCounts, FrameStep, Frames, PictureDiff, Region};
use crossterm::event::KeyCode;
use image::{DynamicImage, Rgba, RgbaImage};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};
use ratatui_image::{
    Resize, StatefulImage,
    picker::{Picker, ProtocolType, cap_parser::QueryStdioOptions},
    protocol::StatefulProtocol,
};

/// What pictures are shown over: transparency is compared as over white (`diff::picture`), and
/// half blocks would draw it black. Also what pads a scaled picture to whole cells.
const BACKDROP: Rgba<u8> = Rgba([255, 255, 255, 255]);

/// Asks the terminal which graphics protocol it speaks; `None` if the query fails. Reads the answer
/// from stdin, so it must run before anything else reads input.
///
/// Kitty is also asked whether it takes zlib-compressed pictures (`o=z`), and gets them if so. A
/// picture is sent scaled to its pane, so an icon arrives as megabytes of repeated pixels: through
/// tmux or ssh that transfer was most of the wait for every frame, and deflated it is a few
/// percent of the size.
///
/// **`OMNIDIFF_GRAPHICS`** overrides the detection: `halfblocks` (no query sent at all), `kitty`,
/// `sixel` or `iterm2`; `auto` or unset detects.
///
/// **Inside tmux the answers cannot be trusted**: tmux passes the query to every client attached
/// to the session, so a kitty on a desk answers for a phone's terminal attached to the same
/// session, which then shows kitty's placeholders as rows of crossed-out boxes. tmux knows each
/// client's terminal, though, so only the protocols the client in front of the pane supports are
/// allowed ([`protocols_for_client`]), and with none of them the query is not sent.
pub fn query_graphics() -> Option<Picker> {
    detect_graphics().0
}

/// [`query_graphics`], with why it chose what it did, in a few words (`omnidiff util graphics`
/// prints both).
pub fn detect_graphics() -> (Option<Picker>, String) {
    let query = |blacklist: Vec<ProtocolType>| {
        Picker::from_query_stdio_with_options(QueryStdioOptions {
            kitty_compression: true,
            blacklist_protocols: blacklist,
            ..QueryStdioOptions::default()
        })
        .ok()
    };
    let setting = std::env::var("OMNIDIFF_GRAPHICS")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let forced = match setting.as_str() {
        "halfblocks" | "blocks" | "none" | "off" => {
            set_half_blocks_only(true);
            return (
                Some(Picker::halfblocks()),
                format!("OMNIDIFF_GRAPHICS={setting}"),
            );
        }
        "kitty" => Some(ProtocolType::Kitty),
        "sixel" => Some(ProtocolType::Sixel),
        "iterm2" => Some(ProtocolType::Iterm2),
        _ => None,
    };
    if let Some(protocol) = forced {
        let mut picker = query(Vec::new()).unwrap_or_else(Picker::halfblocks);
        picker.set_protocol_type(protocol);
        return (Some(picker), format!("OMNIDIFF_GRAPHICS={setting}"));
    }
    let Some((client, allowed)) = tmux_client_protocols() else {
        return (query(Vec::new()), "the terminal's answer".to_string());
    };
    if allowed.is_empty() {
        return (
            Some(Picker::halfblocks()),
            format!("tmux client {client} draws no pictures; not asked"),
        );
    }
    let blacklist = [
        ProtocolType::Kitty,
        ProtocolType::Sixel,
        ProtocolType::Iterm2,
    ]
    .into_iter()
    .filter(|protocol| !allowed.contains(protocol))
    .collect();
    let picked = query(blacklist);
    // Every attached client answered the query, and the reading stopped at the first answer:
    // the rest would arrive as keys.
    drain_late_answers();
    let Some(mut picker) = picked else {
        return (None, format!("tmux client {client}; the query failed"));
    };
    if !allowed.contains(&picker.protocol_type()) {
        picker.set_protocol_type(allowed[0]);
    }
    (Some(picker), format!("tmux client {client}"))
}

/// Reads and drops whatever the terminal sends until it has been quiet for 150 ms, half a second
/// at most: the answers of the other clients of a tmux session to a graphics query, which come
/// after the one the query read and would otherwise reach the program as typed keys.
///
/// On the file descriptor, never blocking: the query's own reader thread may still be waiting on
/// the terminal too, and a blocking read that loses the bytes to it would wait for the next key
/// (crossterm's `read` after its `poll` did exactly that).
#[cfg(unix)]
fn drain_late_answers() {
    use std::os::fd::AsRawFd;
    let stdin = std::io::stdin();
    let fd = stdin.as_raw_fd();
    // SAFETY: `fcntl`, `poll` and `read` on our own stdin, into a buffer we own; the flags are
    // restored before returning.
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags < 0 {
            return;
        }
        libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        let deadline = Instant::now() + Duration::from_millis(500);
        let mut buffer = [0u8; 4096];
        while Instant::now() < deadline {
            let mut ready = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };
            if libc::poll(&mut ready, 1, 150) <= 0 {
                break;
            }
            while libc::read(fd, buffer.as_mut_ptr().cast(), buffer.len()) > 0 {}
        }
        libc::fcntl(fd, libc::F_SETFL, flags);
    }
}

#[cfg(not(unix))]
fn drain_late_answers() {}

/// The tmux client in front of this pane (its `TERM` and the name its terminal answered), and the
/// graphics protocols it supports, best first; `None` outside tmux, or when tmux does not say.
fn tmux_client_protocols() -> Option<(String, Vec<ProtocolType>)> {
    std::env::var_os("TMUX")?;
    let output = std::process::Command::new("tmux")
        .args([
            "display-message",
            "-p",
            "#{client_termname}\t#{client_termtype}\t#{client_termfeatures}",
        ])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    let line = String::from_utf8_lossy(&output.stdout);
    let mut fields = line.trim_end().split('\t');
    let (name, kind, features) = (fields.next()?, fields.next()?, fields.next().unwrap_or(""));
    if !output.status.success() || name.is_empty() {
        return None;
    }
    let client = if kind.is_empty() {
        name.to_string()
    } else {
        format!("{name} {kind}")
    };
    Some((client, protocols_for_client(name, kind, features)))
}

/// The graphics protocols a terminal supports by what tmux knows of it - its `TERM`, the name and
/// version it answered (`kitty(0.44.0)`) and tmux's feature list - best first: kitty's for kitty
/// and Ghostty, iTerm2's for iTerm2 and WezTerm, sixel where tmux lists it.
pub fn protocols_for_client(term: &str, term_type: &str, features: &str) -> Vec<ProtocolType> {
    let names = format!("{term} {term_type}").to_ascii_lowercase();
    let mut protocols = Vec::new();
    if names.contains("kitty") || names.contains("ghostty") {
        protocols.push(ProtocolType::Kitty);
    }
    if names.contains("iterm") || names.contains("wezterm") {
        protocols.push(ProtocolType::Iterm2);
    }
    if features.split(',').any(|feature| feature == "sixel") {
        protocols.push(ProtocolType::Sixel);
    }
    protocols
}

/// Whether every picture is drawn in half blocks, whatever the terminal answered: `b` toggles it,
/// for a terminal that claimed a protocol it cannot draw. For the whole process, so the next
/// picture or sample opens the same way.
static HALF_BLOCKS_ONLY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn half_blocks_only() -> bool {
    HALF_BLOCKS_ONLY.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn set_half_blocks_only(on: bool) {
    HALF_BLOCKS_ONLY.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// The four ways a picture pair is shown, in the order `t` steps through them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PictureMode {
    SideBySide,
    Difference,
    Blend,
    Swipe,
}

impl PictureMode {
    fn next(self) -> Self {
        match self {
            Self::SideBySide => Self::Difference,
            Self::Difference => Self::Blend,
            Self::Blend => Self::Swipe,
            Self::Swipe => Self::SideBySide,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::SideBySide => "side by side",
            Self::Difference => "difference",
            Self::Blend => "blend",
            Self::Swipe => "swipe",
        }
    }
}

/// The overlay colors a picture view paints with, from the current theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PictureColors {
    pub delete: [u8; 3],
    pub insert: [u8; 3],
    pub update: [u8; 3],
}

/// The swipe divider: deep pink, which few pictures are and which shows on the white backdrop.
/// Twice an outline's width (`Built::outline`), so it reads as a handle rather than an edge.
const DIVIDER: [u8; 3] = [255, 20, 147];

impl PictureColors {
    /// From theme colors; a color that is not RGB (a named or indexed one) falls back to a vivid
    /// default.
    pub fn from_theme(delete: Color, insert: Color, update: Color) -> Self {
        let rgb = |color: Color, fallback: [u8; 3]| match color {
            Color::Rgb(r, g, b) => [r, g, b],
            _ => fallback,
        };
        Self {
            delete: rgb(delete, [220, 50, 47]),
            insert: rgb(insert, [64, 160, 43]),
            update: rgb(update, [230, 160, 20]),
        }
    }
}

/// What the composites were built for; anything else means rebuilding them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Built {
    half_blocks: bool,
    mode: PictureMode,
    swipe_percent: u16,
    moment: usize,
    outline: u32,
    colors: PictureColors,
}

/// Each pane's title and picture, encoded for the terminal; `None` for a side with no picture.
type Panes = Vec<(String, Option<StatefulProtocol>)>;

/// One position of the frame stepper: which frame of each side shows (`None` where that side has
/// none here), and what the engine found between them.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Moment {
    before: Option<usize>,
    after: Option<usize>,
    regions: Vec<Region>,
    /// "changed", "added", "removed", or "" for frames that look the same or are not compared.
    change: &'static str,
}

/// The moments of a pair: one for two stills, one per aligned frame for animations.
fn timeline(
    diff: &PictureDiff,
    before: Option<&Frames>,
    after: Option<&Frames>,
    by_position: bool,
) -> Vec<Moment> {
    let moment = |before, after, regions, change| Moment {
        before,
        after,
        regions,
        change,
    };
    match &diff.comparison {
        Comparison::Pixels { regions, .. } if !by_position => {
            vec![moment(Some(0), Some(0), regions.clone(), "")]
        }
        Comparison::Frames { steps, .. } if !by_position => steps
            .iter()
            .flat_map(|step| -> Vec<Moment> {
                match step {
                    FrameStep::Same {
                        before,
                        after,
                        frames,
                    } => (0..*frames)
                        .map(|k| moment(Some(before + k), Some(after + k), Vec::new(), ""))
                        .collect(),
                    FrameStep::Changed {
                        before,
                        after,
                        regions,
                        ..
                    } => vec![moment(
                        Some(*before),
                        Some(*after),
                        regions.clone(),
                        "changed",
                    )],
                    FrameStep::Inserted { after, frames } => (0..*frames)
                        .map(|k| moment(None, Some(after + k), Vec::new(), "added"))
                        .collect(),
                    FrameStep::Deleted { before, frames } => (0..*frames)
                        .map(|k| moment(Some(before + k), None, Vec::new(), "removed"))
                        .collect(),
                }
            })
            .collect(),
        // Resized, one-sided, or annotating: frame k beside frame k.
        _ => {
            let count = |side: Option<&Frames>| side.map_or(0, Frames::count);
            let (b, a) = (count(before), count(after));
            (0..b.max(a))
                .map(|k| moment((k < b).then_some(k), (k < a).then_some(k), Vec::new(), ""))
                .collect()
        }
    }
}

pub struct PictureViewer {
    before_name: String,
    after_name: String,
    before: Option<Frames>,
    after: Option<Frames>,
    diff: PictureDiff,
    timeline: Vec<Moment>,
    moment: usize,
    /// While playing: when the current moment started showing.
    playing: Option<Instant>,
    mode: PictureMode,
    swipe_percent: u16,
    picker: Picker,
    shown: Option<(Built, Panes)>,
    /// For a human recording a verdict (`human_solver`): nothing the engine decided is shown - no
    /// outlined regions, no difference view, no "how much changed" - only the two pictures and
    /// what their files say they are.
    annotating: bool,
}

impl PictureViewer {
    /// The view of `before` and `after` if they are a picture pair that decodes; `None` otherwise,
    /// and the caller reports them as any other binary pair.
    pub fn open(before: &Path, after: &Path, picker: Picker) -> Option<Self> {
        Self::open_with(before, after, picker, false)
    }

    /// [`Self::open`] for annotation: see [`Self::annotating`](#structfield.annotating).
    pub fn open_for_annotation(before: &Path, after: &Path, picker: Picker) -> Option<Self> {
        Self::open_with(before, after, picker, true)
    }

    fn open_with(before: &Path, after: &Path, picker: Picker, annotating: bool) -> Option<Self> {
        let read = |path: &Path| std::fs::read(path).ok();
        let (before_bytes, after_bytes) = (read(before)?, read(after)?);
        if !picture::is_picture_pair(&before_bytes, &after_bytes) {
            return None;
        }
        // Decoded once and kept: an animation's frames are the expensive part.
        let (before_decoded, after_decoded) =
            picture::decode_pair(&before_bytes, &after_bytes).ok()?;
        Some(Self::from_decoded(
            before.display().to_string(),
            after.display().to_string(),
            before_decoded,
            after_decoded,
            picker,
            annotating,
        ))
    }

    /// The view of two pictures already decoded (a glyph, a page, a picture in an archive), each
    /// pane titled with its name; a side that is `None` is absent.
    pub fn from_decoded(
        before_name: String,
        after_name: String,
        before: picture::Decoded,
        after: picture::Decoded,
        picker: Picker,
        annotating: bool,
    ) -> Self {
        let diff = picture::compare_decoded(before.as_ref(), after.as_ref());
        let (before_frames, after_frames) = (
            before.map(|(_, frames)| frames),
            after.map(|(_, frames)| frames),
        );
        let timeline = timeline(
            &diff,
            before_frames.as_ref(),
            after_frames.as_ref(),
            annotating,
        );
        Self {
            before_name,
            after_name,
            before: before_frames,
            after: after_frames,
            diff,
            timeline,
            moment: 0,
            playing: None,
            mode: PictureMode::SideBySide,
            swipe_percent: 50,
            picker: {
                let mut picker = picker;
                picker.set_background_color(Some(BACKDROP));
                picker
            },
            shown: None,
            annotating,
        }
    }

    /// Switches to a different drawing protocol (the terminal answered the graphics query).
    pub fn set_picker(&mut self, mut picker: Picker) {
        picker.set_background_color(Some(BACKDROP));
        self.picker = picker;
        self.shown = None;
    }

    pub fn mode(&self) -> PictureMode {
        self.mode
    }

    /// The engine's verdict on the pair (see [`PictureDiff::verdict`]).
    pub fn verdict(&self) -> picture::Verdict {
        self.diff.verdict()
    }

    /// Whether the annotation mode hides what the engine decided.
    pub fn annotating(&self) -> bool {
        self.annotating
    }

    /// Switches the annotation mode on or off: off shows the outlined regions, the difference
    /// view and "how much changed" again. Leaving the difference view when it is switched on.
    pub fn set_annotating(&mut self, annotating: bool) {
        self.annotating = annotating;
        if annotating && self.mode == PictureMode::Difference {
            self.mode = PictureMode::SideBySide;
        }
        // The pairing changes with it; stay on the after frame being looked at.
        let after = self.current().after;
        self.timeline = timeline(
            &self.diff,
            self.before.as_ref(),
            self.after.as_ref(),
            annotating,
        );
        self.moment = self
            .timeline
            .iter()
            .position(|moment| after.is_some() && moment.after == after)
            .unwrap_or(0);
        self.shown = None;
    }

    /// True while an animation plays: the caller should draw again soon, not only on a key.
    pub fn is_playing(&self) -> bool {
        self.playing.is_some()
    }

    fn current(&self) -> &Moment {
        &self.timeline[self.moment.min(self.timeline.len() - 1)]
    }

    /// How long the current moment shows while playing: the after frame's time, or before's where
    /// after has none. A delay under 20 ms plays at 100 ms, as browsers do.
    fn current_delay(&self) -> Duration {
        let moment = self.current();
        let delay = match (&self.after, moment.after, &self.before, moment.before) {
            (Some(frames), Some(index), _, _) | (_, _, Some(frames), Some(index)) => {
                frames.delay_ms(index)
            }
            _ => 0,
        };
        Duration::from_millis(if delay < 20 { 100 } else { u64::from(delay) })
    }

    /// Moves playback on to the moment that should be showing now, wrapping at the end.
    fn advance_playback(&mut self) {
        let Some(mut started) = self.playing else {
            return;
        };
        let mut steps = 0;
        // Bounded: a slow draw skips frames rather than replaying all it missed.
        while started.elapsed() >= self.current_delay() && steps < self.timeline.len() {
            started += self.current_delay();
            self.moment = (self.moment + 1) % self.timeline.len();
            steps += 1;
        }
        if steps == self.timeline.len() {
            started = Instant::now();
        }
        self.playing = Some(started);
    }

    /// Handles a picture view key: `t` cycles the view, `h`/`l` or the arrows move the swipe
    /// divider, and for an animation `,`/`.` step a frame back or on and space plays or pauses.
    /// False for any other key, which the viewer leaves to the rest of the app.
    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        let animated = self.timeline.len() > 1;
        match code {
            KeyCode::Char('.') if animated => {
                self.playing = None;
                self.moment = (self.moment + 1).min(self.timeline.len() - 1);
            }
            KeyCode::Char(',') if animated => {
                self.playing = None;
                self.moment = self.moment.saturating_sub(1);
            }
            KeyCode::Char(' ') if animated => {
                self.playing = match self.playing {
                    Some(_) => None,
                    None => Some(Instant::now()),
                };
            }
            KeyCode::Char('t') => {
                self.mode = self.mode.next();
                if self.annotating && self.mode == PictureMode::Difference {
                    self.mode = self.mode.next();
                }
            }
            KeyCode::Char('h') | KeyCode::Left if self.mode == PictureMode::Swipe => {
                self.swipe_percent = self.swipe_percent.saturating_sub(5);
            }
            KeyCode::Char('l') | KeyCode::Right if self.mode == PictureMode::Swipe => {
                self.swipe_percent = (self.swipe_percent + 5).min(100);
            }
            _ => return false,
        }
        true
    }

    /// One line for the status bar: what each side is, how much changed, and the view. Without
    /// "how much changed" when annotating.
    pub fn status(&self) -> String {
        if self.annotating {
            let side = |info: &Option<picture::PictureInfo>| match info {
                Some(info) => format!(
                    "{} {}x{} {}, {} bytes",
                    info.format, info.width, info.height, info.color, info.bytes
                ),
                None => "nothing".to_string(),
            };
            return format!(
                "{} -> {}{} · view: {} (t){}",
                side(&self.diff.before),
                side(&self.diff.after),
                self.frame_status(),
                self.mode.label(),
                self.drawing_status()
            );
        }
        let side = |info: &Option<picture::PictureInfo>| match info {
            Some(info) => format!(
                "{} {}x{} {}",
                info.format, info.width, info.height, info.color
            ),
            None => "nothing".to_string(),
        };
        let change = match &self.diff.comparison {
            Comparison::OneSided if self.diff.before.is_none() => "added".to_string(),
            Comparison::OneSided => "deleted".to_string(),
            Comparison::Resized => "resized".to_string(),
            Comparison::Pixels { regions, .. } if regions.is_empty() => {
                "no pixel changed".to_string()
            }
            Comparison::Pixels {
                changed_pixels,
                total_pixels,
                regions,
            } => format!(
                "{:.2}% of pixels changed, in {} region{}",
                100.0 * *changed_pixels as f64 / (*total_pixels).max(1) as f64,
                regions.len(),
                if regions.len() == 1 { "" } else { "s" }
            ),
            Comparison::Frames { steps, retimed, .. } => {
                let counts = FrameCounts::of(steps).describe();
                if *retimed {
                    format!("{counts}, retimed")
                } else {
                    counts
                }
            }
        };
        format!(
            "{} -> {} · {change}{} · view: {} (t){}",
            side(&self.diff.before),
            side(&self.diff.after),
            self.frame_status(),
            self.mode.label(),
            self.drawing_status()
        )
    }

    /// " · frame 12/218 → 13/220, changed (,/. step, space play)" for an animation, "" otherwise.
    /// Without what changed when annotating.
    fn frame_status(&self) -> String {
        if self.timeline.len() < 2 {
            return String::new();
        }
        let moment = self.current();
        let position = |index: Option<usize>, frames: &Option<Frames>| match index {
            Some(index) => format!("{}/{}", index + 1, frames.as_ref().map_or(0, Frames::count)),
            None => "-".to_string(),
        };
        let change = if self.annotating || moment.change.is_empty() {
            String::new()
        } else {
            format!(", {}", moment.change)
        };
        let keys = if self.playing.is_some() {
            "space pauses"
        } else {
            ",/. step, space plays"
        };
        format!(
            " · frame {} -> {}{change} ({keys})",
            position(moment.before, &self.before),
            position(moment.after, &self.after)
        )
    }

    /// How pictures are drawn, and that `b` changes it: " · half blocks (b: pixels)", or
    /// " · kitty (b: half blocks)"; nothing when the terminal speaks no protocol.
    fn drawing_status(&self) -> String {
        if half_blocks_only() {
            return " · half blocks (b: pixels)".to_string();
        }
        match self.picker.protocol_type() {
            ProtocolType::Halfblocks => String::new(),
            protocol => format!(
                " · {} (b: half blocks)",
                format!("{protocol:?}").to_lowercase()
            ),
        }
    }

    /// The picker pictures are drawn with: half blocks while [`half_blocks_only`], the terminal's
    /// otherwise.
    fn effective_picker(&self) -> Picker {
        if half_blocks_only() {
            let mut picker = Picker::halfblocks();
            picker.set_background_color(Some(BACKDROP));
            picker
        } else {
            self.picker.clone()
        }
    }

    /// Draws the current view into `area`.
    pub fn draw(&mut self, frame: &mut Frame, area: Rect, colors: PictureColors) {
        self.advance_playback();
        let panes: Vec<Rect> = if self.mode == PictureMode::SideBySide {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area)
                .to_vec()
        } else {
            vec![area]
        };
        let built = Built {
            half_blocks: half_blocks_only(),
            mode: self.mode,
            swipe_percent: self.swipe_percent,
            moment: self.moment,
            outline: self.outline_width(Block::default().borders(Borders::ALL).inner(panes[0])),
            colors,
        };
        if self.shown.as_ref().map(|(was, _)| *was) != Some(built) {
            let shown = self
                .composites(built)
                .into_iter()
                .map(|(title, pixels)| {
                    let protocol = pixels.map(|pixels| {
                        self.effective_picker()
                            .new_resize_protocol(DynamicImage::ImageRgba8(over_backdrop(pixels)))
                    });
                    (title, protocol)
                })
                .collect();
            self.shown = Some((built, shown));
        }
        let Some((_, shown)) = self.shown.as_mut() else {
            return;
        };
        for (pane, (title, protocol)) in panes.iter().zip(shown.iter_mut()) {
            let block = Block::default().borders(Borders::ALL).title(title.as_str());
            let inner = block.inner(*pane);
            frame.render_widget(block, *pane);
            match protocol {
                Some(protocol) => frame.render_stateful_widget(
                    StatefulImage::default().resize(Resize::Scale(None)),
                    inner,
                    protocol,
                ),
                None => frame.render_widget(
                    Paragraph::new("nothing").style(Style::new().fg(Color::DarkGray)),
                    inner,
                ),
            }
        }
    }

    /// How many picture pixels wide an outline must be to show once the picture is scaled to fit
    /// `pane`: a whole cell in half blocks, whose cell is one picture pixel wide, and two screen
    /// pixels under a graphics protocol.
    fn outline_width(&self, pane: Rect) -> u32 {
        let half_blocks = self.effective_picker().protocol_type() == ProtocolType::Halfblocks;
        let (per_cell, screen_pixels) = if half_blocks {
            (1, 1)
        } else {
            (u32::from(self.picker.font_size().width.max(1)), 2)
        };
        let pane_pixels = u32::from(pane.width.max(1)) * per_cell;
        let widest = [&self.before, &self.after]
            .iter()
            .filter_map(|side| side.as_ref().map(|frames| frames.dimensions().0))
            .max()
            .unwrap_or(1);
        (widest.div_ceil(pane_pixels) * screen_pixels).max(1)
    }

    /// One `(title, picture)` per pane of `built.mode`; `None` for a side that has no picture.
    fn composites(&self, built: Built) -> Vec<(String, Option<RgbaImage>)> {
        let moment = &self.timeline[built.moment.min(self.timeline.len() - 1)];
        let regions: &[Region] = if self.annotating {
            &[]
        } else {
            &moment.regions
        };
        let pixels = |frames: &Option<Frames>, index: Option<usize>| -> Option<RgbaImage> {
            Some(frames.as_ref()?.pixels(index?).into_owned())
        };
        let (before_pixels, after_pixels) = (
            pixels(&self.before, moment.before),
            pixels(&self.after, moment.after),
        );
        let outlined = |pixels: &Option<RgbaImage>, color: [u8; 3]| {
            pixels.as_ref().map(|pixels| {
                let mut pixels = pixels.clone();
                for region in regions {
                    outline(&mut pixels, region, built.outline, color);
                }
                pixels
            })
        };
        let titled = |side: &str, name: &str, index: Option<usize>| match index {
            _ if self.timeline.len() < 2 => format!("{side}: {name}"),
            Some(index) => format!("{side}: {name} · frame {}", index + 1),
            None => format!("{side}: {name} · no frame here"),
        };
        let before_title = titled("before", &self.before_name, moment.before);
        let after_title = titled("after", &self.after_name, moment.after);
        // The other three need both sides at one size: before is scaled to after's.
        let (Some(before), Some(after)) = (&before_pixels, &after_pixels) else {
            return vec![
                (before_title, outlined(&before_pixels, built.colors.delete)),
                (after_title, outlined(&after_pixels, built.colors.insert)),
            ];
        };
        let before_at_after_size = || {
            if before.dimensions() == after.dimensions() {
                before.clone()
            } else {
                image::imageops::resize(
                    before,
                    after.width(),
                    after.height(),
                    image::imageops::FilterType::Triangle,
                )
            }
        };
        match built.mode {
            PictureMode::SideBySide => vec![
                (before_title, outlined(&before_pixels, built.colors.delete)),
                (after_title, outlined(&after_pixels, built.colors.insert)),
            ],
            PictureMode::Difference => {
                let mask = (before.dimensions() == after.dimensions())
                    .then(|| picture::changed_mask(before, after));
                let title = match &mask {
                    Some(_) => "difference: changed pixels on the faded after picture",
                    None => "difference: not comparable pixel by pixel (resized)",
                };
                vec![(
                    title.to_string(),
                    Some(difference(after, mask.as_deref(), built.colors.update)),
                )]
            }
            PictureMode::Blend => {
                vec![(
                    "blend: before and after, half and half".to_string(),
                    Some(blend(&before_at_after_size(), after)),
                )]
            }
            PictureMode::Swipe => vec![(
                format!("swipe: before | after at {}% (h/l)", built.swipe_percent),
                Some(swipe(
                    &before_at_after_size(),
                    after,
                    built.swipe_percent,
                    built.outline * 2,
                    DIVIDER,
                )),
            )],
        }
    }
}

/// `pixels` flattened over [`BACKDROP`], fully opaque.
fn over_backdrop(mut pixels: RgbaImage) -> RgbaImage {
    for pixel in pixels.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for channel in 0..3 {
            let over = u16::from(BACKDROP[channel]);
            pixel[channel] =
                ((u16::from(pixel[channel]) * alpha + over * (255 - alpha)) / 255) as u8;
        }
        pixel[3] = 255;
    }
    pixels
}

/// Draws `region`'s border, `width` pixels thick and outside the region where there is room.
fn outline(pixels: &mut RgbaImage, region: &Region, width: u32, color: [u8; 3]) {
    let (w, h) = pixels.dimensions();
    let x0 = region.x.saturating_sub(width);
    let y0 = region.y.saturating_sub(width);
    let x1 = (region.x + region.width + width).min(w);
    let y1 = (region.y + region.height + width).min(h);
    let color = Rgba([color[0], color[1], color[2], 255]);
    for y in y0..y1 {
        for x in x0..x1 {
            let inside = x >= region.x
                && x < region.x + region.width
                && y >= region.y
                && y < region.y + region.height;
            if !inside {
                pixels.put_pixel(x, y, color);
            }
        }
    }
}

/// `after` faded three quarters of the way to white, its changed pixels in `color`.
fn difference(after: &RgbaImage, mask: Option<&[bool]>, color: [u8; 3]) -> RgbaImage {
    let mut out = after.clone();
    for (index, pixel) in out.pixels_mut().enumerate() {
        if mask.is_some_and(|mask| mask[index]) {
            *pixel = Rgba([color[0], color[1], color[2], 255]);
        } else {
            let fade = |channel: u8| (u16::from(channel) / 4 + 191) as u8;
            *pixel = Rgba([
                fade(pixel[0]),
                fade(pixel[1]),
                fade(pixel[2]),
                pixel[3].max(64),
            ]);
        }
    }
    out
}

/// The two same-size pictures mixed half and half.
fn blend(before: &RgbaImage, after: &RgbaImage) -> RgbaImage {
    let mut out = after.clone();
    for (pixel, other) in out.pixels_mut().zip(before.pixels()) {
        for channel in 0..4 {
            pixel[channel] = ((u16::from(pixel[channel]) + u16::from(other[channel])) / 2) as u8;
        }
    }
    out
}

/// Before left of the divider at `percent` of the width, after right of it, the divider `width`
/// pixels in `color`.
fn swipe(
    before: &RgbaImage,
    after: &RgbaImage,
    percent: u16,
    width: u32,
    color: [u8; 3],
) -> RgbaImage {
    let mut out = after.clone();
    let split = after.width() * u32::from(percent) / 100;
    for (x, y, pixel) in out.enumerate_pixels_mut() {
        if x < split {
            *pixel = *before.get_pixel(x, y);
        }
        if x >= split.saturating_sub(width / 2) && x < split + width.div_ceil(2) {
            *pixel = Rgba([color[0], color[1], color[2], 255]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLORS: PictureColors = PictureColors {
        delete: [200, 0, 0],
        insert: [0, 200, 0],
        update: [250, 150, 0],
    };

    fn png(pixels: &RgbaImage, dir: &tempfile::TempDir, name: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        pixels.save(&path).expect("writes");
        path
    }

    /// A 20x10 white picture, and the same with a black 4x3 block at (6, 2).
    fn pair(dir: &tempfile::TempDir) -> (std::path::PathBuf, std::path::PathBuf) {
        let before = RgbaImage::from_pixel(20, 10, Rgba([255, 255, 255, 255]));
        let mut after = before.clone();
        for y in 2..5 {
            for x in 6..10 {
                after.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            }
        }
        (png(&before, dir, "a.png"), png(&after, dir, "b.png"))
    }

    fn viewer(before: &Path, after: &Path) -> PictureViewer {
        PictureViewer::open(before, after, Picker::halfblocks()).expect("a picture pair")
    }

    /// Before: frames 0, 1, 2 of `picture::test_frame`; after: the same with a new frame 5 after
    /// the first.
    fn animated_pair(dir: &tempfile::TempDir) -> (std::path::PathBuf, std::path::PathBuf) {
        let gif = |ks: &[u32]| {
            picture::test_gif(
                &ks.iter()
                    .map(|&k| (picture::test_frame(k), 100))
                    .collect::<Vec<_>>(),
            )
        };
        let (before, after) = (dir.path().join("a.gif"), dir.path().join("b.gif"));
        std::fs::write(&before, gif(&[0, 1, 2])).expect("writes");
        std::fs::write(&after, gif(&[0, 5, 1, 2])).expect("writes");
        (before, after)
    }

    fn sides(viewer: &PictureViewer) -> (Option<usize>, Option<usize>) {
        (viewer.current().before, viewer.current().after)
    }

    #[test]
    fn tmux_clients_are_trusted_for_what_their_terminal_draws() {
        assert_eq!(
            protocols_for_client("xterm-kitty", "kitty(0.44.0)", "RGB,title"),
            vec![ProtocolType::Kitty]
        );
        assert_eq!(
            protocols_for_client("xterm-256color", "", "bpaste,clipboard"),
            Vec::<ProtocolType>::new(),
            "Termius, say: half blocks, and no query"
        );
        assert_eq!(
            protocols_for_client("xterm-256color", "WezTerm 20240203", "sixel,RGB"),
            vec![ProtocolType::Iterm2, ProtocolType::Sixel]
        );
    }

    #[test]
    fn an_animation_steps_through_its_frames_as_the_diff_pairs_them() {
        let dir = tempfile::tempdir().expect("dir");
        let (a, b) = animated_pair(&dir);
        let mut viewer = viewer(&a, &b);
        assert_eq!(sides(&viewer), (Some(0), Some(0)));
        assert!(viewer.handle_key(KeyCode::Char('.')));
        assert_eq!(sides(&viewer), (None, Some(1)), "the added frame");
        assert!(
            viewer.status().contains("frame - -> 2/4, added"),
            "{}",
            viewer.status()
        );
        viewer.handle_key(KeyCode::Char('.'));
        assert_eq!(sides(&viewer), (Some(1), Some(2)));
        viewer.handle_key(KeyCode::Char(','));
        viewer.handle_key(KeyCode::Char(','));
        viewer.handle_key(KeyCode::Char(','));
        assert_eq!(sides(&viewer), (Some(0), Some(0)), "clamped at the first");

        assert!(viewer.handle_key(KeyCode::Char(' ')));
        assert!(viewer.is_playing());
        viewer.handle_key(KeyCode::Char('.'));
        assert!(!viewer.is_playing(), "stepping pauses");

        viewer.moment = 1;
        let panes = viewer.composites(Built {
            moment: 1,
            ..built(PictureMode::SideBySide)
        });
        assert!(panes[0].0.ends_with("no frame here"), "{}", panes[0].0);
        assert!(panes[0].1.is_none());
        assert!(panes[1].0.ends_with("frame 2"), "{}", panes[1].0);
    }

    /// The engine's pairing is part of its answer, so a human judging the pair steps through the
    /// frames by position.
    #[test]
    fn annotating_pairs_frames_by_position_and_says_nothing_about_them() {
        let dir = tempfile::tempdir().expect("dir");
        let (a, b) = animated_pair(&dir);
        let mut viewer =
            PictureViewer::open_for_annotation(&a, &b, Picker::halfblocks()).expect("a pair");
        viewer.handle_key(KeyCode::Char('.'));
        assert_eq!(sides(&viewer), (Some(1), Some(1)));
        assert!(!viewer.status().contains("added"), "{}", viewer.status());
        viewer.handle_key(KeyCode::Char('.'));
        viewer.handle_key(KeyCode::Char('.'));
        assert_eq!(sides(&viewer), (None, Some(3)));

        viewer.set_annotating(false);
        assert_eq!(
            sides(&viewer),
            (Some(2), Some(3)),
            "showing the engine's pairing keeps the after frame in view"
        );
    }

    #[test]
    fn a_still_pair_leaves_the_frame_keys_to_the_app() {
        let dir = tempfile::tempdir().expect("dir");
        let (a, b) = pair(&dir);
        let mut viewer = viewer(&a, &b);
        for key in [',', '.', ' '] {
            assert!(!viewer.handle_key(KeyCode::Char(key)), "{key:?}");
        }
        assert!(!viewer.status().contains("frame"), "{}", viewer.status());
    }

    fn built(mode: PictureMode) -> Built {
        Built {
            half_blocks: false,
            mode,
            swipe_percent: 50,
            moment: 0,
            outline: 1,
            colors: COLORS,
        }
    }

    #[test]
    fn t_cycles_the_views_and_the_divider_moves_only_in_swipe() {
        let dir = tempfile::tempdir().expect("dir");
        let (a, b) = pair(&dir);
        let mut viewer = viewer(&a, &b);
        assert!(
            !viewer.handle_key(KeyCode::Char('l')),
            "no divider outside swipe"
        );
        for expected in [
            PictureMode::Difference,
            PictureMode::Blend,
            PictureMode::Swipe,
        ] {
            assert!(viewer.handle_key(KeyCode::Char('t')));
            assert_eq!(viewer.mode(), expected);
        }
        assert!(viewer.handle_key(KeyCode::Right));
        assert_eq!(viewer.swipe_percent, 55);
        assert!(viewer.handle_key(KeyCode::Char('t')));
        assert_eq!(viewer.mode(), PictureMode::SideBySide);
        assert!(!viewer.handle_key(KeyCode::Char('x')));
    }

    #[test]
    fn side_by_side_outlines_each_region_in_its_sides_color() {
        let dir = tempfile::tempdir().expect("dir");
        let (a, b) = pair(&dir);
        let panes = viewer(&a, &b).composites(built(PictureMode::SideBySide));
        let [(_, Some(before)), (_, Some(after))] = &panes[..] else {
            panic!("two panes, both pictures: {}", panes.len());
        };
        // The outline runs one pixel outside the 4x3 region at (6, 2).
        assert_eq!(before.get_pixel(5, 1).0, [200, 0, 0, 255]);
        assert_eq!(after.get_pixel(10, 5).0, [0, 200, 0, 255]);
        assert_eq!(
            after.get_pixel(7, 3).0,
            [0, 0, 0, 255],
            "inside is left alone"
        );
        assert_eq!(
            after.get_pixel(0, 9).0,
            [255, 255, 255, 255],
            "far away too"
        );
    }

    #[test]
    fn difference_paints_changed_pixels_over_the_faded_picture() {
        let dir = tempfile::tempdir().expect("dir");
        let (a, b) = pair(&dir);
        let panes = viewer(&a, &b).composites(built(PictureMode::Difference));
        let [(_, Some(difference))] = &panes[..] else {
            panic!("one pane");
        };
        assert_eq!(difference.get_pixel(7, 3).0, [250, 150, 0, 255]);
        assert_eq!(difference.get_pixel(0, 0).0, [254, 254, 254, 255]);
    }

    #[test]
    fn swipe_shows_before_left_of_the_divider_and_blend_mixes_the_two() {
        let dir = tempfile::tempdir().expect("dir");
        let (a, b) = pair(&dir);
        let mut viewer = viewer(&a, &b);
        let mut at = built(PictureMode::Swipe);
        at.swipe_percent = 80; // the divider at x=16, right of the block
        let panes = viewer.composites(at);
        let [(_, Some(swiped))] = &panes[..] else {
            panic!("one pane")
        };
        assert_eq!(
            swiped.get_pixel(7, 3).0,
            [255, 255, 255, 255],
            "before: no block yet"
        );
        let [r, g, b] = DIVIDER;
        assert_eq!(swiped.get_pixel(16, 3).0, [r, g, b, 255], "the divider");

        viewer.mode = PictureMode::Blend;
        let panes = viewer.composites(built(PictureMode::Blend));
        let [(_, Some(blended))] = &panes[..] else {
            panic!("one pane")
        };
        assert_eq!(blended.get_pixel(7, 3).0, [127, 127, 127, 255]);
    }

    #[test]
    fn a_resized_pair_still_blends_and_an_added_picture_shows_one_side() {
        let dir = tempfile::tempdir().expect("dir");
        let small = png(
            &RgbaImage::from_pixel(4, 4, Rgba([0, 0, 0, 255])),
            &dir,
            "s.png",
        );
        let large = png(
            &RgbaImage::from_pixel(8, 8, Rgba([255, 255, 255, 255])),
            &dir,
            "l.png",
        );
        let resized = viewer(&small, &large);
        assert!(resized.status().contains("resized"), "{}", resized.status());
        let panes = resized.composites(built(PictureMode::Blend));
        assert!(matches!(&panes[..], [(_, Some(blended))] if blended.dimensions() == (8, 8)));

        let empty = dir.path().join("empty.png");
        std::fs::write(&empty, b"").expect("writes");
        let added = viewer(&empty, &large);
        assert!(added.status().contains("added"), "{}", added.status());
        let panes = added.composites(built(PictureMode::Difference));
        assert!(
            matches!(&panes[..], [(_, None), (_, Some(_))]),
            "{}",
            panes.len()
        );
    }

    #[test]
    fn annotating_shows_nothing_the_engine_decided() {
        let dir = tempfile::tempdir().expect("dir");
        let (a, b) = pair(&dir);
        let mut viewer =
            PictureViewer::open_for_annotation(&a, &b, Picker::halfblocks()).expect("a pair");
        assert!(!viewer.status().contains("changed"), "{}", viewer.status());
        let panes = viewer.composites(built(PictureMode::SideBySide));
        let [(_, Some(before)), _] = &panes[..] else {
            panic!("two panes")
        };
        assert_eq!(before.get_pixel(5, 1).0, [255, 255, 255, 255], "no outline");
        viewer.handle_key(KeyCode::Char('t'));
        assert_eq!(viewer.mode(), PictureMode::Blend, "difference is skipped");

        // Switched off, the engine's view is back: outlines and the change summary.
        viewer.set_annotating(false);
        assert!(viewer.status().contains("changed"), "{}", viewer.status());
        let panes = viewer.composites(built(PictureMode::SideBySide));
        let [(_, Some(before)), _] = &panes[..] else {
            panic!("two panes")
        };
        assert_eq!(before.get_pixel(5, 1).0, [200, 0, 0, 255], "outlined again");
        // And on again, the difference view is left.
        viewer.mode = PictureMode::Difference;
        viewer.set_annotating(true);
        assert_eq!(viewer.mode(), PictureMode::SideBySide);
    }

    #[test]
    fn a_pair_that_is_not_two_pictures_is_not_opened() {
        let dir = tempfile::tempdir().expect("dir");
        let (a, _) = pair(&dir);
        let text = dir.path().join("a.txt");
        std::fs::write(&text, "text").expect("writes");
        assert!(PictureViewer::open(&a, &text, Picker::halfblocks()).is_none());
    }
}
