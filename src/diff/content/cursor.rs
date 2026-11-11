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

//! Mouse cursors, in the four formats cursor themes ship in. Every cursor is a picture in the
//! end, often animated, so each format decodes to the picture engine's frames:
//!
//! * **Windows `.cur`** is an ICO with a hotspot: one picture, the largest of its sizes.
//! * **Windows `.ani`** is a RIFF file of `.cur` or `.ico` frames, with their order (`seq `) and
//!   times (`rate`, in sixtieths of a second): one animation.
//! * **X cursors** (`Xcur`, extensionless files named after the cursor: `wait`, `crosshair`) hold
//!   one animation per nominal size, so a file is a container of sizes (`24px`, `32px`).
//! * **Hyprland's `.hlc`** is a zip of SVG or PNG pictures and a `meta.hl` naming them: a
//!   container of those files, the SVGs drawn at the size `meta.hl` defines.
//!
//! The hotspot (the pixel that clicks) is not compared: it is metadata, and a changed one shows
//! nowhere a reader could see.
//!
//! **What a file may decode to is bounded**: its pictures together [`MAX_CURSOR_BYTES`] (an X
//! cursor's table may point every entry at one image, an `.ani` show one frame many times), an
//! `.ani` [`MAX_ANI_STEPS`] steps, each frame deflated once however many steps show it, and an
//! SVG is drawn at most [`MAX_SVG_SIZE`] pixels square and never reads a file it names.

use std::collections::BTreeMap;
use std::io::Cursor;

use anyhow::{Context, Result, bail};
use image::RgbaImage;

use super::archive::Zip;
use super::container::{Container, ContainerInfo, Member, MemberContent, hash_bytes};
use crate::diff::picture::{Frames, PictureInfo};

/// True if `bytes` start like a Windows cursor: an ICO directory of type 2.
pub fn is_cur(bytes: &[u8]) -> bool {
    matches!(bytes, [0, 0, 2, 0, count_low, count_high, ..] if (*count_low, *count_high) != (0, 0))
}

/// True if `bytes` are a Windows animated cursor: a RIFF file of form `ACON`.
pub fn is_ani(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"ACON"
}

/// True if `bytes` are an X cursor file.
pub fn is_xcursor(bytes: &[u8]) -> bool {
    bytes.starts_with(b"Xcur")
}

/// The member names that make a zip a Hyprland cursor rather than an archive.
const HYPRCURSOR_META: [&str; 2] = ["meta.hl", "meta.toml"];

/// True if the zip in `bytes` is a Hyprland cursor: it has a `meta.hl` (or `meta.toml`).
pub fn is_hyprcursor(bytes: &[u8]) -> bool {
    zip::ZipArchive::new(Cursor::new(bytes)).is_ok_and(|archive| {
        archive
            .file_names()
            .any(|name| name.is_ok_and(|name| HYPRCURSOR_META.contains(&name.as_ref())))
    })
}

/// The most RGBA bytes one cursor file's pictures may decode to: a 256-pixel cursor of a thousand
/// frames, past any theme's.
pub const MAX_CURSOR_BYTES: u64 = 256 * 1024 * 1024;

/// The most steps an `.ani` may play: a step is four bytes of `seq `, a whole frame to compare.
/// Real cursors play a few dozen.
pub const MAX_ANI_STEPS: usize = 10_000;

/// The largest square a Hyprland SVG is drawn on, whatever `define_size` says: cursors are drawn
/// at 256 pixels at most.
pub const MAX_SVG_SIZE: u32 = 1024;

/// Adds a `width` x `height` picture's RGBA bytes to `total`, unless that passes
/// [`MAX_CURSOR_BYTES`].
fn count_decoded(total: &mut u64, width: u32, height: u32) -> Result<()> {
    *total += u64::from(width) * u64::from(height) * 4;
    if *total > MAX_CURSOR_BYTES {
        bail!("a cursor of more than {MAX_CURSOR_BYTES} bytes of pictures");
    }
    Ok(())
}

/// A picture's description, for a cursor decoded here.
fn info(format: &str, bytes: usize, frames: &Frames) -> PictureInfo {
    let (width, height) = frames.dimensions();
    PictureInfo {
        format: format.to_string(),
        width,
        height,
        color: "RGBA8".to_string(),
        bytes,
        frames: frames.count(),
        duration_ms: if frames.count() > 1 {
            frames.total_ms()
        } else {
            0
        },
    }
}

/// The largest picture of an ICO or CUR, as RGBA: a CUR is read as the ICO it is but for its type.
fn decode_icon(bytes: &[u8]) -> Result<RgbaImage> {
    let mut icon = bytes.to_vec();
    if is_cur(&icon) {
        icon[2] = 1;
    }
    Ok(
        image::load_from_memory_with_format(&icon, image::ImageFormat::Ico)
            .context("decoding the cursor's picture")?
            .to_rgba8(),
    )
}

/// A Windows `.cur`, as a still picture.
pub fn decode_cur(bytes: &[u8]) -> Result<(PictureInfo, Frames)> {
    let frames = Frames::from_frames(vec![(decode_icon(bytes)?, 0)])?;
    Ok((info("CUR", bytes.len(), &frames), frames))
}

/// A sixtieth of a second, in which `.ani` times its frames, in milliseconds.
const JIFFY_MS: f64 = 1000.0 / 60.0;

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// The chunks of a RIFF body: (id, data), each padded to an even length.
fn riff_chunks(mut body: &[u8]) -> Vec<(&[u8], &[u8])> {
    let mut chunks = Vec::new();
    while body.len() >= 8 {
        let id = &body[0..4];
        let size = u32_at(body, 4).unwrap_or(0) as usize;
        let data = &body[8..(8 + size).min(body.len())];
        chunks.push((id, data));
        body = &body[(8 + size + size % 2).min(body.len())..];
    }
    chunks
}

/// A Windows `.ani`, as an animation: its frames in `seq ` order (stored order without one), each
/// shown for its `rate` (or the header's rate).
pub fn decode_ani(bytes: &[u8]) -> Result<(PictureInfo, Frames)> {
    if !is_ani(bytes) {
        bail!("not an animated cursor");
    }
    let (mut header, mut rates, mut sequence, mut icons) = (None, None, None, Vec::new());
    for (id, data) in riff_chunks(&bytes[12..]) {
        match id {
            b"anih" => header = Some(data),
            b"rate" => rates = Some(data),
            b"seq " => sequence = Some(data),
            b"LIST" if data.starts_with(b"fram") => {
                icons.extend(
                    riff_chunks(&data[4..])
                        .into_iter()
                        .filter(|(id, _)| *id == b"icon")
                        .map(|(_, data)| data),
                );
            }
            _ => {}
        }
    }
    let header = header.context("an animated cursor without its header")?;
    let steps = u32_at(header, 8).context("a short animated cursor header")? as usize;
    let default_rate = u32_at(header, 28).unwrap_or(0);
    let flags = u32_at(header, 32).unwrap_or(0);
    if flags & 1 == 0 {
        bail!("an animated cursor of raw bitmaps, not icons");
    }
    let mut decoded = 0;
    let mut pictures = Vec::with_capacity(icons.len());
    for icon in &icons {
        let picture = decode_icon(icon)?;
        count_decoded(&mut decoded, picture.width(), picture.height())?;
        pictures.push(picture);
    }
    let first = pictures
        .first()
        .context("an animated cursor with no frame")?;
    let (width, height) = first.dimensions();
    // A frame of another size is fitted to the first, as Windows draws it.
    let mut fitted = 0;
    let pictures = pictures
        .into_iter()
        .map(|picture| {
            count_decoded(&mut fitted, width, height)?;
            Ok(if picture.dimensions() == (width, height) {
                picture
            } else {
                image::imageops::resize(
                    &picture,
                    width,
                    height,
                    image::imageops::FilterType::Nearest,
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let steps = if steps == 0 { pictures.len() } else { steps };
    // A step past both the sequence and the frames shows a frame that is not there.
    let listed = sequence.map_or(0, |sequence| sequence.len() / 4);
    if steps > MAX_ANI_STEPS || steps > listed.max(pictures.len()) {
        bail!("an animated cursor of {steps} steps");
    }
    let mut frames = Vec::with_capacity(steps);
    for step in 0..steps {
        let index = sequence
            .and_then(|sequence| u32_at(sequence, step * 4))
            .map_or(step, |index| index as usize);
        if index >= pictures.len() {
            bail!("step {step} shows frame {index}, which is not there");
        }
        let jiffies = rates
            .and_then(|rates| u32_at(rates, step * 4))
            .unwrap_or(default_rate);
        frames.push((index, (f64::from(jiffies) * JIFFY_MS).round() as u32));
    }
    let frames = Frames::from_steps(&pictures, &frames)?;
    Ok((info("ANI", bytes.len(), &frames), frames))
}

/// The image chunks of an X cursor, grouped by nominal size: each group the frames of one
/// animation, with their delays.
fn xcursor_sizes(bytes: &[u8]) -> Result<BTreeMap<u32, Vec<(RgbaImage, u32)>>> {
    const IMAGE: u32 = 0xfffd_0002;
    let toc = u32_at(bytes, 12).context("a short X cursor header")? as usize;
    let mut sizes: BTreeMap<u32, Vec<(RgbaImage, u32)>> = BTreeMap::new();
    let mut decoded = 0;
    for entry in 0..toc.min(bytes.len() / 12) {
        let at = 16 + entry * 12;
        let (Some(kind), Some(position)) = (u32_at(bytes, at), u32_at(bytes, at + 8)) else {
            bail!("a short X cursor table of contents");
        };
        if kind != IMAGE {
            continue;
        }
        let chunk = bytes
            .get(position as usize..)
            .context("an X cursor image past the end")?;
        let field = |index: usize| u32_at(chunk, index * 4).context("a short X cursor image");
        let (size, width, height, delay) = (field(2)?, field(4)?, field(5)?, field(8)?);
        if width > 0x7fff || height > 0x7fff {
            bail!("an X cursor image of {width}x{height}");
        }
        count_decoded(&mut decoded, width, height)?;
        let header = field(0)? as usize;
        let pixels = (width * height) as usize;
        let argb = chunk
            .get(header..header + pixels * 4)
            .context("an X cursor image's pixels past the end")?;
        // Premultiplied ARGB, little-endian, to straight RGBA.
        let rgba = argb
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|&[b, g, r, a]| {
                let straight = |c: u8| match a {
                    0 => 0,
                    a => ((u32::from(c) * 255 + u32::from(a) / 2) / u32::from(a)).min(255) as u8,
                };
                [straight(r), straight(g), straight(b), a]
            })
            .collect();
        let picture = RgbaImage::from_raw(width, height, rgba).context("an X cursor image")?;
        sizes.entry(size).or_default().push((picture, delay));
    }
    if sizes.is_empty() {
        bail!("an X cursor with no image");
    }
    Ok(sizes)
}

/// An X cursor: one member per nominal size (`24px`), each an animation (or a still).
pub struct Xcursor {
    info: ContainerInfo,
    members: Vec<Member>,
    pictures: Vec<(PictureInfo, Frames)>,
}

impl Xcursor {
    pub fn new(bytes: &[u8]) -> Result<Self> {
        let (mut members, mut pictures) = (Vec::new(), Vec::new());
        for (size, frames) in xcursor_sizes(bytes)? {
            let mut hashed = Vec::new();
            for (picture, delay) in &frames {
                hashed.extend_from_slice(picture.as_raw());
                hashed.extend_from_slice(&delay.to_le_bytes());
            }
            members.push(Member {
                key: format!("{size}px"),
                hash: hash_bytes(&hashed),
            });
            let frames = Frames::from_frames(frames)?;
            pictures.push((info("XCURSOR", bytes.len(), &frames), frames));
        }
        Ok(Self {
            info: ContainerInfo {
                format: "XCURSOR".to_string(),
                bytes: bytes.len(),
                members: members.len(),
            },
            members,
            pictures,
        })
    }
}

impl Container for Xcursor {
    fn info(&self) -> &ContainerInfo {
        &self.info
    }

    fn members(&self) -> &[Member] {
        &self.members
    }

    fn open(&self, index: usize) -> Result<MemberContent> {
        Ok(MemberContent::Picture(Box::new(
            self.pictures[index].clone(),
        )))
    }
}

/// The size an SVG member is drawn at when `meta.hl` gives none (`define_size = 0`, any size).
const SVG_SIZE: u32 = 64;

/// A Hyprland cursor: the zip's files as members, its pictures drawn.
pub struct Hyprcursor {
    zip: Zip,
    info: ContainerInfo,
    /// The size `meta.hl` draws each picture at, by file name.
    sizes: BTreeMap<String, u32>,
}

impl Hyprcursor {
    pub fn new(bytes: &[u8]) -> Result<Self> {
        let zip = Zip::new(bytes)?;
        let mut sizes = BTreeMap::new();
        if let Some(meta) = HYPRCURSOR_META.iter().find_map(|name| zip.find(name)) {
            let meta = String::from_utf8_lossy(&zip.read(meta)?).into_owned();
            // `define_size = 32, wait_32.png` or `define_size = 0, wait.svg, 500`.
            for line in meta.lines() {
                let Some(definition) = line.trim().strip_prefix("define_size") else {
                    continue;
                };
                let mut fields = definition.trim_start_matches([' ', '=']).split(',');
                let size = fields.next().and_then(|size| size.trim().parse().ok());
                if let (Some(size), Some(file)) = (size, fields.next()) {
                    sizes.insert(file.trim().to_string(), size);
                }
            }
        }
        let info = ContainerInfo {
            format: "HYPRCURSOR".to_string(),
            ..zip.info().clone()
        };
        Ok(Self { zip, info, sizes })
    }
}

impl Container for Hyprcursor {
    fn info(&self) -> &ContainerInfo {
        &self.info
    }

    fn members(&self) -> &[Member] {
        self.zip.members()
    }

    fn open(&self, index: usize) -> Result<MemberContent> {
        let key = &self.zip.members()[index].key;
        let bytes = self.zip.read(index)?;
        if key.to_ascii_lowercase().ends_with(".svg") {
            let size = match self.sizes.get(key) {
                Some(&size) if size > 0 => size.min(MAX_SVG_SIZE),
                _ => SVG_SIZE,
            };
            let frames = Frames::from_frames(vec![(render_svg(&bytes, size)?, 0)])?;
            return Ok(MemberContent::Picture(Box::new((
                info("SVG", bytes.len(), &frames),
                frames,
            ))));
        }
        Ok(MemberContent::Bytes(bytes))
    }
}

/// `svg` drawn to fit a `size` x `size` square. An image it places is read only from a `data:`
/// URL: usvg's default reads any path an `href` names, `/dev/zero` or a FIFO included.
pub fn render_svg(svg: &[u8], size: u32) -> Result<RgbaImage> {
    use resvg::{tiny_skia, usvg};
    let options = usvg::Options {
        resources_dir: None,
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_data(svg, &options).context("reading the SVG")?;
    let (width, height) = (tree.size().width(), tree.size().height());
    let size = size.min(MAX_SVG_SIZE);
    let scale = size as f32 / width.max(height).max(1.0);
    let (w, h) = (
        ((width * scale).round() as u32).max(1),
        ((height * scale).round() as u32).max(1),
    );
    let mut pixmap = tiny_skia::Pixmap::new(w, h).context("an SVG of no size")?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let pixel = pixel.demultiply();
            [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()]
        })
        .collect();
    RgbaImage::from_raw(w, h, rgba).context("an SVG's pixels")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// An ICO of one `size` x `size` picture of `color`, as a CUR if `cursor`.
    pub(crate) fn icon(size: u32, color: [u8; 4], cursor: bool) -> Vec<u8> {
        let mut bytes = Vec::new();
        image::RgbaImage::from_pixel(size, size, image::Rgba(color))
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Ico)
            .expect("encodes");
        if cursor {
            bytes[2] = 2;
        }
        bytes
    }

    fn chunk(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut bytes = id.to_vec();
        bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(data);
        if data.len() % 2 == 1 {
            bytes.push(0);
        }
        bytes
    }

    /// An `.ani` of `frames` (icons), shown in `sequence` order for `rates` jiffies each.
    pub(crate) fn ani(frames: &[Vec<u8>], sequence: &[u32], rates: &[u32]) -> Vec<u8> {
        let mut header = Vec::new();
        for field in [
            36,
            frames.len() as u32,
            sequence.len() as u32,
            0,
            0,
            0,
            0,
            6,
            3,
        ] {
            header.extend_from_slice(&field.to_le_bytes());
        }
        let words = |values: &[u32]| {
            values
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<u8>>()
        };
        let mut fram = b"fram".to_vec();
        for frame in frames {
            fram.extend(chunk(b"icon", frame));
        }
        let mut body = b"ACON".to_vec();
        body.extend(chunk(b"anih", &header));
        body.extend(chunk(b"rate", &words(rates)));
        body.extend(chunk(b"seq ", &words(sequence)));
        body.extend(chunk(b"LIST", &fram));
        chunk(b"RIFF", &body)
    }

    /// An X cursor of `images` (nominal size, side, ARGB color, delay).
    pub(crate) fn xcursor(images: &[(u32, u32, u32, u32)]) -> Vec<u8> {
        let mut bytes = b"Xcur".to_vec();
        for field in [16u32, 0x1_0000, images.len() as u32] {
            bytes.extend_from_slice(&field.to_le_bytes());
        }
        let mut position = 16 + 12 * images.len() as u32;
        let mut chunks = Vec::new();
        for &(size, side, argb, delay) in images {
            for field in [0xfffd_0002u32, size, position] {
                bytes.extend_from_slice(&field.to_le_bytes());
            }
            let mut chunk = Vec::new();
            for field in [36, 0xfffd_0002, size, 1, side, side, 0, 0, delay] {
                chunk.extend_from_slice(&u32::to_le_bytes(field));
            }
            for _ in 0..side * side {
                chunk.extend_from_slice(&argb.to_le_bytes());
            }
            position += chunk.len() as u32;
            chunks.extend(chunk);
        }
        bytes.extend(chunks);
        bytes
    }

    #[test]
    fn a_cur_decodes_as_the_icon_it_is() -> Result<()> {
        let cur = icon(16, [255, 0, 0, 255], true);
        assert!(is_cur(&cur));
        let (info, frames) = decode_cur(&cur)?;
        assert_eq!((info.format.as_str(), info.width), ("CUR", 16));
        assert_eq!(frames.pixels(0).get_pixel(3, 3).0, [255, 0, 0, 255]);
        Ok(())
    }

    #[test]
    fn an_ani_plays_its_frames_in_sequence_at_their_rates() -> Result<()> {
        let (red, blue) = (
            icon(8, [255, 0, 0, 255], true),
            icon(8, [0, 0, 255, 255], false),
        );
        let bytes = ani(&[red, blue], &[1, 0, 1], &[6, 12, 3]);
        assert!(is_ani(&bytes));
        let (info, frames) = decode_ani(&bytes)?;
        assert_eq!(info.frames, 3);
        assert_eq!(frames.pixels(0).get_pixel(0, 0).0, [0, 0, 255, 255]);
        assert_eq!(frames.pixels(1).get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(
            (frames.delay_ms(0), frames.delay_ms(1), frames.delay_ms(2)),
            (100, 200, 50)
        );
        Ok(())
    }

    #[test]
    fn an_x_cursor_is_one_member_per_size() -> Result<()> {
        // Half-transparent white, premultiplied: 0x80808080.
        let bytes = xcursor(&[
            (24, 2, 0x8080_8080, 50),
            (24, 2, 0xff00_00ff, 50),
            (32, 3, 0xffff_0000, 0),
        ]);
        assert!(is_xcursor(&bytes));
        let cursor = Xcursor::new(&bytes)?;
        let keys: Vec<&str> = cursor.members().iter().map(|m| m.key.as_str()).collect();
        assert_eq!(keys, vec!["24px", "32px"]);
        let MemberContent::Picture(picture) = cursor.open(0)? else {
            panic!("a picture");
        };
        let (info, frames) = *picture;
        assert_eq!((info.frames, info.duration_ms), (2, 100));
        assert_eq!(frames.pixels(0).get_pixel(0, 0).0, [255, 255, 255, 128]);
        assert_eq!(frames.pixels(1).get_pixel(0, 0).0, [0, 0, 255, 255]);
        Ok(())
    }

    #[test]
    fn a_hyprcursor_draws_its_svg_at_the_defined_size() -> Result<()> {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="red"/></svg>"#;
        let bytes = crate::diff::content::archive::tests::zip(&[
            ("meta.hl", b"define_size = 32, arrow.svg\nhotspot_x = 0.5\n"),
            ("arrow.svg", svg),
        ]);
        assert!(is_hyprcursor(&bytes));
        let cursor = Hyprcursor::new(&bytes)?;
        let MemberContent::Picture(picture) = cursor.open(1)? else {
            panic!("the SVG is drawn");
        };
        let (info, frames) = *picture;
        assert_eq!((info.width, info.height), (32, 32));
        assert_eq!(frames.pixels(0).get_pixel(16, 16).0, [255, 0, 0, 255]);
        assert!(
            matches!(cursor.open(0)?, MemberContent::Bytes(_)),
            "meta.hl is text"
        );
        Ok(())
    }

    #[test]
    fn an_ani_shares_its_frames_and_bounds_its_steps() -> Result<()> {
        let red = icon(8, [255, 0, 0, 255], true);
        // Five thousand steps of one frame: one picture, deflated once.
        let bytes = ani(std::slice::from_ref(&red), &[0; 5000], &[1; 5000]);
        let (info, frames) = decode_ani(&bytes)?;
        assert_eq!(
            (info.frames, frames.pixels(4999).get_pixel(0, 0).0),
            (5000, [255, 0, 0, 255])
        );
        let too_many = ani(std::slice::from_ref(&red), &vec![0; MAX_ANI_STEPS + 1], &[]);
        assert!(decode_ani(&too_many).is_err());
        // A header claiming four billion steps, with no sequence to play them: refused before
        // anything is allocated for them.
        let mut claims = ani(&[red], &[], &[]);
        let at = claims
            .windows(4)
            .position(|window| window == b"anih")
            .expect("a header")
            + 8;
        claims[at + 8..at + 12].copy_from_slice(&u32::MAX.to_le_bytes());
        let error = decode_ani(&claims).expect_err("too many steps");
        assert!(error.to_string().contains("steps"), "{error}");
        Ok(())
    }

    #[test]
    fn an_x_cursor_pointing_every_entry_at_one_image_is_bounded() {
        // 512 x 512 is a MiB of pixels; 300 entries for it pass MAX_CURSOR_BYTES.
        let mut bytes = xcursor(&[(512, 512, 0xff00_00ff, 0)]);
        let entries = 300u32;
        let image_at = 16 + 12 * entries;
        let image = bytes[28..].to_vec();
        bytes.truncate(12);
        bytes.extend_from_slice(&entries.to_le_bytes());
        for _ in 0..entries {
            for field in [0xfffd_0002u32, 512, image_at] {
                bytes.extend_from_slice(&field.to_le_bytes());
            }
        }
        bytes.extend(image);
        let error = Xcursor::new(&bytes).err().expect("too many pictures");
        assert!(error.to_string().contains("bytes of pictures"), "{error}");
    }

    #[test]
    fn a_hyprcursor_svg_is_drawn_at_most_at_the_largest_size() -> Result<()> {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="red"/></svg>"#;
        let bytes = crate::diff::content::archive::tests::zip(&[
            ("meta.hl", b"define_size = 4294967295, arrow.svg\n"),
            ("arrow.svg", svg),
        ]);
        let MemberContent::Picture(picture) = Hyprcursor::new(&bytes)?.open(1)? else {
            panic!("the SVG is drawn");
        };
        assert_eq!(
            (picture.0.width, picture.0.height),
            (MAX_SVG_SIZE, MAX_SVG_SIZE)
        );
        Ok(())
    }

    #[test]
    fn an_svg_never_reads_a_file_it_names() -> Result<()> {
        use resvg::usvg;
        let dir = tempfile::tempdir()?;
        let red = dir.path().join("red.svg");
        std::fs::write(
            &red,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="red"/></svg>"#,
        )?;
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="10" height="10"><image width="10" height="10" xlink:href="{}"/></svg>"#,
            red.display()
        );
        let by_default = usvg::Tree::from_data(svg.as_bytes(), &usvg::Options::default())?;
        assert!(
            by_default.root().has_children(),
            "usvg's default reads the file"
        );
        let drawn = render_svg(svg.as_bytes(), 10)?;
        assert!(
            drawn.pixels().all(|pixel| pixel.0[3] == 0),
            "nothing drawn from the file"
        );
        Ok(())
    }
}
