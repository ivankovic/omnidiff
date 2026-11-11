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
//!
//! **Both files are hostile.** Under `GIT_EXTERNAL_DIFF` OmniDiff reads other people's commits,
//! so no file may make it allocate without bound, hang, read local files or take the process
//! down. Each decoder caps what its format can amplify (a named constant beside it), and two
//! guards span the whole diff:
//!
//! * **One expansion budget** ([`MAX_DIFF_EXPANDED_BYTES`]): every byte decompressed - on both
//!   sides, at every nesting level, on every worker thread - is charged to it
//!   ([`charge_expanded`]). A per-container cap alone would multiply by the nesting depth and the
//!   worker count. The budget is the calling thread's, and a thread spawned for the diff takes
//!   its parent's ([`Budget::current`], [`Budget::enter`]).
//! * **Panics are contained** ([`contained`]): the decoders are third-party code (image, zip,
//!   hayro, skrifa, usvg) a crafted file can make panic. [`diff`] never panics: a panic decoding a
//!   side is that side's error, so the pair is reported as binary, and a panic opening one
//!   member is that member not opening.

pub mod archive;
pub mod catalog;
pub mod class;
pub mod container;
pub mod cursor;
pub mod font;
pub mod pdf;

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};

use super::picture::{self, PictureDiff};
use container::{Container, ContainerDiff};

/// How much a change changed, smallest first: the level of a [`Verdict`], the one question a
/// content fixture's human verdict answers, and the engine's answer to it
/// ([`ContentDiff::verdict`]).
///
/// **The level judges the content once the [`Tag`]s are undone.** A picture scaled to twice its
/// size and nothing else is `invisible` (or `artifacts`, if resampling left traces) and tagged
/// `resized`; a new picture at a new size is `replaced` and tagged `resized`. Frames added or
/// removed are content, not undone: they count towards the level as well as tagging it.
///
/// The first three are three strengths of "nothing", by how hard one has to look; the last three
/// are kinds of "something", by what the after side is - not by how much of it changed.
///
/// **Text calls four of them by plainer names** ([`Vocabulary::Text`]): identical for invisible,
/// whitespace for imperceptible, formatting for artifacts, rewritten for redrawn. **A whole
/// container takes two levels** ([`Vocabulary::Container`]) - an archive, a font, a PDF, a message
/// catalog, a cursor of several pictures: identical (invisible - no member added, removed or
/// changed, whatever its timestamps or compression) and changed (edited), and says what changed
/// with the [`Tag::AddedFiles`], [`Tag::RemovedFiles`] and [`Tag::ChangedFiles`] tags; how much a
/// member changed is that member's own verdict. **So do bytes nothing more is known about**
/// ([`Vocabulary::Binary`]): with no content to look at, the only question is whether they
/// differ. Only the names differ: files and stubs spell every level one way ([`Level::name`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Nothing to see even where OmniDiff points: only bytes differ (metadata, a lossless
    /// re-encoding, colours under fully transparent pixels; for text, line endings or the
    /// encoding).
    Invisible,
    /// Nothing to see by flipping between the sides, but visible once OmniDiff highlights it (a
    /// lossy re-encoding's noise, a colour one step off; trailing whitespace).
    Imperceptible,
    /// Visible, but nobody chose it: what an algorithm left behind - compression, resampling,
    /// dithering, antialiasing, a different exporter or rasteriser; for text, a formatter's
    /// reflowing or re-indenting.
    Artifacts,
    /// A part changed, deliberately, and the rest is the same.
    Edited,
    /// No one part changed but the whole did, and it is still a version of the same thing: a
    /// restyled icon, a recoloured theme, a retaken screenshot; text rewritten throughout.
    Redrawn,
    /// The after side is not a version of the before side.
    Replaced,
}

impl Level {
    /// In the order of `human_solver`'s keys `0`-`5`.
    pub const ALL: [Level; 6] = [
        Level::Invisible,
        Level::Imperceptible,
        Level::Artifacts,
        Level::Edited,
        Level::Redrawn,
        Level::Replaced,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Level::Invisible => "invisible",
            Level::Imperceptible => "imperceptible",
            Level::Artifacts => "artifacts",
            Level::Edited => "edited",
            Level::Redrawn => "redrawn",
            Level::Replaced => "replaced",
        }
    }

    pub fn from_name(name: &str) -> Option<Level> {
        Level::ALL.into_iter().find(|level| level.name() == name)
    }

    /// What `vocabulary` calls the level: [`Level::name`] for pictures, plainer names for text,
    /// containers and binary bytes.
    pub fn name_in(self, vocabulary: Vocabulary) -> &'static str {
        match (vocabulary, self) {
            (Vocabulary::Text | Vocabulary::Container | Vocabulary::Binary, Level::Invisible) => {
                "identical"
            }
            (Vocabulary::Text, Level::Imperceptible) => "whitespace",
            (Vocabulary::Text, Level::Artifacts) => "formatting",
            (Vocabulary::Text, Level::Redrawn) => "rewritten",
            (Vocabulary::Container | Vocabulary::Binary, Level::Edited) => "changed",
            (_, level) => level.name(),
        }
    }

    /// The levels `vocabulary` judges with, in key order: all six, or the two of a container or
    /// of binary bytes.
    pub fn offered(vocabulary: Vocabulary) -> &'static [Level] {
        match vocabulary {
            Vocabulary::Container | Vocabulary::Binary => &[Level::Invisible, Level::Edited],
            _ => &Level::ALL,
        }
    }
}

/// Which names a [`Level`] goes by on screen (see the note on [`Level`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vocabulary {
    Pictures,
    Text,
    /// A whole container: an archive, a font, a PDF, a catalog, a cursor of several pictures.
    Container,
    /// Bytes nothing more is known about ([`container::MemberDetail::Binary`]).
    Binary,
}

/// What a verdict is about, for which [`Tag`]s it can take: a picture's shape and timing, a
/// container's members, nothing for text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Picture,
    Container,
    Text,
}

/// How the shape of a change differs, beside its [`Level`]: any number of these, each mostly
/// checkable from the files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tag {
    /// Scaled to a different size.
    Resized,
    /// Cropped or padded: the canvas changed.
    Canvas,
    /// Rotated or flipped.
    Rotated,
    /// An animation's frames show for different times.
    Timing,
    /// An animation gained or lost frames.
    Frames,
    /// A container gained members: an archive's files, a font's glyphs, a PDF's pages.
    AddedFiles,
    /// A container lost members.
    RemovedFiles,
    /// Members both sides of a container hold differ.
    ChangedFiles,
}

impl Tag {
    pub const ALL: [Tag; 8] = [
        Tag::Resized,
        Tag::Canvas,
        Tag::Rotated,
        Tag::Timing,
        Tag::Frames,
        Tag::AddedFiles,
        Tag::RemovedFiles,
        Tag::ChangedFiles,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Tag::Resized => "resized",
            Tag::Canvas => "canvas",
            Tag::Rotated => "rotated",
            Tag::Timing => "timing",
            Tag::Frames => "frames",
            Tag::AddedFiles => "added_files",
            Tag::RemovedFiles => "removed_files",
            Tag::ChangedFiles => "changed_files",
        }
    }

    pub fn from_name(name: &str) -> Option<Tag> {
        Tag::ALL.into_iter().find(|tag| tag.name() == name)
    }

    /// True if a verdict about `shape` can take the tag.
    pub fn fits(self, shape: Shape) -> bool {
        match self {
            Tag::AddedFiles | Tag::RemovedFiles | Tag::ChangedFiles => shape == Shape::Container,
            _ => shape == Shape::Picture,
        }
    }
}

/// A set of [`Tag`]s, in [`Tag::ALL`]'s order.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Tags(u16);

impl Tags {
    pub fn contains(self, tag: Tag) -> bool {
        self.0 & Self::bit(tag) != 0
    }

    pub fn with(self, tag: Tag) -> Self {
        Self(self.0 | Self::bit(tag))
    }

    pub fn toggle(&mut self, tag: Tag) {
        self.0 ^= Self::bit(tag);
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn iter(self) -> impl Iterator<Item = Tag> {
        Tag::ALL.into_iter().filter(move |tag| self.contains(*tag))
    }

    fn bit(tag: Tag) -> u16 {
        1 << tag as u16
    }
}

impl std::fmt::Debug for Tags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

/// What happened to a picture, a member or a whole container: a [`Level`] and its [`Tag`]s.
/// Written `level+tag+tag` (`artifacts+resized`), as stubs and ground truth files spell it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Verdict {
    pub level: Level,
    pub tags: Tags,
}

impl Verdict {
    pub fn new(level: Level) -> Self {
        Self {
            level,
            tags: Tags::default(),
        }
    }

    pub fn with(self, tag: Tag) -> Self {
        Self {
            tags: self.tags.with(tag),
            ..self
        }
    }

    /// `artifacts+resized`.
    pub fn label(self) -> String {
        self.to_string()
    }

    /// [`Verdict::label`] with the level named in `vocabulary` (`formatting`, `changed`).
    pub fn label_in(self, vocabulary: Vocabulary) -> String {
        let mut label = self.level.name_in(vocabulary).to_string();
        for tag in self.tags.iter() {
            label.push('+');
            label.push_str(tag.name());
        }
        label
    }
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.level.name())?;
        for tag in self.tags.iter() {
            write!(f, "+{}", tag.name())?;
        }
        Ok(())
    }
}

impl std::str::FromStr for Verdict {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self> {
        let mut parts = text.split('+');
        let level = parts.next().unwrap_or_default();
        let mut verdict = Verdict::new(
            Level::from_name(level).with_context(|| format!("no level is called '{level}'"))?,
        );
        for tag in parts {
            verdict = verdict
                .with(Tag::from_name(tag).with_context(|| format!("no tag is called '{tag}'"))?);
        }
        Ok(verdict)
    }
}

/// The groups content is sampled, annotated and reported in: one fixture dataset each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Family {
    Pictures,
    Archives,
    Cursors,
    Fonts,
    Catalogs,
    Documents,
}

impl Serialize for Family {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

impl Family {
    pub const ALL: [Family; 6] = [
        Family::Pictures,
        Family::Archives,
        Family::Cursors,
        Family::Fonts,
        Family::Catalogs,
        Family::Documents,
    ];

    /// The family's fixture dataset, `src/test/data/<name>/`, and its `sample.csv` tag.
    pub fn name(self) -> &'static str {
        match self {
            Family::Pictures => "pictures",
            Family::Archives => "archives",
            Family::Cursors => "cursors",
            Family::Fonts => "fonts",
            Family::Catalogs => "catalogs",
            Family::Documents => "documents",
        }
    }

    /// What one file of the family is called in a report (`"Picture"`), or two (`"Pictures"`).
    pub fn noun(self, plural: bool) -> &'static str {
        match (self, plural) {
            (Family::Pictures, false) => "Picture",
            (Family::Pictures, true) => "Pictures",
            (Family::Archives, false) => "Archive",
            (Family::Archives, true) => "Archives",
            (Family::Cursors, false) => "Cursor",
            (Family::Cursors, true) => "Cursors",
            (Family::Fonts, false) => "Font",
            (Family::Fonts, true) => "Fonts",
            (Family::Catalogs, false) => "Catalog",
            (Family::Catalogs, true) => "Catalogs",
            (Family::Documents, false) => "Document",
            (Family::Documents, true) => "Documents",
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
            // X cursors carry no extension at all: the sampler sniffs extensionless files too.
            Family::Cursors => &["cur", "ani", "hlc"],
            Family::Fonts => &["ttf", "otf", "ttc", "otc", "woff", "woff2", "eot"],
            Family::Catalogs => &["mo", "gmo", "qm"],
            Family::Documents => &["pdf"],
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
    /// A Windows cursor.
    Cur,
    /// A Windows animated cursor.
    Ani,
    /// An X cursor.
    Xcursor,
    /// A Hyprland cursor: a zip with a `meta.hl`.
    Hyprcursor,
    /// TrueType or OpenType, or a collection of them.
    Sfnt,
    Woff,
    Woff2,
    /// Embedded OpenType.
    Eot,
    /// A gettext catalog.
    Mo,
    /// A Qt catalog.
    Qm,
    Pdf,
}

impl Format {
    pub fn family(self) -> Family {
        match self {
            Format::Picture(_) => Family::Pictures,
            Format::Zip | Format::Tar | Format::Gzip | Format::Xz | Format::Bzip2 => {
                Family::Archives
            }
            Format::Cur | Format::Ani | Format::Xcursor | Format::Hyprcursor => Family::Cursors,
            Format::Sfnt | Format::Woff | Format::Woff2 | Format::Eot => Family::Fonts,
            Format::Mo | Format::Qm => Family::Catalogs,
            Format::Pdf => Family::Documents,
        }
    }

    /// The engine that diffs this format; two sides are one pair only if they share it.
    pub fn engine(self) -> Engine {
        match self {
            // A single cursor is a picture, still or animated.
            Format::Picture(_) | Format::Cur | Format::Ani => Engine::Picture,
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
            Format::Cur => "CUR".to_string(),
            Format::Ani => "ANI".to_string(),
            Format::Xcursor => "XCURSOR".to_string(),
            Format::Hyprcursor => "HYPRCURSOR".to_string(),
            Format::Sfnt => "SFNT".to_string(),
            Format::Woff => "WOFF".to_string(),
            Format::Woff2 => "WOFF2".to_string(),
            Format::Eot => "EOT".to_string(),
            Format::Mo => "MO".to_string(),
            Format::Qm => "QM".to_string(),
            Format::Pdf => "PDF".to_string(),
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
    if cursor::is_cur(bytes) {
        return Some(Format::Cur);
    }
    if font::is_sfnt(bytes) {
        return Some(Format::Sfnt);
    }
    if font::is_woff(bytes) {
        return Some(Format::Woff);
    }
    if font::is_woff2(bytes) {
        return Some(Format::Woff2);
    }
    if font::is_eot(bytes) {
        return Some(Format::Eot);
    }
    if catalog::is_mo(bytes) {
        return Some(Format::Mo);
    }
    if catalog::is_qm(bytes) {
        return Some(Format::Qm);
    }
    if pdf::is_pdf(bytes) {
        return Some(Format::Pdf);
    }
    if cursor::is_ani(bytes) {
        return Some(Format::Ani);
    }
    if cursor::is_xcursor(bytes) {
        return Some(Format::Xcursor);
    }
    // An empty zip is only its end-of-directory record.
    if bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06") {
        if cursor::is_hyprcursor(bytes) {
            return Some(Format::Hyprcursor);
        }
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
        Some(Format::Xcursor) => Ok(Box::new(cursor::Xcursor::new(bytes)?)),
        Some(Format::Hyprcursor) => Ok(Box::new(cursor::Hyprcursor::new(bytes)?)),
        Some(format) if format.family() == Family::Fonts => {
            Ok(Box::new(font::Font::new(bytes, format)?))
        }
        Some(Format::Mo) => Ok(Box::new(catalog::Catalog::mo(bytes)?)),
        Some(Format::Qm) => Ok(Box::new(catalog::Catalog::qm(bytes)?)),
        Some(Format::Pdf) => Ok(Box::new(pdf::Document::new(bytes)?)),
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
        Format::Cur | Format::Ani => {
            let (info, _) = picture::decode(bytes).ok()?;
            Some(Probe {
                format,
                size: u64::from(info.width) * u64::from(info.height),
                shape: u64::from(info.width) << 32 | u64::from(info.height),
            })
        }
        // A font's size is its glyphs, counted without drawing them.
        Format::Sfnt | Format::Woff | Format::Woff2 | Format::Eot => Some(Probe {
            format,
            size: font::glyph_count(bytes, format).ok()?,
            shape: container::hash_bytes(
                font::member_keys(bytes, format).ok()?.join("\n").as_bytes(),
            ),
        }),
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
///
/// The one entry point a caller can rely on: whatever the bytes, it returns, within one
/// [`MAX_DIFF_EXPANDED_BYTES`] budget, and a panic in a decoder comes back as an `Err`.
pub fn diff(before: &[u8], after: &[u8]) -> Result<Option<ContentDiff>> {
    Budget::within(|| contained("diffing the pair", || diff_at_depth(before, after, 0)))
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

/// Both sides of a container pair decoded, on two threads; an empty side is `None`. A side whose
/// decoder panics is an error.
pub fn decode_container_pair(
    before: &[u8],
    after: &[u8],
) -> Result<(DecodedContainer, DecodedContainer)> {
    let side = |bytes: &[u8]| {
        contained("decoding a container", || {
            (!bytes.is_empty())
                .then(|| decode_container(bytes))
                .transpose()
        })
    };
    Budget::within(|| {
        let budget = Budget::current();
        let (before, after) = std::thread::scope(|scope| {
            let before = scope.spawn(|| Budget::enter(budget, || side(before)));
            let after = side(after);
            (joined(before.join(), "decoding a container"), after)
        });
        Ok((before?, after?))
    })
}

/// The most bytes one diff may decompress, over both sides, every container nested in them and
/// every worker thread: two sides each at [`container::MAX_EXPANDED_BYTES`], and as much again
/// for the archives inside them. Past it, what is still to expand does not open.
pub const MAX_DIFF_EXPANDED_BYTES: u64 = 4 * container::MAX_EXPANDED_BYTES;

/// What is left of one diff's [`MAX_DIFF_EXPANDED_BYTES`], shared by the threads working on it.
#[derive(Clone, Debug)]
pub struct Budget(Arc<AtomicU64>);

thread_local! {
    static BUDGET: RefCell<Option<Budget>> = const { RefCell::new(None) };
}

impl Budget {
    fn full() -> Self {
        Self::of(MAX_DIFF_EXPANDED_BYTES)
    }

    /// A budget of `bytes`: [`MAX_DIFF_EXPANDED_BYTES`] for a diff, less for a test.
    pub(crate) fn of(bytes: u64) -> Self {
        Self(Arc::new(AtomicU64::new(bytes)))
    }

    /// The budget this thread is working within, for a thread it spawns to take along.
    pub fn current() -> Option<Budget> {
        BUDGET.with(|budget| budget.borrow().clone())
    }

    /// `f` within `budget` on this thread (a spawned worker's parent's), or within a full one if
    /// there is none.
    pub fn enter<T>(budget: Option<Budget>, f: impl FnOnce() -> T) -> T {
        struct Restore(Option<Budget>);
        impl Drop for Restore {
            fn drop(&mut self) {
                BUDGET.with(|budget| *budget.borrow_mut() = self.0.take());
            }
        }
        let budget = budget.unwrap_or_else(Budget::full);
        let _restore = Restore(BUDGET.with(|current| current.borrow_mut().replace(budget)));
        f()
    }

    /// `f` within this thread's budget, or a full one if it has none: an entry point's.
    pub fn within<T>(f: impl FnOnce() -> T) -> T {
        match Budget::current() {
            Some(_) => f(),
            None => Budget::enter(None, f),
        }
    }
}

/// Charges `bytes` just decompressed to this thread's [`Budget`]: an error once the diff has spent
/// it. Without a budget (a decoder called on its own) only the decoder's own caps apply.
pub fn charge_expanded(bytes: u64) -> Result<()> {
    let Some(Budget(left)) = Budget::current() else {
        return Ok(());
    };
    let mut current = left.load(Ordering::Relaxed);
    loop {
        let Some(rest) = current.checked_sub(bytes) else {
            bail!("the diff expands past {MAX_DIFF_EXPANDED_BYTES} bytes");
        };
        match left.compare_exchange_weak(current, rest, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return Ok(()),
            Err(actual) => current = actual,
        }
    }
}

/// `f`, with a panic in it turned into an error: `what` panicked.
pub fn contained<T>(what: &str, f: impl FnOnce() -> Result<T>) -> Result<T> {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|panic| Err(panicked(what, &*panic)))
}

/// A thread's [`std::thread::Result`] as an error if it panicked while doing `what`.
pub fn joined<T>(result: std::thread::Result<Result<T>>, what: &str) -> Result<T> {
    result.unwrap_or_else(|panic| Err(panicked(what, &*panic)))
}

fn panicked(what: &str, panic: &(dyn std::any::Any + Send)) -> anyhow::Error {
    let message = panic
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("a panic");
    anyhow!("{what} panicked: {message}")
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

    #[test]
    fn a_panic_is_contained_as_an_error() {
        let result: Result<()> = contained("decoding", || panic!("a crafted file"));
        let error = result.expect_err("a panic is an error");
        assert_eq!(error.to_string(), "decoding panicked: a crafted file");
    }

    #[test]
    fn a_spawned_thread_spends_its_parents_budget() {
        let budget = Budget::of(10);
        Budget::enter(Some(budget.clone()), || {
            assert!(charge_expanded(6).is_ok());
            let parent = Budget::current();
            std::thread::scope(|scope| {
                scope.spawn(|| {
                    Budget::enter(parent, || {
                        assert!(charge_expanded(6).is_err(), "only 4 bytes are left");
                        assert!(charge_expanded(4).is_ok());
                    })
                });
            });
            assert!(charge_expanded(1).is_err());
        });
        assert!(
            Budget::current().is_none(),
            "the budget ends with its scope"
        );
        assert!(
            Budget::within(|| charge_expanded(1 << 20)).is_ok(),
            "a fresh one"
        );
    }
}
