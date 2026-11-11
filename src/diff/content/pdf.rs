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

//! PDFs as containers of pages, each page drawn as a picture by hayro, a pure-Rust PDF
//! renderer, and the document's metadata as text.
//!
//! A page is keyed by its number (`page 7`) and hashed by what draws it: its content stream, its
//! size and rotation, and the bytes of every image or form it places. A PDF rebuilt with the same
//! pages (a new modification date, renumbered objects) has no changed page, and only a changed
//! page is drawn, at [`PAGE_SCALE`] times its size in points.
//!
//! Pages are keyed by number, and match by it when both sides have as many. When a page was
//! inserted or removed they are a sequence aligned by hash ([`Container::sequence`]): a page
//! inserted early is one added page, and the pages after it pair with their unchanged
//! counterparts rather than all redrawing. The text on a page is not compared on its own yet; it
//! is in the picture.
//!
//! The PDF is parsed once to list it, and once more by each worker that draws its pages, which
//! draws them all from that one parse ([`Container::opener`]), not once per page. Workers do not
//! share a parse: hayro drawing pages of one `Pdf` on several threads at once draws them
//! differently from run to run (a fixture's unchanged page 8 came out changed one run in ten).
//! Each page still gets a fresh `RenderCache`, as before: one kept across a worker's pages
//! would make a page's pixels depend on which pages that worker drew first.
//!
//! **What a PDF makes OmniDiff decode is bounded.** Listing hashes at most [`MAX_HASHED_BYTES`]
//! of content streams and placed objects over all pages (a page's `/Contents` may name one
//! stream many times, and every page may place the same image): a content stream is inflated
//! here, counting, and one whose filters cannot be counted is decoded only if it cannot expand
//! past what is left ([`MAX_FILTER_RATIO`]). Past the budget a page is hashed by its raw bytes and
//! not drawn. A page whose content passes [`MAX_DRAWN_CONTENT_BYTES`] is not drawn either, and a
//! page larger than [`MAX_PAGE_PIXELS`] at [`PAGE_SCALE`] is drawn smaller, to fit. What hayro
//! decodes while drawing a page (its images, its forms' streams) has no limit OmniDiff can set.

use std::hash::Hasher;
use std::io::Read;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::object::{Array, Name, Stream};
use hayro::hayro_syntax::page::Page;
use hayro::hayro_syntax::{Filter, Pdf};
use image::RgbaImage;

use super::container::{Container, ContainerInfo, Member, MemberContent, Opener};
use crate::diff::picture::{Frames, PictureInfo};

/// How large a page is drawn: 1.0 is a pixel per point, 72 to the inch.
pub const PAGE_SCALE: f32 = 1.25;

/// The most bytes listing a PDF hashes, over all its pages: decoded content streams and the
/// objects pages place. Hashing is streamed, so this bounds time, not memory.
pub const MAX_HASHED_BYTES: u64 = 1024 * 1024 * 1024;

/// The most a stream whose filters are not counted here may expand per byte: deflate's limit
/// (1032), with room for LZW's.
pub const MAX_FILTER_RATIO: u64 = 1400;

/// The most decoded content a page drawn may have: hayro decodes it again to draw it, all at once.
pub const MAX_DRAWN_CONTENT_BYTES: u64 = 256 * 1024 * 1024;

/// The most pixels a page is drawn with: an A0 poster at [`PAGE_SCALE`] has 12.5 million.
pub const MAX_PAGE_PIXELS: f32 = 16_777_216.0;

/// True if `bytes` are a PDF: `%PDF-` in their first kilobyte, where the format allows it.
pub fn is_pdf(bytes: &[u8]) -> bool {
    bytes[..bytes.len().min(1024)]
        .windows(5)
        .any(|window| window == b"%PDF-")
}

/// A PDF, decoded to its pages and metadata.
pub struct Document {
    info: ContainerInfo,
    pdf: Pdf,
    members: Vec<Member>,
    /// Whether each page may be drawn (see the module doc).
    drawable: Vec<bool>,
    metadata: String,
}

impl Document {
    pub fn new(bytes: &[u8]) -> Result<Self> {
        let pdf = Pdf::new(Arc::new(bytes.to_vec()))
            .map_err(|err| anyhow::anyhow!("reading the PDF: {err:?}"))?;
        let pages = pdf.pages();
        let metadata = metadata(&pdf, pages.len());
        let mut members = vec![Member {
            key: "metadata".to_string(),
            hash: super::container::hash_bytes(metadata.as_bytes()),
        }];
        let mut drawable = Vec::with_capacity(pages.len());
        let mut left = MAX_HASHED_BYTES;
        for (index, page) in pages.iter().enumerate() {
            let (hash, draws) = page_hash(page, &mut left);
            members.push(Member {
                key: format!("page {}", index + 1),
                hash,
            });
            drawable.push(draws);
        }
        Ok(Self {
            info: ContainerInfo {
                format: "PDF".to_string(),
                bytes: bytes.len(),
                members: members.len(),
            },
            pdf,
            members,
            drawable,
            metadata,
        })
    }

    /// This PDF parsed again, for one thread to draw from (see the module doc).
    fn reload(&self) -> Result<Pdf> {
        Pdf::new(self.pdf.data().clone()).map_err(|err| anyhow::anyhow!("reading the PDF: {err:?}"))
    }

    /// Member `index`'s content, a page drawn from `pdf`, this document parsed on this thread.
    fn open_from(&self, pdf: &Pdf, index: usize) -> Result<MemberContent> {
        if index == 0 {
            return Ok(MemberContent::Text(self.metadata.clone()));
        }
        let page = pdf.pages().get(index - 1).context("a page past the end")?;
        if !self.drawable[index - 1] {
            bail!("a page of more content than is drawn");
        }
        let pixels = render(page, &hayro::RenderCache::new())?;
        let (width, height) = pixels.dimensions();
        let frames = Frames::from_frames(vec![(pixels, 0)])?;
        let info = PictureInfo {
            format: "PAGE".to_string(),
            width,
            height,
            color: "RGBA8".to_string(),
            bytes: 0,
            frames: 1,
            duration_ms: 0,
        };
        Ok(MemberContent::Picture(Box::new((info, frames))))
    }
}

impl Container for Document {
    fn info(&self) -> &ContainerInfo {
        &self.info
    }

    fn members(&self) -> &[Member] {
        &self.members
    }

    fn open(&self, index: usize) -> Result<MemberContent> {
        self.open_from(&self.reload()?, index)
    }

    fn opener(&self) -> Opener<'_> {
        let mut pdf = None;
        Box::new(move |index| {
            if pdf.is_none() {
                pdf = Some(self.reload()?);
            }
            self.open_from(pdf.as_ref().expect("loaded"), index)
        })
    }

    fn sequence(&self) -> bool {
        true
    }
}

/// A hash of what draws `page`: its content, size, rotation, and the images and forms it places;
/// and whether it may be drawn. Hashing spends `left` of [`MAX_HASHED_BYTES`].
fn page_hash(page: &Page, left: &mut u64) -> (u64, bool) {
    let mut hasher = metrohash::MetroHash64::default();
    let content = content_hash(page, left, &mut hasher);
    let drawable = content.is_some_and(|bytes| bytes <= MAX_DRAWN_CONTENT_BYTES);
    let (width, height) = page.render_dimensions();
    hasher.write(&width.to_le_bytes());
    hasher.write(&height.to_le_bytes());
    hasher.write(format!("{:?}", page.rotation()).as_bytes());
    let resources = page.resources();
    let mut names: Vec<Name> = resources.x_objects.keys().collect();
    names.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
    for name in names {
        hasher.write(name.as_ref());
        if let Some(object) = resources.get_x_object(&name) {
            let data = object.raw_data();
            match left.checked_sub(data.len() as u64) {
                Some(rest) => {
                    *left = rest;
                    hasher.write(&data);
                }
                // Past the budget an object is told apart by its size alone.
                None => hasher.write(&data.len().to_le_bytes()),
            }
        }
    }
    (hasher.finish(), drawable)
}

/// Feeds what draws `page` to `hasher`: its content streams decoded, each followed by a space, as
/// hayro joins them. Returns how many bytes they decoded to, or `None` if past what is `left` (or
/// what a stream may decode to could not be told), when the rest is hashed as raw bytes.
fn content_hash(page: &Page, left: &mut u64, hasher: &mut impl Hasher) -> Option<u64> {
    let contents = page.raw();
    let streams: Vec<Stream> = if let Some(stream) = contents.get::<Stream>(b"Contents") {
        vec![stream]
    } else if let Some(array) = contents.get::<Array>(b"Contents") {
        array.iter::<Stream>().collect()
    } else {
        Vec::new()
    };
    let mut total = Some(0u64);
    for stream in &streams {
        let decoded = total.and_then(|_| decode_counted(stream, *left));
        match decoded {
            Some(bytes) => {
                *left -= bytes.len() as u64;
                total = total.map(|total| total + bytes.len() as u64);
                hasher.write(&bytes);
            }
            None => {
                total = None;
                hasher.write(&stream.raw_data());
            }
        }
        hasher.write(b" ");
    }
    total
}

/// `stream` decoded, if it decodes to at most `limit` bytes: inflated here, counting, when deflate
/// is its one filter, and by hayro when its other filters cannot take it past `limit`.
fn decode_counted(stream: &Stream, limit: u64) -> Option<Vec<u8>> {
    let raw = stream.raw_data();
    let filters = stream.filters();
    let has_params =
        stream.dict().contains_key(b"DecodeParms") || stream.dict().contains_key(b"DP");
    if filters.is_empty() {
        return (raw.len() as u64 <= limit).then(|| raw.into_owned());
    }
    if let [Filter::FlateDecode] = filters.as_slice()
        && !has_params
    {
        let inflate = |reader: &mut dyn Read| {
            let mut bytes = Vec::new();
            reader.take(limit + 1).read_to_end(&mut bytes).ok()?;
            Some(bytes)
        };
        let inflated = inflate(&mut flate2::read::ZlibDecoder::new(&raw[..]))
            .or_else(|| inflate(&mut flate2::read::DeflateDecoder::new(&raw[..])));
        if let Some(bytes) = inflated {
            return (bytes.len() as u64 <= limit).then_some(bytes);
        }
    }
    let ratio = filters.iter().try_fold(1u64, |ratio, filter| {
        let expands = match filter {
            Filter::AsciiHexDecode | Filter::Ascii85Decode | Filter::Crypt => 1,
            Filter::RunLengthDecode => 128,
            Filter::FlateDecode | Filter::LzwDecode => MAX_FILTER_RATIO,
            // Picture filters: never a page's content.
            _ => return None,
        };
        ratio.checked_mul(expands)
    })?;
    if (raw.len() as u64).checked_mul(ratio)? > limit {
        return None;
    }
    let bytes = stream.decoded().ok()?.into_owned();
    (bytes.len() as u64 <= limit).then_some(bytes)
}

/// `page` drawn on white at [`PAGE_SCALE`], or smaller to fit [`MAX_PAGE_PIXELS`].
fn render<'a>(page: &'a Page<'a>, cache: &hayro::RenderCache<'a>) -> Result<RgbaImage> {
    let (width, height) = page.render_dimensions();
    let area = width * height * PAGE_SCALE * PAGE_SCALE;
    if !area.is_finite() {
        bail!("a page of {width} x {height} points");
    }
    let scale = PAGE_SCALE
        .min(PAGE_SCALE * (MAX_PAGE_PIXELS / area).sqrt())
        .min(f32::from(u16::MAX) / width.max(height));
    if (width * scale).floor() < 1.0 || (height * scale).floor() < 1.0 {
        bail!("a page of {width} x {height} points, too thin to draw");
    }
    let pixmap = hayro::render(
        page,
        cache,
        &InterpreterSettings::default(),
        &hayro::RenderSettings {
            x_scale: scale,
            y_scale: scale,
            bg_color: hayro::vello_cpu::color::palette::css::WHITE,
            ..Default::default()
        },
    );
    let (width, height) = (u32::from(pixmap.width()), u32::from(pixmap.height()));
    let rgba = pixmap
        .take_unpremultiplied()
        .into_iter()
        .flat_map(|pixel| [pixel.r, pixel.g, pixel.b, pixel.a])
        .collect();
    RgbaImage::from_raw(width, height, rgba).context("a page's pixels")
}

/// The document's metadata, one field per line.
fn metadata(pdf: &Pdf, pages: usize) -> String {
    let meta = pdf.metadata();
    let text = |value: &Option<Vec<u8>>| value.as_ref().map(|bytes| pdf_text(bytes));
    let mut out = format!("PDF version: {:?}\npages: {pages}\n", pdf.version());
    for (label, value) in [
        ("title", text(&meta.title)),
        ("author", text(&meta.author)),
        ("subject", text(&meta.subject)),
        ("keywords", text(&meta.keywords)),
        ("creator", text(&meta.creator)),
        ("producer", text(&meta.producer)),
        (
            "created",
            meta.creation_date.as_ref().map(|date| format!("{date:?}")),
        ),
        (
            "modified",
            meta.modification_date
                .as_ref()
                .map(|date| format!("{date:?}")),
        ),
    ] {
        if let Some(value) = value {
            out.push_str(&format!("{label}: {value}\n"));
        }
    }
    out
}

/// A PDF text string: UTF-16BE behind its byte order mark, else PDFDocEncoding, read here as
/// Latin-1 (they differ only in a few punctuation marks).
fn pdf_text(bytes: &[u8]) -> String {
    match bytes {
        [0xFE, 0xFF, rest @ ..] => {
            let units: Vec<u16> = rest
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u16::from_be_bytes(*pair))
                .collect();
            String::from_utf16_lossy(&units)
        }
        _ => bytes.iter().map(|&byte| char::from(byte)).collect(),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::diff::content::ContentDiff;
    use crate::diff::content::container::{MemberDetail, MemberStatus};

    /// A PDF of `pages`, each a content stream on a 100 x 100 point page.
    pub(crate) fn pdf(pages: &[&str]) -> Vec<u8> {
        pdf_sized(pages, "0 0 100 100")
    }

    /// A PDF of `pages`, each a content stream on a page of `media_box`.
    fn pdf_sized(pages: &[&str], media_box: &str) -> Vec<u8> {
        let mut objects: Vec<Vec<u8>> = vec![
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            Vec::new(), // the page tree, once the pages are numbered
        ];
        let mut kids = Vec::new();
        for content in pages {
            let page = objects.len() + 1;
            kids.push(format!("{page} 0 R"));
            objects.push(
                format!(
                    "<< /Type /Page /Parent 2 0 R /MediaBox [{media_box}] /Contents {} 0 R >>",
                    page + 1
                )
                .into_bytes(),
            );
            objects.push(stream("", format!("{content}\n").as_bytes()));
        }
        objects[1] = format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids.join(" "),
            pages.len()
        )
        .into_bytes();
        assemble(&objects)
    }

    /// A stream object of `data`, its dictionary holding `entries` and its length.
    fn stream(entries: &str, data: &[u8]) -> Vec<u8> {
        let mut object = format!("<< {entries} /Length {} >>\nstream\n", data.len()).into_bytes();
        object.extend_from_slice(data);
        object.extend_from_slice(b"\nendstream");
        object
    }

    /// A PDF of `objects`, numbered from 1, the first the catalog.
    fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend(format!("{} 0 obj\n", index + 1).bytes());
            out.extend_from_slice(object);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).bytes());
        for offset in offsets {
            out.extend(format!("{offset:010} 00000 n \n").bytes());
        }
        out.extend(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                objects.len() + 1
            )
            .bytes(),
        );
        out
    }

    const SQUARE: &str = "0 0 0 rg 10 10 30 30 re f";
    const BIG_SQUARE: &str = "0 0 0 rg 10 10 80 80 re f";

    #[test]
    fn a_pdf_is_its_metadata_and_pages() -> Result<()> {
        let bytes = pdf(&[SQUARE, BIG_SQUARE]);
        assert!(is_pdf(&bytes));
        let document = Document::new(&bytes)?;
        let keys: Vec<&str> = document.members().iter().map(|m| m.key.as_str()).collect();
        assert_eq!(keys, vec!["metadata", "page 1", "page 2"]);
        let MemberContent::Picture(page) = document.open(1)? else {
            panic!("a page is a picture");
        };
        let (info, frames) = *page;
        assert_eq!((info.width, info.height), (125, 125));
        // Inside the square (PDF's y grows up), and outside it.
        assert_eq!(frames.pixels(0).get_pixel(30, 95).0, [0, 0, 0, 255]);
        assert_eq!(frames.pixels(0).get_pixel(110, 10).0, [255, 255, 255, 255]);
        Ok(())
    }

    #[test]
    fn only_the_changed_page_is_drawn_and_compared() -> Result<()> {
        let before = pdf(&[SQUARE, SQUARE]);
        let after = pdf(&[SQUARE, BIG_SQUARE]);
        let Some(ContentDiff::Container(diff)) = crate::diff::content::diff(&before, &after)?
        else {
            panic!("a document pair");
        };
        let status = |key: &str| diff.member(key).map(|member| member.status);
        assert_eq!(status("page 1"), None, "unchanged");
        assert_eq!(status("page 2"), Some(MemberStatus::Changed));
        assert!(matches!(
            diff.member("page 2")
                .and_then(|member| member.detail.as_ref()),
            Some(MemberDetail::Picture(_))
        ));
        Ok(())
    }

    #[test]
    fn a_huge_page_is_drawn_smaller_to_fit() -> Result<()> {
        let bytes = pdf_sized(&[SQUARE, BIG_SQUARE], "0 0 50000 50000");
        let document = Document::new(&bytes)?;
        let mut open = document.opener();
        for index in [1, 2] {
            let MemberContent::Picture(page) = open(index)? else {
                panic!("a page is a picture");
            };
            let (width, height) = (page.0.width, page.0.height);
            assert!(
                width == height && (width * height) as f32 <= MAX_PAGE_PIXELS,
                "{width} x {height}"
            );
        }
        Ok(())
    }

    #[test]
    fn a_page_naming_one_stream_many_times_is_hashed_within_its_budget() -> Result<()> {
        use std::io::Write;
        // A megabyte of content, deflated to a kilobyte, named ten times by the page.
        let mut deflated = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        deflated.write_all(&vec![b' '; 1024 * 1024])?;
        let contents = ["4 0 R"; 10].join(" ");
        let bytes = assemble(&[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents [{contents}] >>")
                .into_bytes(),
            stream("/Filter /FlateDecode", &deflated.finish()?),
        ]);
        let pdf = Pdf::new(Arc::new(bytes.clone())).expect("a PDF");
        let page = pdf.pages().iter().next().expect("a page");
        let mut left = 3 * 1024 * 1024 + 10;
        let (_, drawable) = page_hash(page, &mut left);
        assert!(!drawable, "past the budget, the page is not drawn");
        assert!(left < 1024 * 1024, "three of the ten fit: {left}");
        let mut plenty = MAX_HASHED_BYTES;
        let (_, drawable) = page_hash(page, &mut plenty);
        assert!(drawable);
        assert_eq!(MAX_HASHED_BYTES - plenty, 10 * 1024 * 1024);
        Ok(())
    }

    #[test]
    fn a_page_inserted_in_the_middle_is_one_added_page() -> Result<()> {
        const DOT: &str = "0 0 0 rg 50 50 2 2 re f";
        let before = pdf(&[SQUARE, BIG_SQUARE, SQUARE, BIG_SQUARE]);
        let after = pdf(&[SQUARE, BIG_SQUARE, DOT, SQUARE, BIG_SQUARE]);
        let Some(ContentDiff::Container(diff)) = crate::diff::content::diff(&before, &after)?
        else {
            panic!("a document pair");
        };
        let changes: Vec<(&str, MemberStatus)> = diff
            .members
            .iter()
            .map(|member| (member.key.as_str(), member.status))
            .collect();
        assert_eq!(
            changes,
            vec![
                ("metadata", MemberStatus::Changed),
                ("page 3", MemberStatus::Added)
            ],
            "the pages after it are unchanged"
        );
        assert_eq!(diff.unchanged, 4);
        // The other way round, the page is removed.
        let Some(ContentDiff::Container(diff)) = crate::diff::content::diff(&after, &before)?
        else {
            panic!("a document pair");
        };
        assert_eq!(
            diff.member("page 3").map(|member| member.status),
            Some(MemberStatus::Removed)
        );
        assert_eq!(diff.unchanged, 4);
        Ok(())
    }
}
