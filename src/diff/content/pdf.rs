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

//! PDFs as containers of pages, each page drawn as a picture by hayro, a pure-Rust PDF
//! renderer, and the document's metadata as text.
//!
//! A page is keyed by its number (`page 7`) and hashed by what draws it: its content stream, its
//! size and rotation, and the bytes of every image or form it places. A PDF rebuilt with the same
//! pages (a new modification date, renumbered objects) has no changed page, and only a changed
//! page is drawn, at [`PAGE_SCALE`] times its size in points.
//!
//! Pages are matched by number, so a page inserted early makes every later page "changed": they
//! are compared as pictures and say what moved. The text on a page is not compared on its own yet;
//! it is in the picture.

use std::hash::Hasher;
use std::sync::Arc;

use anyhow::{Context, Result};
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::hayro_syntax::object::Name;
use image::RgbaImage;

use super::container::{Container, ContainerInfo, Member, MemberContent};
use crate::diff::picture::{Frames, PictureInfo};

/// How large a page is drawn: 1.0 is a pixel per point, 72 to the inch.
pub const PAGE_SCALE: f32 = 1.25;

/// True if `bytes` are a PDF: `%PDF-` in their first kilobyte, where the format allows it.
pub fn is_pdf(bytes: &[u8]) -> bool {
    bytes[..bytes.len().min(1024)]
        .windows(5)
        .any(|window| window == b"%PDF-")
}

/// A PDF, decoded to its pages and metadata.
pub struct Document {
    info: ContainerInfo,
    data: Arc<Vec<u8>>,
    members: Vec<Member>,
    metadata: String,
}

fn load(data: &Arc<Vec<u8>>) -> Result<Pdf> {
    Pdf::new(data.clone()).map_err(|err| anyhow::anyhow!("reading the PDF: {err:?}"))
}

impl Document {
    pub fn new(bytes: &[u8]) -> Result<Self> {
        let data = Arc::new(bytes.to_vec());
        let pdf = load(&data)?;
        let pages = pdf.pages();
        let metadata = metadata(&pdf, pages.len());
        let mut members = vec![Member {
            key: "metadata".to_string(),
            hash: super::container::hash_bytes(metadata.as_bytes()),
        }];
        for (index, page) in pages.iter().enumerate() {
            members.push(Member {
                key: format!("page {}", index + 1),
                hash: page_hash(page),
            });
        }
        Ok(Self {
            info: ContainerInfo {
                format: "PDF".to_string(),
                bytes: bytes.len(),
                members: members.len(),
            },
            data,
            members,
            metadata,
        })
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
        if index == 0 {
            return Ok(MemberContent::Text(self.metadata.clone()));
        }
        let pdf = load(&self.data)?;
        let page = pdf.pages().get(index - 1).context("a page past the end")?;
        let pixels = render(page)?;
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

/// A hash of what draws `page`: its content, size, rotation, and the images and forms it places.
fn page_hash(page: &hayro::hayro_syntax::page::Page) -> u64 {
    let mut hasher = metrohash::MetroHash64::default();
    hasher.write(page.page_stream().unwrap_or_default());
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
            hasher.write(&object.raw_data());
        }
    }
    hasher.finish()
}

/// `page` drawn on white at [`PAGE_SCALE`].
fn render(page: &hayro::hayro_syntax::page::Page) -> Result<RgbaImage> {
    let pixmap = hayro::render(
        page,
        &hayro::RenderCache::new(),
        &InterpreterSettings::default(),
        &hayro::RenderSettings {
            x_scale: PAGE_SCALE,
            y_scale: PAGE_SCALE,
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
                .chunks_exact(2)
                .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
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
        let mut objects: Vec<String> = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            String::new(), // the page tree, once the pages are numbered
        ];
        let mut kids = Vec::new();
        for content in pages {
            let page = objects.len() + 1;
            kids.push(format!("{page} 0 R"));
            objects.push(format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents {} 0 R >>",
                page + 1
            ));
            objects.push(format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len() + 1
            ));
        }
        objects[1] = format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids.join(" "),
            pages.len()
        );
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend(format!("{} 0 obj\n{object}\nendobj\n", index + 1).bytes());
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
        let after = pdf(&[SQUARE, BIG_SQUARE, SQUARE]);
        let Some(ContentDiff::Container(diff)) = crate::diff::content::diff(&before, &after)?
        else {
            panic!("a document pair");
        };
        let status = |key: &str| diff.member(key).map(|member| member.status);
        assert_eq!(status("page 1"), None, "unchanged");
        assert_eq!(status("page 2"), Some(MemberStatus::Changed));
        assert_eq!(status("page 3"), Some(MemberStatus::Added));
        assert!(matches!(
            diff.member("page 2")
                .and_then(|member| member.detail.as_ref()),
            Some(MemberDetail::Picture(_))
        ));
        Ok(())
    }
}
