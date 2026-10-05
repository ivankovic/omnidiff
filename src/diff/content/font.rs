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

//! Fonts as containers of glyphs: TrueType and OpenType (and collections of them), and the web's
//! WOFF, WOFF2 and EOT wrappings of the same.
//!
//! A font's members are its **glyphs, keyed by the character they draw** (`U+0041 A`), or by
//! their name for a glyph no character maps to (a ligature, a small capital), and two text
//! members: its **names** (family, style, version, copyright, ...) and its **metrics** (units per
//! em, ascent, descent, ...). A glyph is hashed by its outline and advance width, so a font
//! rebuilt with the same shapes - re-hinted, re-compressed, re-wrapped as WOFF2 - has no changed
//! glyph, and only a changed glyph is drawn: a font of thousands with two edited glyphs draws
//! four pictures.
//!
//! A glyph is drawn the same way on both sides: an em of [`EM_PIXELS`] on a [`CELL_PIXELS`]
//! square, the baseline and the advance width marked in grey, so pictures of the two sides
//! compare pixel by pixel, and a wider glyph shows as a longer baseline. Hinting is not applied:
//! it is how a glyph snaps to a screen's pixels, not what it looks like.
//!
//! A glyph with no outline (a bitmap emoji, a color glyph) is drawn empty; its hash still sees its
//! advance change, but not its picture. Variable fonts are drawn at their default instance.
//!
//! A glyph's verdict is the picture rule over its whole cell, most of which is empty, so even a
//! redesigned glyph rarely changes more than [`crate::diff::picture::REPLACED_SHARE`] of it and
//! reads as edited, never redrawn or replaced. The font fixtures will say whether the share should
//! be of the glyph's own area instead.

use anyhow::{Context, Result, bail};
use image::RgbaImage;
use skrifa::{
    FontRef, GlyphId, MetadataProvider,
    instance::{LocationRef, Size},
    outline::{DrawSettings, OutlinePen},
    raw::{FileRef, TableProvider},
    string::StringId,
};

use super::Format;
use super::container::{Container, ContainerInfo, Member, MemberContent, hash_bytes};
use crate::diff::picture::{Frames, PictureInfo};

/// The side of the square a glyph is drawn on, in pixels.
pub const CELL_PIXELS: u32 = 96;
/// How many pixels an em is drawn at.
pub const EM_PIXELS: f32 = 64.0;
/// Where the origin is drawn: from the left, and the baseline from the top.
const ORIGIN: (f32, f32) = (16.0, 72.0);

/// True if `bytes` start like an sfnt: TrueType, OpenType, Apple's `true`, or a collection.
pub fn is_sfnt(bytes: &[u8]) -> bool {
    let tables = bytes.get(4..6).map(|n| u16::from_be_bytes([n[0], n[1]]));
    match bytes.get(0..4) {
        Some(b"ttcf") => true,
        Some([0, 1, 0, 0] | b"OTTO" | b"true") => tables.is_some_and(|n| (1..=200).contains(&n)),
        _ => false,
    }
}

pub fn is_woff(bytes: &[u8]) -> bool {
    bytes.starts_with(b"wOFF")
}

pub fn is_woff2(bytes: &[u8]) -> bool {
    bytes.starts_with(b"wOF2")
}

/// True if `bytes` are an Embedded OpenType file: its size first, and the magic number `0x504C`.
pub fn is_eot(bytes: &[u8]) -> bool {
    let u32_at = |at: usize| {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    bytes.len() >= 36
        && u32_at(0) == Some(bytes.len() as u32)
        && bytes.get(34..36) == Some(&[0x4C, 0x50])
}

/// The sfnt inside a font file: itself, or unwrapped from WOFF, WOFF2 or EOT.
pub fn sfnt(bytes: &[u8], format: Format) -> Result<Vec<u8>> {
    match format {
        Format::Sfnt => Ok(bytes.to_vec()),
        Format::Woff => wuff::decompress_woff1(bytes)
            .map_err(|err| anyhow::anyhow!("reading the WOFF: {err:?}")),
        Format::Woff2 => wuff::decompress_woff2(bytes)
            .map_err(|err| anyhow::anyhow!("reading the WOFF2: {err:?}")),
        Format::Eot => eot_font_data(bytes),
        other => bail!("{} is not a font", other.name()),
    }
}

/// An EOT's font data: after its header, XOR-obfuscated if its flags say so. MicroType Express
/// compression (flag 0x4) is rare and not read.
fn eot_font_data(bytes: &[u8]) -> Result<Vec<u8>> {
    let u32_at = |at: usize| {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .context("a short EOT header")
    };
    let (size, data_size, flags) = (u32_at(0)? as usize, u32_at(4)? as usize, u32_at(12)?);
    if flags & 0x4 != 0 {
        bail!("an EOT compressed with MicroType Express");
    }
    let start = size
        .checked_sub(data_size)
        .context("an EOT larger than itself")?;
    let mut data = bytes
        .get(start..start + data_size)
        .context("an EOT's font data past its end")?
        .to_vec();
    if flags & 0x1000_0000 != 0 {
        data.iter_mut().for_each(|byte| *byte ^= 0x50);
    }
    Ok(data)
}

/// The format name an sfnt reports as: by its outlines, or a collection.
fn sfnt_name(data: &[u8]) -> &'static str {
    match data.get(0..4) {
        Some(b"ttcf") => "TTC",
        Some(b"OTTO") => "OTF",
        _ => "TTF",
    }
}

/// Where a member's content comes from.
#[derive(Debug, Clone)]
enum Source {
    Glyph { font: u32, glyph: GlyphId },
    Text(String),
}

/// A font, decoded to its glyphs and names.
pub struct Font {
    info: ContainerInfo,
    /// The sfnt, unwrapped.
    data: Vec<u8>,
    members: Vec<Member>,
    sources: Vec<Source>,
}

/// The fonts of an sfnt: one, or each of a collection's.
fn fonts(data: &[u8]) -> Result<Vec<FontRef<'_>>> {
    Ok(match FileRef::new(data).context("reading the font")? {
        FileRef::Font(font) => vec![font],
        FileRef::Collection(collection) => collection
            .iter()
            .collect::<Result<Vec<_>, _>>()
            .context("reading the font collection")?,
    })
}

impl Font {
    pub fn new(bytes: &[u8], format: Format) -> Result<Self> {
        let data = sfnt(bytes, format)?;
        let fonts = fonts(&data)?;
        let mut members = Vec::new();
        let mut sources = Vec::new();
        for (index, font) in fonts.iter().enumerate() {
            // A collection's members carry which font they are in.
            let prefix = if fonts.len() > 1 {
                format!("#{} ", index + 1)
            } else {
                String::new()
            };
            for (key, text) in [("names", names(font)), ("metrics", metrics(font))] {
                members.push(Member {
                    key: format!("{prefix}{key}"),
                    hash: hash_bytes(text.as_bytes()),
                });
                sources.push(Source::Text(text));
            }
            for (key, glyph) in glyph_keys(font) {
                members.push(Member {
                    key: format!("{prefix}{key}"),
                    hash: glyph_hash(font, glyph),
                });
                sources.push(Source::Glyph {
                    font: index as u32,
                    glyph,
                });
            }
        }
        let format_name = match format {
            Format::Sfnt => sfnt_name(&data).to_string(),
            other => other.name(),
        };
        Ok(Self {
            info: ContainerInfo {
                format: format_name,
                bytes: bytes.len(),
                members: members.len(),
            },
            data,
            members,
            sources,
        })
    }
}

impl Container for Font {
    fn info(&self) -> &ContainerInfo {
        &self.info
    }

    fn members(&self) -> &[Member] {
        &self.members
    }

    fn open(&self, index: usize) -> Result<MemberContent> {
        match &self.sources[index] {
            Source::Text(text) => Ok(MemberContent::Text(text.clone())),
            Source::Glyph { font, glyph } => {
                let fonts = fonts(&self.data)?;
                let font = fonts
                    .get(*font as usize)
                    .context("a font of the collection")?;
                let frames = Frames::from_frames(vec![(draw_glyph(font, *glyph)?, 0)])?;
                let info = PictureInfo {
                    format: "GLYPH".to_string(),
                    width: CELL_PIXELS,
                    height: CELL_PIXELS,
                    color: "RGBA8".to_string(),
                    bytes: 0,
                    frames: 1,
                    duration_ms: 0,
                };
                Ok(MemberContent::Picture(Box::new((info, frames))))
            }
        }
    }
}

/// Every glyph worth a member, keyed: each mapped character's (`U+0041 A`), then each named glyph
/// no character maps to (`glyph a.sc`). A glyph that is neither is skipped: nothing would key it
/// the same way in the other version.
fn glyph_keys(font: &FontRef) -> Vec<(String, GlyphId)> {
    let mut keys = Vec::new();
    let mut mapped = std::collections::HashSet::new();
    for (codepoint, glyph) in font.charmap().mappings() {
        mapped.insert(glyph);
        let shown = char::from_u32(codepoint)
            .filter(|c| !c.is_control() && !c.is_whitespace())
            .map(|c| format!(" {c}"))
            .unwrap_or_default();
        keys.push((format!("U+{codepoint:04X}{shown}"), glyph));
    }
    let names = font.glyph_names();
    let count = font.maxp().map(|maxp| maxp.num_glyphs()).unwrap_or(0);
    for id in 0..u32::from(count) {
        let glyph = GlyphId::new(id);
        if mapped.contains(&glyph) {
            continue;
        }
        if let Some(name) = names.get(glyph) {
            keys.push((format!("glyph {}", name.as_str()), glyph));
        }
    }
    keys
}

/// The font's names, one per line: what a reader reads in a font picker or its license.
fn names(font: &FontRef) -> String {
    let ids = [
        ("family", StringId::FAMILY_NAME),
        ("style", StringId::SUBFAMILY_NAME),
        ("full name", StringId::FULL_NAME),
        ("version", StringId::VERSION_STRING),
        ("PostScript name", StringId::POSTSCRIPT_NAME),
        ("copyright", StringId::COPYRIGHT_NOTICE),
        ("trademark", StringId::TRADEMARK),
        ("manufacturer", StringId::MANUFACTURER),
        ("designer", StringId::DESIGNER),
        ("license", StringId::LICENSE_DESCRIPTION),
        ("license URL", StringId::LICENSE_URL),
    ];
    ids.into_iter()
        .filter_map(|(label, id)| {
            let value = font.localized_strings(id).english_or_first()?;
            Some(format!("{label}: {}\n", value.to_string().trim()))
        })
        .collect()
}

/// The font's vertical metrics and size, one per line.
fn metrics(font: &FontRef) -> String {
    let metrics = font.metrics(Size::unscaled(), LocationRef::default());
    let optional = |value: Option<f32>| value.map_or("-".to_string(), |value| value.to_string());
    format!(
        "units per em: {}\nglyphs: {}\nascent: {}\ndescent: {}\nline gap: {}\ncap height: {}\nx height: {}\nitalic angle: {}\n",
        metrics.units_per_em,
        metrics.glyph_count,
        metrics.ascent,
        metrics.descent,
        metrics.leading,
        optional(metrics.cap_height),
        optional(metrics.x_height),
        metrics.italic_angle,
    )
}

/// A pen that records an outline as bytes, to hash.
#[derive(Default)]
struct Recorder(Vec<u8>);

impl Recorder {
    fn push(&mut self, op: u8, points: &[f32]) {
        self.0.push(op);
        for point in points {
            self.0.extend_from_slice(&point.to_le_bytes());
        }
    }
}

impl OutlinePen for Recorder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.push(b'M', &[x, y]);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.push(b'L', &[x, y]);
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.push(b'Q', &[cx0, cy0, x, y]);
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.push(b'C', &[cx0, cy0, cx1, cy1, x, y]);
    }
    fn close(&mut self) {
        self.push(b'Z', &[]);
    }
}

/// A hash of a glyph's outline, in font units, and its advance width.
fn glyph_hash(font: &FontRef, glyph: GlyphId) -> u64 {
    let mut recorder = Recorder::default();
    if let Some(outline) = font.outline_glyphs().get(glyph) {
        let _ = outline.draw(
            DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
            &mut recorder,
        );
    }
    let advance = font
        .glyph_metrics(Size::unscaled(), LocationRef::default())
        .advance_width(glyph)
        .unwrap_or(0.0);
    recorder.push(b'A', &[advance]);
    hash_bytes(&recorder.0)
}

/// A pen that builds a `tiny_skia` path, moved so the glyph's origin sits at [`ORIGIN`] and y
/// grows down.
struct PathPen(resvg::tiny_skia::PathBuilder);

impl PathPen {
    fn at(x: f32, y: f32) -> (f32, f32) {
        (ORIGIN.0 + x, ORIGIN.1 - y)
    }
}

impl OutlinePen for PathPen {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = Self::at(x, y);
        self.0.move_to(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = Self::at(x, y);
        self.0.line_to(x, y);
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let ((cx0, cy0), (x, y)) = (Self::at(cx0, cy0), Self::at(x, y));
        self.0.quad_to(cx0, cy0, x, y);
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let ((cx0, cy0), (cx1, cy1), (x, y)) =
            (Self::at(cx0, cy0), Self::at(cx1, cy1), Self::at(x, y));
        self.0.cubic_to(cx0, cy0, cx1, cy1, x, y);
    }
    fn close(&mut self) {
        self.0.close();
    }
}

/// Draws `glyph` black on a transparent [`CELL_PIXELS`] square, over a grey baseline as long as
/// its advance width.
pub fn draw_glyph(font: &FontRef, glyph: GlyphId) -> Result<RgbaImage> {
    use resvg::tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Transform};
    let mut pixmap = Pixmap::new(CELL_PIXELS, CELL_PIXELS).context("a glyph's canvas")?;
    let size = Size::new(EM_PIXELS);
    let advance = font
        .glyph_metrics(size, LocationRef::default())
        .advance_width(glyph)
        .unwrap_or(0.0);
    let mut grey = Paint::default();
    grey.set_color(Color::from_rgba8(160, 160, 160, 255));
    if let Some(baseline) = Rect::from_xywh(ORIGIN.0, ORIGIN.1, advance.max(1.0), 1.0) {
        pixmap.fill_rect(baseline, &grey, Transform::identity(), None);
    }
    if let Some(outline) = font.outline_glyphs().get(glyph) {
        let mut pen = PathPen(PathBuilder::new());
        outline
            .draw(
                DrawSettings::unhinted(size, LocationRef::default()),
                &mut pen,
            )
            .map_err(|err| anyhow::anyhow!("drawing a glyph: {err:?}"))?;
        if let Some(path) = pen.0.finish() {
            let mut black = Paint::default();
            black.set_color(Color::BLACK);
            black.anti_alias = true;
            pixmap.fill_path(
                &path,
                &black,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let pixel = pixel.demultiply();
            [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()]
        })
        .collect();
    RgbaImage::from_raw(CELL_PIXELS, CELL_PIXELS, rgba).context("a glyph's pixels")
}

/// The glyph count of each font in `bytes`, for a sampler's probe without hashing every outline.
pub fn glyph_count(bytes: &[u8], format: Format) -> Result<u64> {
    let data = sfnt(bytes, format)?;
    Ok(fonts(&data)?
        .iter()
        .map(|font| u64::from(font.maxp().map(|maxp| maxp.num_glyphs()).unwrap_or(0)))
        .sum())
}

/// The keys of every member, sorted, for a sampler's probe: a font that gained or lost a glyph
/// has another shape.
pub fn member_keys(bytes: &[u8], format: Format) -> Result<Vec<String>> {
    let data = sfnt(bytes, format)?;
    let mut keys: Vec<String> = fonts(&data)?
        .iter()
        .flat_map(|font| glyph_keys(font).into_iter().map(|(key, _)| key))
        .collect();
    keys.sort_unstable();
    Ok(keys)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::content::ContentDiff;
    use crate::diff::content::container::{MemberDetail, MemberStatus};

    /// A font of `make_fonts.py`'s.
    fn read(name: &str) -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/test/data/content")
            .join(name);
        std::fs::read(&path).unwrap_or_else(|err| panic!("{path:?}: {err}"))
    }

    fn keys(font: &Font) -> Vec<&str> {
        font.members()
            .iter()
            .map(|member| member.key.as_str())
            .collect()
    }

    #[test]
    fn every_wrapping_sniffs_as_a_font() {
        use crate::diff::content::sniff;
        assert_eq!(sniff(&read("before.ttf")), Some(Format::Sfnt));
        assert_eq!(sniff(&read("after.woff")), Some(Format::Woff));
        assert_eq!(sniff(&read("after.woff2")), Some(Format::Woff2));
    }

    #[test]
    fn a_font_is_its_names_metrics_and_glyphs() -> Result<()> {
        let font = Font::new(&read("before.ttf"), Format::Sfnt)?;
        assert_eq!(font.info().format, "TTF");
        assert_eq!(
            keys(&font),
            vec!["names", "metrics", "U+0041 A", "U+0042 B", "glyph .notdef"],
            "the missing-glyph box too: it shows wherever a character has no glyph"
        );
        let MemberContent::Text(names) = font.open(0)? else {
            panic!("names are text");
        };
        assert!(names.contains("family: Diff Test\n"), "{names}");
        let MemberContent::Picture(glyph) = font.open(2)? else {
            panic!("a glyph is a picture");
        };
        let pixels = glyph.1.pixels(0);
        // The square's middle is black, the cell's corner empty.
        assert_eq!(pixels.get_pixel(40, 50).0, [0, 0, 0, 255]);
        assert_eq!(pixels.get_pixel(2, 2).0[3], 0);
        Ok(())
    }

    #[test]
    fn woff_and_woff2_unwrap_to_the_same_glyphs() -> Result<()> {
        let ttf = Font::new(&read("after.ttf"), Format::Sfnt)?;
        for (name, format) in [("after.woff", Format::Woff), ("after.woff2", Format::Woff2)] {
            let wrapped = Font::new(&read(name), format)?;
            assert_eq!(wrapped.members(), ttf.members(), "{name}");
        }
        Ok(())
    }

    #[test]
    fn a_changed_glyph_is_drawn_and_an_unchanged_one_is_not() -> Result<()> {
        let Some(ContentDiff::Container(diff)) =
            crate::diff::content::diff(&read("before.ttf"), &read("after.woff2"))?
        else {
            panic!("a font pair");
        };
        assert_eq!(diff.unchanged, 2, "A and .notdef: {diff:?}");
        assert_eq!(
            diff.member("metrics").map(|member| member.status),
            Some(MemberStatus::Changed),
            "the glyph count"
        );
        let status = |key: &str| diff.member(key).map(|member| member.status);
        assert_eq!(status("names"), Some(MemberStatus::Changed), "the version");
        assert_eq!(status("U+0042 B"), Some(MemberStatus::Changed));
        assert_eq!(status("U+0043 C"), Some(MemberStatus::Added));
        let b = diff.member("U+0042 B").unwrap();
        assert!(matches!(b.detail, Some(MemberDetail::Picture(_))), "{b:?}");
        assert_eq!(
            b.verdict().map(|verdict| verdict.to_string()).as_deref(),
            Some("edited")
        );
        Ok(())
    }

    #[test]
    fn an_eot_holds_its_font_obfuscated_or_not() -> Result<()> {
        let ttf = read("before.ttf");
        let eot = |flags: u32| {
            let mut header = vec![0u8; 82];
            let size = (header.len() + ttf.len()) as u32;
            header[0..4].copy_from_slice(&size.to_le_bytes());
            header[4..8].copy_from_slice(&(ttf.len() as u32).to_le_bytes());
            header[8..12].copy_from_slice(&0x0002_0001u32.to_le_bytes());
            header[12..16].copy_from_slice(&flags.to_le_bytes());
            header[34..36].copy_from_slice(&[0x4C, 0x50]);
            let xor = if flags & 0x1000_0000 != 0 { 0x50 } else { 0 };
            header.extend(ttf.iter().map(|byte| byte ^ xor));
            header
        };
        for flags in [0, 0x1000_0000] {
            let bytes = eot(flags);
            assert!(is_eot(&bytes));
            assert_eq!(sfnt(&bytes, Format::Eot)?, ttf);
        }
        assert!(sfnt(&eot(0x4), Format::Eot).is_err(), "MicroType Express");
        Ok(())
    }
}
