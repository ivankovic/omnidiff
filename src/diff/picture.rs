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

//! Pictures: what changed between two raster images, for the files a code diff cannot read.
//!
//! The change census (`research/data/corpus_stats/change_census.csv`) puts images at 39% of the
//! binary files commits change (fonts, the other big share, come from about thirty repositories),
//! and PNG in 8.4% of repositories - the most widespread binary format by far. A picture pair gets what a reader asks first: whether the picture changed size
//! or format, how much of it changed, and where.
//!
//! **Which pixels changed** is pixelmatch's perceptual test (Mapbox's `pixelmatch`, after
//! Kotsarenko and Ramos, "Measuring perceived color difference using YIQ NTSC transmission color
//! space"): both pixels blended over white by their alpha, the YIQ distance compared against
//! `THRESHOLD` of its maximum. Unlike exact equality it does not report re-encoding noise as change,
//! and unlike a per-channel tolerance it weighs brightness as the eye does.
//!
//! **Where** is the changed pixels grouped into rectangles: pixels within `REGION_GAP` of each other
//! belong to one region, so one edited icon is one region rather than a scatter of its pixels.
//!
//! **Animations** (GIF, APNG, animated WebP) are compared frame by frame, the frames aligned the
//! way a text diff aligns lines: equal frames (by a hash of their pixels) match first, and the
//! frames left between matches are paired in order and compared as pictures, or reported inserted
//! or deleted where one side has more. Two-minute terminal recordings run to hundreds of
//! megabytes of raw pixels, so the frames are kept deflated ([`Frames`]) and unpacked one at a
//! time.
//!
//! **The verdict** ([`PictureDiff::verdict`], levels and tags in `content::Level`): pixels equal
//! once fully transparent ones count as alike are invisible; pixels that differ but pass
//! pixelmatch's test are imperceptible; changes no pixel of which passes `STRONG_THRESHOLD` are
//! artifacts - faint everywhere, as compression or resampling leaves them; more than
//! [`REPLACED_SHARE`] of the pixels changed is redrawn if the layout stayed (`layout_kept`) and
//! replaced if not; anything else is edited. The thresholds are guesses for the picture fixtures to
//! calibrate.
//!
//! Pictures of different sizes are not compared pixel by pixel - nothing says which pixel became
//! which. When they keep their aspect ratio they are compared at the smaller one's size, the
//! larger scaled down to it by averaging, and tagged resized; otherwise they are tagged canvas (a crop or a
//! padding, not found) and called edited, as nothing was compared. SVG is not a picture here: it is XML, and the XML grammar
//! diffs it structurally (`code::language::XML_FORMAT_EXTENSIONS`).

use std::borrow::Cow;
use std::hash::Hasher;
use std::io::{Read, Write};

use anyhow::{Context, Result};
use image::{AnimationDecoder, GenericImageView, ImageDecoder, RgbaImage};
use serde::Serialize;

/// Extensions treated as pictures, lower-cased: the raster formats `image` decodes here.
pub const PICTURE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "tif", "tiff",
];

/// The share of the largest possible YIQ distance above which two pixels differ: pixelmatch's
/// default.
pub const THRESHOLD: f64 = 0.1;

/// The share of the largest YIQ distance above which a changed pixel is changed clearly, not
/// faintly: a picture none of whose changed pixels passes it shows only artifacts.
pub const STRONG_THRESHOLD: f64 = 0.3;

/// The cells a side of a picture is averaged into for [`layout_kept`].
const LAYOUT_CELLS: u32 = 16;

/// The correlation of two pictures' cell brightnesses, either way round (a dark theme inverts
/// it), from which they keep one layout.
const LAYOUT_CORRELATION: f64 = 0.6;

/// Changed pixels this close (in both directions) belong to one region.
pub const REGION_GAP: u32 = 2;

/// The share of pixels above which a change reads as a different picture rather than an edit of
/// the same one ([`Verdict::Replaced`]): the human's rule too, who calls a picture more than half
/// edited replaced. The human judges the area that looks changed and this counts changed pixels;
/// the picture fixtures measure how far the two agree.
pub const REPLACED_SHARE: f64 = 0.5;

/// What happened to a picture, in one word (see [`super::content::Verdict`], which every kind
/// of content shares).
pub use super::content::Verdict;
use super::content::{Level, Tag};

/// True if `path` names a raster picture by its extension.
pub fn is_picture_path(path: &std::path::Path) -> bool {
    let path = match path.extension() {
        // A fixture is `before.png.test`.
        Some(ext) if ext.eq_ignore_ascii_case("test") => {
            std::path::Path::new(path.file_stem().unwrap_or_default())
        }
        _ => path,
    };
    path.extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .is_some_and(|ext| PICTURE_EXTENSIONS.contains(&ext.as_str()))
}

/// True if `bytes` start like one of the supported raster formats, whatever the file is called:
/// under `git diff` the paths are git's temporary files.
pub fn is_picture(bytes: &[u8]) -> bool {
    use image::ImageFormat::*;
    matches!(
        image::guess_format(bytes),
        Ok(Png | Jpeg | Gif | WebP | Bmp | Ico | Tiff)
    )
}

/// True if a binary pair is a picture pair: every side present is a picture, and one is. An empty
/// side is git's `/dev/null` for an added or deleted file.
pub fn is_picture_pair(before: &[u8], after: &[u8]) -> bool {
    let fits = |bytes: &[u8]| bytes.is_empty() || is_picture(bytes);
    fits(before) && fits(after) && !(before.is_empty() && after.is_empty())
}

/// One side of a picture pair, as the file says it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PictureInfo {
    /// The format the bytes are in, whatever the extension says (`"PNG"`).
    pub format: String,
    pub width: u32,
    pub height: u32,
    /// The stored color type (`"RGBA8"`, `"L8"`): a palette or bit-depth change shows here when
    /// the pixels did not change.
    pub color: String,
    /// The file's size in bytes.
    pub bytes: usize,
    /// How many frames: 1 for a still picture, which leaves this out of the JSON.
    #[serde(skip_serializing_if = "is_one_frame")]
    pub frames: usize,
    /// An animation's running time, its frames' delays added up, in milliseconds; left out of
    /// the JSON for a still picture.
    #[serde(skip_serializing_if = "is_no_time")]
    pub duration_ms: u64,
}

fn is_one_frame(frames: &usize) -> bool {
    *frames <= 1
}

fn is_no_time(duration_ms: &u64) -> bool {
    *duration_ms == 0
}

/// A rectangle of changed pixels, in the pixel coordinates both pictures share.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    /// How many of the rectangle's pixels changed.
    pub changed_pixels: u64,
}

/// What changed between two pictures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Comparison {
    /// One side is missing (an added or deleted file): nothing to compare.
    OneSided,
    /// The same size, compared pixel by pixel. No region means no visible change, even if the
    /// bytes differ (a re-encoding, a metadata change).
    Pixels {
        /// Pixels that pass pixelmatch's test.
        changed_pixels: u64,
        total_pixels: u64,
        regions: Vec<Region>,
        /// Pixels that differ at all, fully transparent ones alike whatever their colour.
        differing_pixels: u64,
        /// Changed pixels past [`STRONG_THRESHOLD`].
        strong_pixels: u64,
        /// True if the two keep one layout ([`layout_kept`]).
        layout_kept: bool,
    },
    /// Different sizes: not compared pixel by pixel.
    Resized {
        /// True if both sides have one aspect ratio, to a pixel.
        aspect_kept: bool,
        /// Two still pictures of one aspect ratio compared at the smaller one's size, the larger
        /// scaled down to it: always [`Comparison::Pixels`], in the smaller one's pixels.
        #[serde(skip_serializing_if = "Option::is_none")]
        scaled: Option<Box<Comparison>>,
    },
    /// Animations of the same size (or an animation and a still), compared frame by frame.
    Frames {
        /// The aligned frames, in order: runs of frames that look the same, single changed
        /// frames, and runs only one side has.
        steps: Vec<FrameStep>,
        /// Pixels in one frame.
        total_pixels: u64,
        /// True if frames that look the same show for different times.
        retimed: bool,
        /// True if frames that look the same differ in their pixels nonetheless.
        faint: bool,
    },
}

/// One step of an animation diff. Frame numbers count from 0, in each side's own frames.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FrameStep {
    /// `frames` frames that look the same, before's from `before` on and after's from `after` on.
    Same {
        before: usize,
        after: usize,
        frames: usize,
    },
    /// Before's frame `before` became after's frame `after`, with these pixels changed.
    Changed {
        before: usize,
        after: usize,
        changed_pixels: u64,
        regions: Vec<Region>,
        /// Changed pixels past [`STRONG_THRESHOLD`].
        strong_pixels: u64,
    },
    /// `frames` frames only after has, from its frame `after` on.
    Inserted { after: usize, frames: usize },
    /// `frames` frames only before has, from its frame `before` on.
    Deleted { before: usize, frames: usize },
}

/// The whole picture diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PictureDiff {
    pub before: Option<PictureInfo>,
    pub after: Option<PictureInfo>,
    pub comparison: Comparison,
}

impl PictureDiff {
    /// The engine's verdict (see the module doc). An animation counts its frames: inserted or
    /// deleted ones whole and changed ones by their changed share, and is replaced past
    /// [`REPLACED_SHARE`] of its frames; then edited if a frame was added, removed or clearly
    /// changed, artifacts if frames changed only faintly, and imperceptible or invisible as its
    /// frames that look the same differ or not. It is never redrawn. An added or deleted picture,
    /// which the fixtures do not hold, is edited.
    pub fn verdict(&self) -> Verdict {
        match &self.comparison {
            Comparison::OneSided => Verdict::new(Level::Edited),
            Comparison::Pixels { .. } => Verdict::new(pixel_level(&self.comparison)),
            Comparison::Resized {
                aspect_kept,
                scaled,
            } => {
                let level = scaled.as_deref().map_or(Level::Edited, pixel_level);
                let tag = if *aspect_kept {
                    Tag::Resized
                } else {
                    Tag::Canvas
                };
                Verdict::new(level).with(tag)
            }
            Comparison::Frames {
                steps,
                total_pixels,
                retimed,
                faint,
            } => {
                // An inserted or deleted frame counts whole, a changed one by its changed share.
                let (mut positions, mut edited) = (0usize, 0f64);
                let (mut added_or_removed, mut clear, mut changed) = (false, false, false);
                for step in steps {
                    match step {
                        FrameStep::Same { frames, .. } => positions += frames,
                        FrameStep::Changed {
                            changed_pixels,
                            strong_pixels,
                            ..
                        } => {
                            positions += 1;
                            edited += *changed_pixels as f64 / (*total_pixels).max(1) as f64;
                            changed = true;
                            clear |= *strong_pixels > 0;
                        }
                        FrameStep::Inserted { frames, .. } | FrameStep::Deleted { frames, .. } => {
                            positions += frames;
                            edited += *frames as f64;
                            added_or_removed = true;
                        }
                    }
                }
                let level = if edited > REPLACED_SHARE * positions as f64 {
                    Level::Replaced
                } else if added_or_removed || clear {
                    Level::Edited
                } else if changed {
                    Level::Artifacts
                } else if *faint {
                    Level::Imperceptible
                } else {
                    Level::Invisible
                };
                let mut verdict = Verdict::new(level);
                if *retimed {
                    verdict = verdict.with(Tag::Timing);
                }
                if added_or_removed {
                    verdict = verdict.with(Tag::Frames);
                }
                verdict
            }
        }
    }

    /// True if a reader would see a difference: a side added or deleted, a size, format or color
    /// change, or changed pixels.
    pub fn differs(&self) -> bool {
        let format_or_color = || {
            let (Some(before), Some(after)) = (&self.before, &self.after) else {
                return true;
            };
            before.format != after.format || before.color != after.color
        };
        match &self.comparison {
            Comparison::OneSided | Comparison::Resized { .. } => true,
            Comparison::Frames { steps, retimed, .. } => {
                *retimed
                    || steps
                        .iter()
                        .any(|step| !matches!(step, FrameStep::Same { .. }))
                    || format_or_color()
            }
            Comparison::Pixels { regions, .. } => !regions.is_empty() || format_or_color(),
        }
    }
}

/// A picture's frames: one for a still, every frame of an animation, each the whole canvas as it
/// looks once that frame is drawn.
#[derive(Debug, Clone)]
pub struct Frames {
    width: u32,
    height: u32,
    store: FrameStore,
}

#[derive(Debug, Clone)]
enum FrameStore {
    Still(RgbaImage),
    Animation(Vec<PackedFrame>),
}

/// One animation frame: its RGBA pixels deflated, a hash of them (equal frames align by it), and
/// how long it shows.
#[derive(Debug, Clone)]
struct PackedFrame {
    pixels: Vec<u8>,
    hash: u64,
    delay_ms: u32,
}

impl Frames {
    /// A picture of `frames`, each with how long it shows in milliseconds: a still for one
    /// frame, an animation for more. Every frame is the same size, the first one's.
    pub fn from_frames(frames: Vec<(RgbaImage, u32)>) -> Result<Self> {
        let Some((first, _)) = frames.first() else {
            anyhow::bail!("a picture with no frame");
        };
        let (width, height) = first.dimensions();
        if frames.len() == 1 {
            let (pixels, _) = frames.into_iter().next().expect("one frame");
            return Ok(Self::still(pixels));
        }
        let mut packed = Vec::with_capacity(frames.len());
        for (pixels, delay_ms) in &frames {
            if pixels.dimensions() != (width, height) {
                anyhow::bail!("frames of different sizes");
            }
            packed.push(pack(pixels, *delay_ms)?);
        }
        Ok(Self {
            width,
            height,
            store: FrameStore::Animation(packed),
        })
    }

    /// An animation's running time, its frames' delays added up.
    pub fn total_ms(&self) -> u64 {
        self.duration_ms()
    }

    fn still(pixels: RgbaImage) -> Self {
        let (width, height) = pixels.dimensions();
        Self {
            width,
            height,
            store: FrameStore::Still(pixels),
        }
    }

    /// How many frames; at least 1.
    pub fn count(&self) -> usize {
        match &self.store {
            FrameStore::Still(_) => 1,
            FrameStore::Animation(frames) => frames.len(),
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Frame `index`'s pixels, unpacked for an animation; the last frame past the end.
    pub fn pixels(&self, index: usize) -> Cow<'_, RgbaImage> {
        match &self.store {
            FrameStore::Still(pixels) => Cow::Borrowed(pixels),
            FrameStore::Animation(frames) => {
                let frame = &frames[index.min(frames.len() - 1)];
                let mut raw = Vec::with_capacity(self.width as usize * self.height as usize * 4);
                flate2::read::DeflateDecoder::new(frame.pixels.as_slice())
                    .read_to_end(&mut raw)
                    .expect("inflating a frame this process deflated");
                Cow::Owned(
                    RgbaImage::from_raw(self.width, self.height, raw)
                        .expect("a frame inflates to its canvas size"),
                )
            }
        }
    }

    /// How long frame `index` shows, in milliseconds; 0 for a still picture.
    pub fn delay_ms(&self, index: usize) -> u32 {
        match &self.store {
            FrameStore::Still(_) => 0,
            FrameStore::Animation(frames) => frames[index.min(frames.len() - 1)].delay_ms,
        }
    }

    fn hash(&self, index: usize) -> u64 {
        match &self.store {
            FrameStore::Still(pixels) => hash_pixels(pixels),
            FrameStore::Animation(frames) => frames[index].hash,
        }
    }

    fn duration_ms(&self) -> u64 {
        (0..self.count())
            .map(|index| u64::from(self.delay_ms(index)))
            .sum()
    }
}

fn hash_pixels(pixels: &RgbaImage) -> u64 {
    let mut hasher = metrohash::MetroHash64::default();
    hasher.write(pixels.as_raw());
    hasher.finish()
}

fn pack(pixels: &RgbaImage, delay_ms: u32) -> Result<PackedFrame> {
    let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(pixels.as_raw())?;
    Ok(PackedFrame {
        pixels: encoder.finish()?,
        hash: hash_pixels(pixels),
        delay_ms,
    })
}

/// Decodes `bytes` as a picture: its description and its frames as 8-bit RGBA - one for a still
/// picture, all of them for an animated GIF, PNG or WebP.
pub fn decode(bytes: &[u8]) -> Result<(PictureInfo, Frames)> {
    use image::ImageFormat;
    // Cursors are pictures `image` does not know by their bytes (`content::cursor`).
    if super::content::cursor::is_cur(bytes) {
        return super::content::cursor::decode_cur(bytes);
    }
    if super::content::cursor::is_ani(bytes) {
        return super::content::cursor::decode_ani(bytes);
    }
    use image::codecs::{gif::GifDecoder, png::PngDecoder, webp::WebPDecoder};
    let cursor = || std::io::Cursor::new(bytes);
    let format = image::guess_format(bytes).context("reading the picture")?;
    let animation = match format {
        ImageFormat::Gif => {
            let decoder = GifDecoder::new(cursor()).context("reading the GIF")?;
            Some((decoder.color_type(), decoder.into_frames()))
        }
        ImageFormat::Png => {
            let decoder = PngDecoder::new(cursor()).context("reading the PNG")?;
            if decoder.is_apng().context("reading the PNG")? {
                let color = decoder.color_type();
                Some((
                    color,
                    decoder.apng().context("reading the APNG")?.into_frames(),
                ))
            } else {
                None
            }
        }
        ImageFormat::WebP => {
            let decoder = WebPDecoder::new(cursor()).context("reading the WebP")?;
            if decoder.has_animation() {
                Some((decoder.color_type(), decoder.into_frames()))
            } else {
                None
            }
        }
        _ => None,
    };
    let info = |width, height, color: image::ColorType, frames, duration_ms| PictureInfo {
        format: format!("{format:?}").to_uppercase(),
        width,
        height,
        color: format!("{color:?}").to_uppercase(),
        bytes: bytes.len(),
        frames,
        duration_ms,
    };
    if let Some((color, decoded)) = animation {
        // One frame at a time: packed as it comes, never all unpacked at once.
        let mut packed = Vec::new();
        let mut size = (0, 0);
        for frame in decoded {
            let frame = frame.context("decoding a frame")?;
            let (numerator, denominator) = frame.delay().numer_denom_ms();
            let pixels = frame.into_buffer();
            size = pixels.dimensions();
            packed.push(pack(&pixels, numerator / denominator.max(1))?);
        }
        if packed.len() > 1 {
            let frames = Frames {
                width: size.0,
                height: size.1,
                store: FrameStore::Animation(packed),
            };
            let info = info(size.0, size.1, color, frames.count(), frames.duration_ms());
            return Ok((info, frames));
        }
    }
    let picture =
        image::load_from_memory_with_format(bytes, format).context("decoding the picture")?;
    let (width, height) = picture.dimensions();
    let info = info(width, height, picture.color(), 1, 0);
    Ok((info, Frames::still(picture.to_rgba8())))
}

/// Compares two pictures; an empty side (git's `/dev/null` for an added or deleted file) is
/// absent rather than an error.
pub fn diff(before: &[u8], after: &[u8]) -> Result<PictureDiff> {
    let (before, after) = decode_pair(before, after)?;
    Ok(compare_decoded(before.as_ref(), after.as_ref()))
}

/// A decoded side of a pair: `None` for an empty one.
pub type Decoded = Option<(PictureInfo, Frames)>;

/// Both sides [`decode`]d at once, an empty side as `None`: an animation's decoding takes long
/// enough to be worth a second thread.
pub fn decode_pair(before: &[u8], after: &[u8]) -> Result<(Decoded, Decoded)> {
    let side = |bytes: &[u8]| (!bytes.is_empty()).then(|| decode(bytes)).transpose();
    let (before, after) = std::thread::scope(|scope| {
        let before = scope.spawn(|| side(before));
        let after = side(after);
        (before.join().expect("decoding a picture panicked"), after)
    });
    Ok((before?, after?))
}

/// [`diff`] of pictures already decoded, for a caller that keeps the frames to show them.
pub fn compare_decoded(
    before: Option<&(PictureInfo, Frames)>,
    after: Option<&(PictureInfo, Frames)>,
) -> PictureDiff {
    let comparison = match (before, after) {
        (Some((_, b)), Some((_, a))) if b.dimensions() != a.dimensions() => resized(b, a),
        (Some((_, b)), Some((_, a))) if b.count() == 1 && a.count() == 1 => {
            compare(&b.pixels(0), &a.pixels(0))
        }
        (Some((_, b)), Some((_, a))) => compare_frames(b, a),
        _ => Comparison::OneSided,
    };
    PictureDiff {
        before: before.map(|(info, _)| info.clone()),
        after: after.map(|(info, _)| info.clone()),
        comparison,
    }
}

/// The frames of two same-size pictures aligned and compared (see the module doc).
fn compare_frames(before: &Frames, after: &Frames) -> Comparison {
    let hashes = |frames: &Frames| {
        (0..frames.count())
            .map(|i| frames.hash(i))
            .collect::<Vec<_>>()
    };
    let matched = align(&hashes(before), &hashes(after));
    // First the plan: which frames match by hash, which pair up to be compared, and which only
    // one side has. The comparisons then run together.
    let mut plan = Vec::new();
    let (mut i, mut j) = (0, 0);
    // The end of both sides closes the last gap.
    for (b, a) in matched.into_iter().chain([(before.count(), after.count())]) {
        let paired = (b - i).min(a - j);
        plan.extend((0..paired).map(|k| Planned::Compare(i + k, j + k)));
        if b - i > paired {
            plan.push(Planned::Step(FrameStep::Deleted {
                before: i + paired,
                frames: b - i - paired,
            }));
        }
        if a - j > paired {
            plan.push(Planned::Step(FrameStep::Inserted {
                after: j + paired,
                frames: a - j - paired,
            }));
        }
        if b < before.count() {
            plan.push(Planned::Same(b, a));
        }
        (i, j) = (b + 1, a + 1);
    }
    let pairs: Vec<(usize, usize)> = plan
        .iter()
        .filter_map(|planned| match planned {
            Planned::Compare(b, a) => Some((*b, *a)),
            _ => None,
        })
        .collect();
    let mut compared = compare_pairs(before, after, &pairs).into_iter();

    let mut steps = Vec::new();
    let (mut retimed, mut faint) = (false, false);
    for planned in plan {
        match planned {
            Planned::Same(b, a) => {
                retimed |= before.delay_ms(b) != after.delay_ms(a);
                push_same(&mut steps, b, a);
            }
            Planned::Compare(b, a) => {
                let changes = compared.next().expect("one result per pair");
                if changes.regions.is_empty() {
                    retimed |= before.delay_ms(b) != after.delay_ms(a);
                    faint |= changes.differing > 0;
                    push_same(&mut steps, b, a);
                } else {
                    steps.push(FrameStep::Changed {
                        before: b,
                        after: a,
                        changed_pixels: changes.changed,
                        regions: changes.regions,
                        strong_pixels: changes.strong,
                    });
                }
            }
            Planned::Step(step) => steps.push(step),
        }
    }
    let (width, height) = after.dimensions();
    Comparison::Frames {
        steps,
        total_pixels: u64::from(width) * u64::from(height),
        retimed,
        faint,
    }
}

/// How an animation diff's frames add up, counted in after's frames (before's for deleted ones).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameCounts {
    pub same: usize,
    pub changed: usize,
    pub inserted: usize,
    pub deleted: usize,
}

impl FrameCounts {
    pub fn of(steps: &[FrameStep]) -> Self {
        let mut counts = Self::default();
        for step in steps {
            match step {
                FrameStep::Same { frames, .. } => counts.same += frames,
                FrameStep::Changed { .. } => counts.changed += 1,
                FrameStep::Inserted { frames, .. } => counts.inserted += frames,
                FrameStep::Deleted { frames, .. } => counts.deleted += frames,
            }
        }
        counts
    }

    /// "12 frames changed, 3 added", or "no frame changed".
    pub fn describe(self) -> String {
        let parts: Vec<(usize, &str)> = [
            (self.changed, "changed"),
            (self.inserted, "added"),
            (self.deleted, "removed"),
        ]
        .into_iter()
        .filter(|(count, _)| *count > 0)
        .collect();
        let Some(&(first, what)) = parts.first() else {
            return "no frame changed".to_string();
        };
        // "1 frame changed, 2 added": the noun goes after the first count, and agrees with it.
        let noun = if first == 1 { "frame" } else { "frames" };
        std::iter::once(format!("{first} {noun} {what}"))
            .chain(
                parts[1..]
                    .iter()
                    .map(|(count, what)| format!("{count} {what}")),
            )
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// One entry of [`compare_frames`]'s plan.
enum Planned {
    /// Equal by hash.
    Same(usize, usize),
    /// Paired in a gap between matches, to compare pixel by pixel.
    Compare(usize, usize),
    /// Frames only one side has.
    Step(FrameStep),
}

/// [`changes`] of each (before, after) frame pair, in order, on every core: unpacking and
/// comparing frames is most of an animation diff's time.
fn compare_pairs(before: &Frames, after: &Frames, pairs: &[(usize, usize)]) -> Vec<Changes> {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let chunk = pairs.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let workers: Vec<_> = pairs
            .chunks(chunk)
            .map(|slice| {
                scope.spawn(move || {
                    slice
                        .iter()
                        .map(|&(b, a)| changes(&before.pixels(b), &after.pixels(a)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("comparing frames panicked"))
            .collect()
    })
}

/// Adds the frame pair (`before`, `after`) to `steps`, extending the last run when it follows on.
fn push_same(steps: &mut Vec<FrameStep>, before: usize, after: usize) {
    if let Some(FrameStep::Same {
        before: run_before,
        after: run_after,
        frames,
    }) = steps.last_mut()
        && *run_before + *frames == before
        && *run_after + *frames == after
    {
        *frames += 1;
        return;
    }
    steps.push(FrameStep::Same {
        before,
        after,
        frames: 1,
    });
}

/// Table cells past which [`align`] leaves the middle of two long animations unmatched rather than
/// spend quadratic time and memory on it: 64 MB of lengths, two animations of 4000 frames.
const ALIGN_CELLS: usize = 16_000_000;

/// The (before, after) index pairs of equal hashes a longest common subsequence matches, in order.
/// Common ends match first, so the table only covers the middle.
fn align(before: &[u64], after: &[u64]) -> Vec<(usize, usize)> {
    let prefix = before.iter().zip(after).take_while(|(b, a)| b == a).count();
    let suffix = before[prefix..]
        .iter()
        .rev()
        .zip(after[prefix..].iter().rev())
        .take_while(|(b, a)| b == a)
        .count();
    let (b, a) = (
        &before[prefix..before.len() - suffix],
        &after[prefix..after.len() - suffix],
    );
    let mut pairs: Vec<(usize, usize)> = (0..prefix).map(|k| (k, k)).collect();
    if !b.is_empty() && !a.is_empty() && b.len() * a.len() <= ALIGN_CELLS {
        // lengths[x * w + y]: the longest common subsequence of b[x..] and a[y..].
        let w = a.len() + 1;
        let mut lengths = vec![0u32; (b.len() + 1) * w];
        for x in (0..b.len()).rev() {
            for y in (0..a.len()).rev() {
                lengths[x * w + y] = if b[x] == a[y] {
                    lengths[(x + 1) * w + y + 1] + 1
                } else {
                    lengths[(x + 1) * w + y].max(lengths[x * w + y + 1])
                };
            }
        }
        let (mut x, mut y) = (0, 0);
        while x < b.len() && y < a.len() {
            if b[x] == a[y] {
                pairs.push((prefix + x, prefix + y));
                (x, y) = (x + 1, y + 1);
            } else if lengths[(x + 1) * w + y] >= lengths[x * w + y + 1] {
                x += 1;
            } else {
                y += 1;
            }
        }
    }
    pairs.extend((0..suffix).map(|k| (before.len() - suffix + k, after.len() - suffix + k)));
    pairs
}

/// Which pixels of two same-size pictures differ, as a row-major mask.
pub fn changed_mask(before: &RgbaImage, after: &RgbaImage) -> Vec<bool> {
    // pixelmatch's maximum YIQ distance, 35215, scaled by the threshold squared.
    let limit = 35215.0 * THRESHOLD * THRESHOLD;
    before
        .pixels()
        .zip(after.pixels())
        .map(|(b, a)| b != a && yiq_distance(b.0, a.0) > limit)
        .collect()
}

fn compare(before: &RgbaImage, after: &RgbaImage) -> Comparison {
    let (width, height) = before.dimensions();
    let changes = changes(before, after);
    Comparison::Pixels {
        changed_pixels: changes.changed,
        total_pixels: u64::from(width) * u64::from(height),
        regions: changes.regions,
        differing_pixels: changes.differing,
        strong_pixels: changes.strong,
        layout_kept: layout_kept(before, after),
    }
}

/// Two pictures of different sizes: compared at the smaller one's size if they are stills of one
/// aspect ratio.
fn resized(before: &Frames, after: &Frames) -> Comparison {
    let ((bw, bh), (aw, ah)) = (before.dimensions(), after.dimensions());
    // One aspect ratio to a pixel: the cross products differ by less than the longest side.
    let cross = |w: u32, h: u32| u64::from(w) * u64::from(h);
    let aspect_kept = cross(bw, ah).abs_diff(cross(aw, bh)) < u64::from(bw.max(bh).max(aw).max(ah));
    let scaled = (aspect_kept && before.count() == 1 && after.count() == 1).then(|| {
        let (b, a) = (before.pixels(0), after.pixels(0));
        let (width, height) = if cross(bw, bh) <= cross(aw, ah) {
            (bw, bh)
        } else {
            (aw, ah)
        };
        let fit = |pixels: &RgbaImage| {
            if pixels.dimensions() == (width, height) {
                pixels.clone()
            } else {
                shrink(pixels, width, height)
            }
        };
        Box::new(compare(&fit(&b), &fit(&a)))
    });
    Comparison::Resized {
        aspect_kept,
        scaled,
    }
}

/// `pixels` scaled down to `width` x `height` by averaging the pixels each new one covers: a
/// picture scaled up by a whole factor and back comes out as it was, which a filter reaching past
/// the covered pixels (`imageops`' triangle and the rest) would smear.
fn shrink(pixels: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    let (from_width, from_height) = pixels.dimensions();
    let span = |at: u32, to: u32, from: u32| {
        let start = u64::from(at) * u64::from(from) / u64::from(to);
        let end = (u64::from(at + 1) * u64::from(from)).div_ceil(u64::from(to));
        start as u32..(end as u32).max(start as u32 + 1)
    };
    RgbaImage::from_fn(width, height, |x, y| {
        let (mut sum, mut count) = ([0u64; 4], 0u64);
        for sy in span(y, height, from_height) {
            for sx in span(x, width, from_width) {
                for (total, channel) in sum.iter_mut().zip(pixels.get_pixel(sx, sy).0) {
                    *total += u64::from(channel);
                }
                count += 1;
            }
        }
        image::Rgba(sum.map(|total| ((total + count / 2) / count) as u8))
    })
}

/// The level of a [`Comparison::Pixels`] (see the module doc).
fn pixel_level(comparison: &Comparison) -> Level {
    let Comparison::Pixels {
        changed_pixels,
        total_pixels,
        differing_pixels,
        strong_pixels,
        layout_kept,
        ..
    } = comparison
    else {
        return Level::Edited;
    };
    if *differing_pixels == 0 {
        Level::Invisible
    } else if *changed_pixels == 0 {
        Level::Imperceptible
    } else if *strong_pixels == 0 {
        Level::Artifacts
    } else if *changed_pixels as f64 <= REPLACED_SHARE * *total_pixels as f64 {
        Level::Edited
    } else if *layout_kept {
        Level::Redrawn
    } else {
        Level::Replaced
    }
}

/// How two same-size pictures differ.
struct Changes {
    /// Pixels that differ at all, fully transparent ones alike whatever their colour.
    differing: u64,
    /// Pixels that pass pixelmatch's test.
    changed: u64,
    /// Changed pixels past [`STRONG_THRESHOLD`].
    strong: u64,
    /// The changed pixels' regions.
    regions: Vec<Region>,
}

/// How many pixels of two same-size pictures differ, changed and changed clearly, and the regions
/// the changed ones make.
fn changes(before: &RgbaImage, after: &RgbaImage) -> Changes {
    let (width, height) = before.dimensions();
    let limit = 35215.0 * THRESHOLD * THRESHOLD;
    let strong_limit = 35215.0 * STRONG_THRESHOLD * STRONG_THRESHOLD;
    let (mut differing, mut strong) = (0, 0);
    let mask: Vec<bool> = before
        .pixels()
        .zip(after.pixels())
        .map(|(b, a)| {
            if !looks_alike(b.0, a.0) {
                differing += 1;
            }
            if b == a {
                return false;
            }
            let distance = yiq_distance(b.0, a.0);
            if distance > strong_limit {
                strong += 1;
            }
            distance > limit
        })
        .collect();
    let changed = mask.iter().filter(|&&changed| changed).count() as u64;
    Changes {
        differing,
        changed,
        strong,
        regions: regions(&mask, width, height),
    }
}

/// True if two pixels are the same once fully transparent ones count as alike.
fn looks_alike(before: [u8; 4], after: [u8; 4]) -> bool {
    before == after || (before[3] == 0 && after[3] == 0)
}

/// True if two same-size pictures keep one layout: their brightness averaged into
/// [`LAYOUT_CELLS`] cells a side correlates, either way round, by [`LAYOUT_CORRELATION`] or more.
/// Two flat pictures keep one; a flat one and a drawn one do not.
fn layout_kept(before: &RgbaImage, after: &RgbaImage) -> bool {
    let (b, a) = (layout(before), layout(after));
    let n = b.len() as f64;
    let mean = |cells: &[f64]| cells.iter().sum::<f64>() / n;
    let (mb, ma) = (mean(&b), mean(&a));
    let (mut covariance, mut vb, mut va) = (0.0, 0.0, 0.0);
    for (x, y) in b.iter().zip(&a) {
        covariance += (x - mb) * (y - ma);
        vb += (x - mb) * (x - mb);
        va += (y - ma) * (y - ma);
    }
    // A cell a tenth of a level off flat is flat.
    let flat = 0.01 * n;
    match (vb < flat, va < flat) {
        (true, true) => true,
        (true, false) | (false, true) => false,
        (false, false) => (covariance / (vb * va).sqrt()).abs() >= LAYOUT_CORRELATION,
    }
}

/// A picture's brightness over white, averaged into at most [`LAYOUT_CELLS`] cells a side.
fn layout(pixels: &RgbaImage) -> Vec<f64> {
    let (width, height) = pixels.dimensions();
    let (columns, rows) = (width.clamp(1, LAYOUT_CELLS), height.clamp(1, LAYOUT_CELLS));
    let mut sums = vec![(0.0, 0u64); (columns * rows) as usize];
    for (x, y, pixel) in pixels.enumerate_pixels() {
        let [r, g, b, a] = pixel.0;
        let alpha = f64::from(a) / 255.0;
        let over_white = |channel: u8| 255.0 + (f64::from(channel) - 255.0) * alpha;
        let luma = over_white(r) * 0.299 + over_white(g) * 0.587 + over_white(b) * 0.114;
        let cell = (y * rows / height.max(1)) * columns + x * columns / width.max(1);
        let sum = &mut sums[cell as usize];
        sum.0 += luma;
        sum.1 += 1;
    }
    sums.into_iter()
        .map(|(sum, count)| sum / count.max(1) as f64)
        .collect()
}

/// pixelmatch's `colorDelta`: both pixels blended over white, then the weighted distance in YIQ.
fn yiq_distance(before: [u8; 4], after: [u8; 4]) -> f64 {
    let blend = |[r, g, b, a]: [u8; 4]| {
        let alpha = f64::from(a) / 255.0;
        let over_white = |channel: u8| 255.0 + (f64::from(channel) - 255.0) * alpha;
        (over_white(r), over_white(g), over_white(b))
    };
    let (r1, g1, b1) = blend(before);
    let (r2, g2, b2) = blend(after);
    let y = |r: f64, g: f64, b: f64| r * 0.298_895_31 + g * 0.586_622_47 + b * 0.114_482_23;
    let i = |r: f64, g: f64, b: f64| r * 0.595_977_99 - g * 0.274_176_47 - b * 0.321_801_52;
    let q = |r: f64, g: f64, b: f64| r * 0.211_470_17 - g * 0.522_617_24 + b * 0.311_147_07;
    let dy = y(r1, g1, b1) - y(r2, g2, b2);
    let di = i(r1, g1, b1) - i(r2, g2, b2);
    let dq = q(r1, g1, b1) - q(r2, g2, b2);
    0.5053 * dy * dy + 0.299 * di * di + 0.1957 * dq * dq
}

/// The changed pixels of a `width` x `height` mask grouped into rectangles: pixels within
/// `REGION_GAP` of each other are one region. Largest first, then top to bottom.
pub fn regions(mask: &[bool], width: u32, height: u32) -> Vec<Region> {
    let (w, h) = (width as usize, height as usize);
    let gap = REGION_GAP as usize;
    let mut seen = vec![false; mask.len()];
    let mut found = Vec::new();
    let mut stack = Vec::new();
    for start in 0..mask.len() {
        if !mask[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
        let mut changed_pixels = 0u64;
        while let Some(at) = stack.pop() {
            let (x, y) = (at % w, at / w);
            changed_pixels += 1;
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            for ny in y.saturating_sub(gap)..(y + gap + 1).min(h) {
                for nx in x.saturating_sub(gap)..(x + gap + 1).min(w) {
                    let next = ny * w + nx;
                    if mask[next] && !seen[next] {
                        seen[next] = true;
                        stack.push(next);
                    }
                }
            }
        }
        found.push(Region {
            x: x0 as u32,
            y: y0 as u32,
            width: (x1 - x0 + 1) as u32,
            height: (y1 - y0 + 1) as u32,
            changed_pixels,
        });
    }
    found.sort_by_key(|region| {
        (
            std::cmp::Reverse(u64::from(region.width) * u64::from(region.height)),
            region.y,
            region.x,
        )
    });
    found
}

/// A GIF of `frames`, each shown for its milliseconds (GIF keeps tens of them), for tests.
#[cfg(test)]
pub(crate) fn test_gif(frames: &[(RgbaImage, u32)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
        encoder
            .encode_frames(frames.iter().map(|(pixels, ms)| {
                image::Frame::from_parts(
                    pixels.clone(),
                    0,
                    0,
                    image::Delay::from_numer_denom_ms(*ms, 1),
                )
            }))
            .expect("encodes");
    }
    bytes
}

/// For tests: a white 8x8 frame with a black pixel at (`k`, `k`), different for every `k`.
#[cfg(test)]
pub(crate) fn test_frame(k: u32) -> RgbaImage {
    let mut pixels = RgbaImage::from_pixel(8, 8, image::Rgba([255, 255, 255, 255]));
    pixels.put_pixel(k % 8, k % 8, image::Rgba([0, 0, 0, 255]));
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Verdict {
        text.parse().expect("a verdict")
    }

    fn diff_of(before: &RgbaImage, after: &RgbaImage) -> PictureDiff {
        diff(&png(before), &png(after)).expect("diffs")
    }

    fn png(picture: &RgbaImage) -> Vec<u8> {
        let mut bytes = Vec::new();
        picture
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .expect("encodes");
        bytes
    }

    fn filled(width: u32, height: u32, color: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(width, height, image::Rgba(color))
    }

    #[test]
    fn identical_pictures_have_no_regions() {
        let picture = png(&filled(8, 8, [10, 20, 30, 255]));
        let diff = diff(&picture, &picture).expect("diffs");
        assert_eq!(
            diff.comparison,
            Comparison::Pixels {
                changed_pixels: 0,
                total_pixels: 64,
                regions: Vec::new(),
                differing_pixels: 0,
                strong_pixels: 0,
                layout_kept: true,
            }
        );
        assert!(!diff.differs());
        assert_eq!(
            diff.before.as_ref().map(|info| info.format.as_str()),
            Some("PNG")
        );
    }

    #[test]
    fn a_changed_block_is_one_region_where_it_changed() {
        let before = filled(20, 10, [255, 255, 255, 255]);
        let mut after = before.clone();
        for y in 2..5 {
            for x in 6..10 {
                after.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
            }
        }
        let diff = diff(&png(&before), &png(&after)).expect("diffs");
        let Comparison::Pixels {
            changed_pixels,
            regions,
            ..
        } = diff.comparison
        else {
            panic!("same size, so compared by pixels");
        };
        assert_eq!(changed_pixels, 12);
        assert_eq!(
            regions,
            vec![Region {
                x: 6,
                y: 2,
                width: 4,
                height: 3,
                changed_pixels: 12
            }]
        );
    }

    #[test]
    fn far_apart_changes_are_separate_regions_and_near_ones_merge() {
        let mut mask = vec![false; 30 * 3];
        for x in [0, 2, 20] {
            mask[x] = true;
        }
        let found = regions(&mask, 30, 3);
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!(
            (found[0].x, found[0].width, found[0].changed_pixels),
            (0, 3, 2)
        );
        assert_eq!((found[1].x, found[1].width), (20, 1));
    }

    #[test]
    fn a_change_below_the_perceptual_threshold_is_not_a_change() {
        let before = filled(4, 4, [100, 100, 100, 255]);
        let after = filled(4, 4, [101, 100, 100, 255]);
        let diff = diff(&png(&before), &png(&after)).expect("diffs");
        assert!(matches!(
            diff.comparison,
            Comparison::Pixels {
                changed_pixels: 0,
                ..
            }
        ));
    }

    #[test]
    fn transparent_pixels_compare_as_what_they_look_like_over_white() {
        // Fully transparent black and fully transparent red are both just white.
        let before = filled(2, 2, [0, 0, 0, 0]);
        let after = filled(2, 2, [255, 0, 0, 0]);
        assert!(changed_mask(&before, &after).iter().all(|changed| !changed));
    }

    #[test]
    fn a_reshaped_picture_is_not_compared_pixel_by_pixel() {
        let diff = diff(&png(&filled(4, 4, [0; 4])), &png(&filled(8, 4, [0; 4]))).expect("diffs");
        assert_eq!(
            diff.comparison,
            Comparison::Resized {
                aspect_kept: false,
                scaled: None
            }
        );
        assert_eq!(
            diff.after.map(|info| (info.width, info.height)),
            Some((8, 4))
        );
    }

    #[test]
    fn a_rescaled_picture_is_compared_at_the_smaller_size() {
        let mut small = filled(4, 4, [255; 4]);
        small.put_pixel(1, 1, image::Rgba([0, 0, 0, 255]));
        // Twice the size, each pixel a 2x2 block: scaled back down, the same picture.
        let large = image::imageops::resize(&small, 8, 8, image::imageops::FilterType::Nearest);
        let diff = diff(&png(&small), &png(&large)).expect("diffs");
        let Comparison::Resized {
            aspect_kept: true,
            scaled: Some(scaled),
        } = &diff.comparison
        else {
            panic!("compared at one size: {:?}", diff.comparison);
        };
        assert!(matches!(
            **scaled,
            Comparison::Pixels {
                total_pixels: 16,
                ..
            }
        ));
        assert_eq!(diff.verdict(), v("invisible+resized"));
        // Off by a pixel of rounding is still one aspect ratio.
        let other = diff_of(&filled(100, 75, [255; 4]), &filled(33, 25, [255; 4]));
        assert_eq!(other.verdict(), v("invisible+resized"));
    }

    #[test]
    fn an_empty_side_is_an_added_or_deleted_picture() {
        let diff = diff(b"", &png(&filled(2, 2, [0; 4]))).expect("diffs");
        assert_eq!(diff.comparison, Comparison::OneSided);
        assert!(diff.before.is_none() && diff.differs());
    }

    #[test]
    fn a_picture_pair_is_known_by_its_bytes() {
        let picture = png(&filled(2, 2, [0; 4]));
        assert!(is_picture_pair(&picture, &picture));
        assert!(is_picture_pair(b"", &picture), "an added picture");
        assert!(
            !is_picture_pair(&picture, b"%PDF-1.7\n"),
            "one side is not a picture"
        );
        assert!(!is_picture_pair(b"", b""));
    }

    /// Frames `ks` of [`test_frame`], each shown for `ms`.
    fn animation(ks: &[u32], ms: u32) -> Vec<u8> {
        let frames: Vec<_> = ks.iter().map(|&k| (test_frame(k), ms)).collect();
        test_gif(&frames)
    }

    fn steps_of(diff: &PictureDiff) -> (&[FrameStep], bool) {
        let Comparison::Frames { steps, retimed, .. } = &diff.comparison else {
            panic!(
                "an animation is compared frame by frame: {:?}",
                diff.comparison
            )
        };
        (steps, *retimed)
    }

    #[test]
    fn an_animation_decodes_every_frame_with_its_time() {
        let bytes = test_gif(&[
            (test_frame(0), 100),
            (test_frame(1), 200),
            (test_frame(2), 300),
        ]);
        let (info, frames) = decode(&bytes).expect("decodes");
        assert_eq!((info.frames, info.duration_ms), (3, 600));
        assert_eq!(frames.count(), 3);
        assert_eq!(frames.delay_ms(2), 300);
        assert_eq!(frames.pixels(1).get_pixel(1, 1).0, [0, 0, 0, 255]);
        assert_eq!(frames.pixels(1).get_pixel(0, 0).0, [255, 255, 255, 255]);
    }

    #[test]
    fn an_identical_animation_has_no_visible_change() {
        let bytes = animation(&[0, 1, 2], 100);
        let diff = diff(&bytes, &bytes).expect("diffs");
        let (steps, retimed) = steps_of(&diff);
        assert_eq!(
            steps,
            [FrameStep::Same {
                before: 0,
                after: 0,
                frames: 3
            }]
        );
        assert!(!retimed);
        assert_eq!(diff.verdict(), v("invisible"));
        assert!(!diff.differs());
    }

    #[test]
    fn the_same_frames_at_a_new_pace_are_only_retimed() {
        let diff = diff(&animation(&[0, 1, 2], 100), &animation(&[0, 1, 2], 50)).expect("diffs");
        let (steps, retimed) = steps_of(&diff);
        assert_eq!(steps.len(), 1, "{steps:?}");
        assert!(retimed);
        assert_eq!(diff.verdict(), v("invisible+timing"));
        assert!(diff.differs());
    }

    #[test]
    fn an_added_frame_is_inserted_where_it_was_added() {
        let diff =
            diff(&animation(&[0, 1, 2], 100), &animation(&[0, 5, 1, 2], 100)).expect("diffs");
        let (steps, _) = steps_of(&diff);
        assert_eq!(
            steps,
            [
                FrameStep::Same {
                    before: 0,
                    after: 0,
                    frames: 1
                },
                FrameStep::Inserted {
                    after: 1,
                    frames: 1
                },
                FrameStep::Same {
                    before: 1,
                    after: 2,
                    frames: 2
                },
            ]
        );
        assert_eq!(diff.verdict(), v("edited+frames"));
    }

    #[test]
    fn a_changed_frame_is_compared_pixel_by_pixel() {
        let mut edited = test_frame(1);
        edited.put_pixel(6, 2, image::Rgba([0, 0, 0, 255]));
        let before = animation(&[0, 1, 2], 100);
        let after = test_gif(&[(test_frame(0), 100), (edited, 100), (test_frame(2), 100)]);
        let diff = diff(&before, &after).expect("diffs");
        let (steps, _) = steps_of(&diff);
        let [
            _,
            FrameStep::Changed {
                before: 1,
                after: 1,
                changed_pixels: 1,
                regions,
                strong_pixels: 1,
            },
            _,
        ] = steps
        else {
            panic!("frame 1 changed, the others match: {steps:?}")
        };
        assert_eq!((regions[0].x, regions[0].y), (6, 2));
        assert_eq!(diff.verdict(), v("edited"));
    }

    #[test]
    fn an_animation_with_mostly_new_frames_is_replaced() {
        let diff =
            diff(&animation(&[0, 1, 2], 100), &animation(&[3, 4, 5, 6], 100)).expect("diffs");
        // Paired frames differ in two pixels; the fourth is new: a quarter of the positions
        // edited whole, so edited - not replaced on frame count alone.
        assert_eq!(diff.verdict(), v("edited+frames"));
        let black = RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 0, 255]));
        let blacked = test_gif(&[(black.clone(), 100), (black, 200)]);
        let diff = super::diff(&animation(&[0, 1], 100), &blacked).expect("diffs");
        assert_eq!(diff.verdict(), v("replaced"));
    }

    #[test]
    fn a_still_against_an_animation_is_compared_frame_by_frame() {
        let still = png(&test_frame(0));
        let diff = diff(&still, &animation(&[0, 1], 100)).expect("diffs");
        let (steps, _) = steps_of(&diff);
        assert_eq!(
            steps,
            [
                FrameStep::Same {
                    before: 0,
                    after: 0,
                    frames: 1
                },
                FrameStep::Inserted {
                    after: 1,
                    frames: 1
                },
            ]
        );
    }

    #[test]
    fn frames_align_like_lines() {
        assert_eq!(
            align(&[1, 2, 3, 4], &[1, 3, 5, 4]),
            [(0, 0), (2, 1), (3, 3)]
        );
        assert_eq!(align(&[1, 2], &[3, 4]), []);
        assert_eq!(align(&[7, 7, 7], &[7, 7]), [(0, 0), (1, 1)]);
    }

    #[test]
    fn frame_counts_read_as_a_sentence() {
        let changed = FrameStep::Changed {
            before: 0,
            after: 0,
            changed_pixels: 1,
            regions: Vec::new(),
            strong_pixels: 1,
        };
        assert_eq!(FrameCounts::of(&[]).describe(), "no frame changed");
        assert_eq!(
            FrameCounts::of(std::slice::from_ref(&changed)).describe(),
            "1 frame changed"
        );
        let steps = [
            changed.clone(),
            changed,
            FrameStep::Inserted {
                after: 2,
                frames: 1,
            },
        ];
        assert_eq!(
            FrameCounts::of(&steps).describe(),
            "2 frames changed, 1 added"
        );
    }

    #[test]
    fn a_still_pictures_json_has_no_frame_fields_and_an_animations_does() {
        let (still, _) = decode(&png(&test_frame(0))).expect("decodes");
        let json = serde_json::to_value(&still).expect("serializes");
        assert!(json.get("frames").is_none() && json.get("duration_ms").is_none());
        let (animated, _) = decode(&animation(&[0, 1], 100)).expect("decodes");
        let json = serde_json::to_value(&animated).expect("serializes");
        assert_eq!(json["frames"], 2);
        assert_eq!(json["duration_ms"], 200);
    }

    #[test]
    fn the_engine_verdict_follows_the_comparison() {
        let white = filled(10, 10, [255; 4]);
        let verdict = |after: &RgbaImage| diff_of(&white, after).verdict();
        assert_eq!(verdict(&white), v("invisible"));

        let mut transparent = filled(10, 10, [255; 4]);
        transparent.put_pixel(0, 0, image::Rgba([0, 0, 0, 0]));
        let mut recoloured = transparent.clone();
        recoloured.put_pixel(0, 0, image::Rgba([255, 0, 0, 0]));
        assert_eq!(
            diff_of(&transparent, &recoloured).verdict(),
            v("invisible"),
            "a colour under full transparency is not seen"
        );

        let mut faint = white.clone();
        faint.put_pixel(3, 3, image::Rgba([254, 254, 254, 255]));
        assert_eq!(verdict(&faint), v("imperceptible"));

        // Every pixel a shade darker: past pixelmatch's threshold, nowhere past the strong one.
        let gray = filled(10, 10, [128, 128, 128, 255]);
        let darker = filled(10, 10, [88, 88, 88, 255]);
        assert_eq!(diff_of(&gray, &darker).verdict(), v("artifacts"));

        let mut dotted = white.clone();
        dotted.put_pixel(3, 3, image::Rgba([0, 0, 0, 255]));
        assert_eq!(verdict(&dotted), v("edited"));

        // Most pixels changed: the left half black against the top half black has another
        // layout; its inverse keeps the layout, darker where it was lighter.
        let half = |left: bool, ink: [u8; 4], paper: [u8; 4]| {
            RgbaImage::from_fn(10, 10, |x, y| {
                image::Rgba(if (left && x < 5) || (!left && y < 5) {
                    ink
                } else {
                    paper
                })
            })
        };
        let black = [0, 0, 0, 255];
        let left = half(true, black, [255; 4]);
        assert_eq!(
            diff_of(&left, &half(false, black, [255; 4])).verdict(),
            v("edited"),
            "half the pixels is not more than half"
        );
        let mut top = half(false, black, [255; 4]);
        top.put_pixel(9, 9, image::Rgba(black));
        top.put_pixel(8, 9, image::Rgba(black));
        assert_eq!(diff_of(&left, &top).verdict(), v("replaced"));
        let inverted = half(true, [255; 4], black);
        assert_eq!(diff_of(&left, &inverted).verdict(), v("redrawn"));

        assert_eq!(
            verdict(&filled(5, 10, [255; 4])),
            v("edited+canvas"),
            "another shape is not compared, and called edited"
        );
    }

    #[test]
    fn pictures_are_known_by_extension_and_svg_is_not_one() {
        let is = |name: &str| is_picture_path(std::path::Path::new(name));
        assert!(is("logo.PNG") && is("photo.jpeg") && is("before.png.test"));
        assert!(!is("logo.svg") && !is("main.rs") && !is("Makefile"));
    }
}
