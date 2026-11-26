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

//! SVG, read by `usvg` and drawn to pixels with `resvg` - a picture side of a pair, and the
//! previews cursor themes' SVG members are drawn for ([`render_fit`]).
//!
//! **Both files are hostile** (see [`super`]'s note): an SVG never reads the filesystem - not an
//! `href` it names (`/dev/zero` or a FIFO included), only `data:` URLs - and a `data:` URL
//! carries at most one nested SVG, whose own URLs are all refused. usvg has no cap on that
//! nesting, and a self-referencing `data:image/svg+xml` would recurse until the stack is gone,
//! which is not a panic anything can catch; one level is the depth every renderer of real files
//! leaves unused. A `data:` URL of a raster picture never decodes: `resvg` is built without its
//! `raster-images` feature, so the bytes would go undrawn anyway, and decoding them here would
//! add a second, unguarded decoder.
//!
//! **Recognition is by parsing**, and an SVG `usvg` will not open is not an SVG here: it stays
//! whatever else it is - text, XML - so the markup still diffs and a parse error is never shown
//! to a reader as a refusal. The parse is what [`super::sniff`] also runs, once per pair.

use anyhow::{Context, Result};
use image::RgbaImage;
use resvg::usvg;

/// The largest square an SVG is drawn on, whatever its own size says: an unbounded intrinsic size
/// would be an unbounded pixmap.
pub const MAX_SVG_SIZE: u32 = 1024;

/// How far into a file `<svg` must appear, if it does: only the declaration, a doctype, comments
/// and instructions may sit before the root, and those are rarely longer than a page.
const SNIFF_HEAD_BYTES: usize = 4096;

/// usvg's options. `open`: a `data:` URL resolves - to a raster kind only usvg would ignore, to
/// one nested SVG drawn from [`closed`] - and any path-shaped `href` to the filesystem resolves
/// to nothing.
fn options(open: bool) -> usvg::Options<'static> {
    usvg::Options {
        resources_dir: None,
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(move |mime: &str, data: std::sync::Arc<Vec<u8>>, _| {
                (open && mime == "image/svg+xml")
                    .then(|| tree(&data, false).ok())
                    .flatten()
                    .map(usvg::ImageKind::SVG)
            }),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    }
}

/// `svg` read as usvg's tree, images nested in it from [`options`].
fn tree(svg: &[u8], open: bool) -> Result<usvg::Tree> {
    usvg::Tree::from_data(svg, &options(open)).context("reading the SVG")
}

/// `svg` drawn at its own size, scaled only to fit [`MAX_SVG_SIZE`] if it is larger - never
/// upscaled, so what is compared is what the file draws.
pub fn render(svg: &[u8]) -> Result<RgbaImage> {
    let tree = tree(svg, true)?;
    let (width, height) = (tree.size().width(), tree.size().height());
    let fit = MAX_SVG_SIZE as f32 / width.max(height).max(1.0);
    rasterize(&tree, fit.min(1.0))
}

/// The size `svg` draws at, `usvg`'s default where the file names none.
pub fn size(svg: &[u8]) -> Result<(u32, u32)> {
    let tree = tree(svg, false)?;
    let size = tree.size();
    Ok((
        size.width().round().max(1.0) as u32,
        size.height().round().max(1.0) as u32,
    ))
}

/// `svg` drawn to fit a `size` x `size` square, up or down: a cursor theme's SVG member, drawn at
/// the size its `meta.hl` names.
pub fn render_fit(svg: &[u8], size: u32) -> Result<RgbaImage> {
    let tree = tree(svg, true)?;
    let (width, height) = (tree.size().width(), tree.size().height());
    let scale = size.min(MAX_SVG_SIZE) as f32 / width.max(height).max(1.0);
    rasterize(&tree, scale)
}

/// `tree` drawn at `scale`, its alpha premultiplied back to `image`'s plain RGBA.
fn rasterize(tree: &usvg::Tree, scale: f32) -> Result<RgbaImage> {
    use resvg::tiny_skia;
    let (width, height) = (tree.size().width(), tree.size().height());
    let (w, h) = (
        ((width * scale).round() as u32).max(1),
        ((height * scale).round() as u32).max(1),
    );
    let mut pixmap = tiny_skia::Pixmap::new(w, h).context("an SVG of no size")?;
    resvg::render(
        tree,
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

/// True if `bytes` are an SVG `usvg` opens: the word `<svg` within [SNIFF_HEAD_BYTES], then a
/// parse. Element names are case-sensitive in XML, so the root is `svg` small; the parse is what
/// makes the word in a comment no more than text.
pub fn is_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(SNIFF_HEAD_BYTES)];
    let mut windows = head.windows(4);
    let has_svg_root = windows.any(|window| window == b"<svg");
    has_svg_root && tree(bytes, true).is_ok()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn svg(width: u32, height: u32, fill: &str) -> Vec<u8> {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}"><rect width="{width}" height="{height}" fill="{fill}"/></svg>"#
        )
        .into_bytes()
    }

    #[test]
    fn an_svg_is_its_bytes_not_its_name() {
        assert!(is_svg(&svg(16, 16, "red")));
        assert!(is_svg(
            br#"<?xml version="1.0"?><!-- a note --><svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1"/></svg>"#
        ));
        assert!(!is_svg(b"fn main() {}\n"), "a source file");
        assert!(!is_svg(b""), "an empty side");
        assert!(!is_svg(b"broken"), "no markup inside");
    }

    #[test]
    fn an_svg_usvg_does_not_open_stays_text() {
        // The word, the hope, and an unclosed tag: refused at the parse, which is what makes the
        // word in these bytes innocuous elsewhere too.
        assert!(!is_svg(
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48"><rect fill="</svg>"#
        ));
    }

    #[test]
    fn an_svg_word_only_in_a_comment_is_not_an_svg() {
        assert!(!is_svg(
            b"<!-- <svg pretend> -->\nplain markup that is not an SVG\n"
        ));
    }

    #[test]
    fn an_svg_draws_at_its_own_size() {
        let drawn = render(&svg(10, 10, "#ff0000")).expect("a drawing");
        assert_eq!(drawn.dimensions(), (10, 10));
        assert_eq!(drawn.get_pixel(0, 0).0, [255, 0, 0, 255]);
    }

    #[test]
    fn an_svg_draws_no_larger_than_the_cap() {
        let drawn = render(&svg(4096, 2048, "#00ff00")).expect("a drawing");
        assert_eq!(
            drawn.dimensions(),
            (
                MAX_SVG_SIZE,
                ((MAX_SVG_SIZE as f32 / 2.0).round() as u32).max(1)
            ),
            "fit to the square, the aspect kept"
        );
    }

    #[test]
    fn a_data_url_carries_one_nested_svg_at_most() {
        let image = |inner: &[u8]| {
            format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><image href="data:image/svg+xml;base64,{}"/></svg>"#,
                base64_of(inner)
            )
        };
        // One level: the nested SVG draws.
        let blue = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#0000ff"/></svg>"##;
        let drawn = render(image(blue.as_bytes()).as_bytes()).expect("the nested SVG draws");
        assert_eq!(drawn.get_pixel(0, 0).0, [0, 0, 255, 255]);

        // Two levels: the inner SVG's own data URL is refused, and what is drawn is nothing.
        let drawn = render(image(image(blue.as_bytes()).as_bytes()).as_bytes())
            .expect("the parse caps the nesting");
        assert!(drawn.pixels().all(|pixel| pixel.0[3] == 0));
    }

    fn base64_of(bytes: &[u8]) -> String {
        // The dependency `data_url` would need; the hand-rolled table keeps the test hermetic.
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in bytes.chunks(3) {
            let b = [Some(chunk[0]), chunk.get(1).copied(), chunk.get(2).copied()];
            let n = (b[0].map(u32::from).unwrap_or(0) << 16)
                | (b[1].map(u32::from).unwrap_or(0) << 8)
                | b[2].map(u32::from).unwrap_or(0);
            out.push(TABLE[(n >> 18) as usize & 63] as char);
            out.push(TABLE[(n >> 12) as usize & 63] as char);
            out.push(if b[1].is_some() {
                TABLE[(n >> 6) as usize & 63] as char
            } else {
                '='
            });
            out.push(if b[2].is_some() {
                TABLE[n as usize & 63] as char
            } else {
                '='
            });
        }
        out
    }
}
