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

//! Content-aware diffing of the files that are not text: which kind of content a binary file
//! holds, and the diff that kind gets.
//!
//! The change census (`research/data/corpus_stats/change_census.csv`) puts 9.1% of the files
//! commits change in binary formats. Text, code or not, is diffed as text; of the rest, pictures
//! are the most widespread, and fonts, cursor themes, archives, PDFs and compiled message catalogs
//! follow. Each [`Family`] here is one of those, and together they are what takes the share of
//! changed files a reader sees as content, not as "Binary files differ", towards 99%.
//!
//! **Kinds are recognised by their bytes** ([`sniff`]), never by the file name: under `git diff`
//! both sides are git's temporary files, and an extensionless file (an X cursor named `wait`) has
//! no name to go by.
//!
//! **A pair is diffed as one kind or not at all** ([`pair_kind`]): every side present must be
//! content of the same engine. An empty side is git's `/dev/null` for an added or deleted file.
//! A pair that does not decode is reported as any other binary pair, never as an error: under
//! `GIT_EXTERNAL_DIFF` an error abandons every file after it.
//!
//! **Two engines.** A picture is pixels ([`picture`]). Everything else is a [`container`] of
//! named members - an archive's files, a font's glyphs, a PDF's pages - matched by name, each
//! changed member diffed by what it holds.

pub mod archive;
pub mod container;

use anyhow::{Result, bail};
use serde::Serialize;

use super::picture::{self, PictureDiff};
use container::{Container, ContainerDiff};

/// What happened to a picture, or to any other content: the question a content fixture's human
/// verdict answers, and the engine's answer to it ([`PictureDiff::verdict`]).
///
/// A pair can fit more than one, and a fixture records exactly one, so the first that fits wins:
/// replaced, then content change, then resized, then frame rate change, then no visible change. A
/// different picture at a new size is replaced; a picture both rescaled and edited is a content
/// change; an animation whose frames changed as well as their timing is a content change. The engine does
/// not follow this order yet: it compares no pixels across a size change, so it calls every such
/// pair resized, and the fixtures that disagree record it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// The same content, edited: something in it changed, half of it or less.
    ContentChange,
    /// Nothing a reader would see changed: re-encoded, re-compressed, or only metadata.
    NoVisibleChange,
    /// Scaled or re-cropped to a different size, and nothing else.
    Resized,
    /// Different content altogether, or the same with more than half of it edited
    /// ([`picture::REPLACED_SHARE`]).
    Replaced,
    /// An animation showing the same frames for different times.
    FrameRateChange,
}

impl Verdict {
    /// In the order `human_solver`'s number keys pick them; new verdicts go at the end, so the
    /// keys do not move under a hand that knows them.
    pub const ALL: [Verdict; 5] = [
        Verdict::ContentChange,
        Verdict::NoVisibleChange,
        Verdict::Resized,
        Verdict::Replaced,
        Verdict::FrameRateChange,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Verdict::ContentChange => "content change",
            Verdict::NoVisibleChange => "no visible change",
            Verdict::Resized => "resized",
            Verdict::Replaced => "replaced",
            Verdict::FrameRateChange => "frame rate change",
        }
    }
}

/// The groups content is sampled, annotated and reported in: one fixture dataset each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Family {
    Pictures,
    Archives,
}

impl Serialize for Family {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

impl Family {
    pub const ALL: [Family; 2] = [Family::Pictures, Family::Archives];

    /// The family's fixture dataset, `src/test/data/<name>/`, and its `sample.csv` tag.
    pub fn name(self) -> &'static str {
        match self {
            Family::Pictures => "pictures",
            Family::Archives => "archives",
        }
    }

    /// What one file of the family is called in a report (`"Picture"`), or two (`"Pictures"`).
    pub fn noun(self, plural: bool) -> &'static str {
        match (self, plural) {
            (Family::Pictures, false) => "Picture",
            (Family::Pictures, true) => "Pictures",
            (Family::Archives, false) => "Archive",
            (Family::Archives, true) => "Archives",
        }
    }

    pub fn from_name(name: &str) -> Option<Family> {
        Family::ALL.into_iter().find(|family| family.name() == name)
    }

    /// The extensions files of this family usually carry, lower-cased: where the sampler looks.
    /// Recognising a file never depends on them ([`sniff`]).
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Family::Pictures => picture::PICTURE_EXTENSIONS,
            // Office documents, EPUBs and Python wheels are zips too.
            Family::Archives => &[
                "zip", "jar", "war", "ear", "aar", "apk", "whl", "nupkg", "vsix", "xpi", "epub",
                "docx", "xlsx", "pptx", "odt", "ods", "odp", "odg", "tar", "gz", "tgz", "xz",
                "txz", "bz2", "tbz2",
            ],
        }
    }
}

/// One content format, as [`sniff`] recognises it from the bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// A raster picture `image` decodes: PNG, JPEG, GIF, WebP, BMP, ICO or TIFF.
    Picture(image::ImageFormat),
    Zip,
    Tar,
    Gzip,
    Xz,
    Bzip2,
}

impl Format {
    pub fn family(self) -> Family {
        match self {
            Format::Picture(_) => Family::Pictures,
            Format::Zip | Format::Tar | Format::Gzip | Format::Xz | Format::Bzip2 => {
                Family::Archives
            }
        }
    }

    /// The engine that diffs this format; two sides are one pair only if they share it.
    pub fn engine(self) -> Engine {
        match self {
            Format::Picture(_) => Engine::Picture,
            other => Engine::Container(other.family()),
        }
    }

    /// The format's name as reports show it (`"PNG"`).
    pub fn name(self) -> String {
        match self {
            Format::Picture(format) => format!("{format:?}").to_uppercase(),
            Format::Zip => "ZIP".to_string(),
            Format::Tar => "TAR".to_string(),
            Format::Gzip => "GZIP".to_string(),
            Format::Xz => "XZ".to_string(),
            Format::Bzip2 => "BZIP2".to_string(),
        }
    }
}

/// How a pair is diffed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Engine {
    /// [`picture`]: pixels and frames.
    Picture,
    /// [`container`]: members of a family, matched by key.
    Container(Family),
}

/// The format `bytes` hold, if it is one OmniDiff diffs by content.
pub fn sniff(bytes: &[u8]) -> Option<Format> {
    use image::ImageFormat::*;
    if let Ok(format @ (Png | Jpeg | Gif | WebP | Bmp | Ico | Tiff)) = image::guess_format(bytes) {
        return Some(Format::Picture(format));
    }
    // An empty zip is only its end-of-directory record.
    if bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06") {
        return Some(Format::Zip);
    }
    if bytes.starts_with(&[0x1F, 0x8B, 0x08]) {
        return Some(Format::Gzip);
    }
    if bytes.starts_with(&[0xFD, b'7', b'z', b'X', b'Z', 0x00]) {
        return Some(Format::Xz);
    }
    if let [b'B', b'Z', b'h', b'1'..=b'9', ..] = bytes {
        return Some(Format::Bzip2);
    }
    if archive::is_tar(bytes) {
        return Some(Format::Tar);
    }
    None
}

/// Decodes `bytes` as the container they are.
pub fn decode_container(bytes: &[u8]) -> Result<Box<dyn Container>> {
    match sniff(bytes) {
        Some(format) if format.family() == Family::Archives => archive::decode(bytes, format),
        Some(format) => bail!("{} is not a container", format.name()),
        None => bail!("not content OmniDiff knows"),
    }
}

/// What a sampler needs to know about one side without decoding all of it: its format, how big
/// it is in the family's own unit (pixels for a picture), and its shape (a picture's size), equal
/// on two sides of the same shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    pub format: Format,
    pub size: u64,
    pub shape: u64,
}

/// [`Probe`]s `bytes`, if they are content and their header reads.
pub fn probe(bytes: &[u8]) -> Option<Probe> {
    let format = sniff(bytes)?;
    match format {
        Format::Picture(_) => {
            let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
                .with_guessed_format()
                .ok()?;
            let (width, height) = reader.into_dimensions().ok()?;
            Some(Probe {
                format,
                size: u64::from(width) * u64::from(height),
                shape: u64::from(width) << 32 | u64::from(height),
            })
        }
        // A container's size is how many members it has, its shape which ones.
        _ => {
            let container = decode_container(bytes).ok()?;
            let mut keys: Vec<&str> = container
                .members()
                .iter()
                .map(|member| member.key.as_str())
                .collect();
            keys.sort_unstable();
            Some(Probe {
                format,
                size: keys.len() as u64,
                shape: container::hash_bytes(keys.join("\n").as_bytes()),
            })
        }
    }
}

/// The engine a pair is diffed with: every side present sniffs to a format of that engine, and
/// at least one side is present. `None` for a pair diffed as text, or reported as binary.
pub fn pair_kind(before: &[u8], after: &[u8]) -> Option<Engine> {
    let engine = |bytes: &[u8]| (!bytes.is_empty()).then(|| sniff(bytes).map(Format::engine));
    match (engine(before), engine(after)) {
        (Some(Some(before)), Some(Some(after))) if before == after => Some(before),
        (Some(Some(engine)), None) | (None, Some(Some(engine))) => Some(engine),
        _ => None,
    }
}

/// What changed between two files of one kind of content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContentDiff {
    Picture(PictureDiff),
    Container(ContainerDiff),
}

impl ContentDiff {
    /// The picture diff, for a picture pair.
    pub fn as_picture(&self) -> Option<&PictureDiff> {
        match self {
            ContentDiff::Picture(diff) => Some(diff),
            ContentDiff::Container(_) => None,
        }
    }

    /// The engine's verdict on the whole pair.
    pub fn verdict(&self) -> Verdict {
        match self {
            ContentDiff::Picture(diff) => diff.verdict(),
            ContentDiff::Container(diff) => diff.verdict(),
        }
    }
}

/// Diffs a pair by its content; `Ok(None)` when it is not a pair [`pair_kind`] recognises.
pub fn diff(before: &[u8], after: &[u8]) -> Result<Option<ContentDiff>> {
    diff_at_depth(before, after, 0)
}

/// [`diff`] of a pair found `depth` containers deep (see [`container::MAX_DEPTH`]).
pub(crate) fn diff_at_depth(
    before: &[u8],
    after: &[u8],
    depth: usize,
) -> Result<Option<ContentDiff>> {
    Ok(match pair_kind(before, after) {
        Some(Engine::Picture) => Some(ContentDiff::Picture(picture::diff(before, after)?)),
        Some(Engine::Container(family)) => {
            let (before, after) = decode_container_pair(before, after)?;
            Some(ContentDiff::Container(container::compare(
                family,
                before.as_deref(),
                after.as_deref(),
                depth,
            )))
        }
        None => None,
    })
}

/// A decoded side of a container pair: `None` for an empty one.
pub type DecodedContainer = Option<Box<dyn Container>>;

/// Both sides of a container pair decoded, on two threads; an empty side is `None`.
pub fn decode_container_pair(
    before: &[u8],
    after: &[u8],
) -> Result<(DecodedContainer, DecodedContainer)> {
    let side = |bytes: &[u8]| {
        (!bytes.is_empty())
            .then(|| decode_container(bytes))
            .transpose()
    };
    let (before, after) = std::thread::scope(|scope| {
        let before = scope.spawn(|| side(before));
        let after = side(after);
        (before.join().expect("decoding a container panicked"), after)
    });
    Ok((before?, after?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png() -> Vec<u8> {
        let mut bytes = Vec::new();
        image::RgbaImage::from_pixel(2, 2, image::Rgba([1, 2, 3, 255]))
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .expect("encodes");
        bytes
    }

    #[test]
    fn a_picture_sniffs_by_its_bytes() {
        assert_eq!(
            sniff(&png()),
            Some(Format::Picture(image::ImageFormat::Png))
        );
        assert_eq!(sniff(b"fn main() {}\n"), None);
        assert_eq!(sniff(b""), None);
    }

    #[test]
    fn a_probe_reads_the_header() {
        let probe = probe(&png()).expect("a picture");
        assert_eq!(probe.format.name(), "PNG");
        assert_eq!(probe.size, 4);
        assert_eq!(probe.shape, 2 << 32 | 2);
    }

    #[test]
    fn a_pair_is_one_engine_or_none() {
        let png = png();
        assert_eq!(pair_kind(&png, &png), Some(Engine::Picture));
        assert_eq!(
            pair_kind(b"", &png),
            Some(Engine::Picture),
            "an added picture"
        );
        assert_eq!(
            pair_kind(&png, b""),
            Some(Engine::Picture),
            "a deleted picture"
        );
        assert_eq!(pair_kind(b"", b""), None);
        assert_eq!(pair_kind(&png, b"text"), None);
    }

    #[test]
    fn families_name_their_datasets() {
        for family in Family::ALL {
            assert_eq!(Family::from_name(family.name()), Some(family));
        }
        assert_eq!(Family::from_name("diffs"), None);
    }
}
