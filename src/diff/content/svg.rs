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
//! add a second, unguarded decoder. The parse runs inside [`super::contained`], so a panic in
//! usvg or roxmltree is an error, whoever asked - `sniff` included.
//!
//! **Only an SVG drawn in full is a picture.** `resvg` is built without `text` (fonts would be
//! the system's, so a drawing would differ between machines) and without raster images, so
//! `<text>`, `<foreignObject>` and any `<image>` but one nested SVG drawn in full come out as
//! nothing: two files that differ only there would draw the same and be called invisible. An SVG
//! with any of them is [`Parsed::complete`] false, and stays text, its markup diffed.
//!
//! **Recognition is by parsing**, and an SVG `usvg` will not open is not an SVG here: it stays
//! whatever else it is - text, XML - so the markup still diffs and a parse error is never shown
//! to a reader as a refusal. One side is parsed once however many callers ask - `sniff`,
//! `pair_kind`, the decode, the size - from a small cache of the last sides read ([`parsed`]).
//!
//! **A small pair is drawn larger.** Each side is drawn at its own size, but a pair of SVGs whose
//! larger side is under [`MIN_COMPARE_SIZE`] is drawn scaled up to it, both sides by one factor
//! ([`pair_scale`]): an icon's edit smaller than one of its own pixels would otherwise round
//! away. An SVG paired with a raster picture is drawn at its own size, to compare with it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use image::RgbaImage;
use resvg::usvg;

/// The largest square an SVG is drawn on, whatever its own size says: an unbounded intrinsic size
/// would be an unbounded pixmap.
pub const MAX_SVG_SIZE: u32 = 1024;

/// The size the larger side of a pair of SVGs is drawn at, at least (see the module doc).
pub const MIN_COMPARE_SIZE: u32 = 256;

/// How far into a file `<svg` must appear, if it does: only the declaration, a doctype, comments
/// and instructions may sit before the root, and those are rarely longer than a page.
const SNIFF_HEAD_BYTES: usize = 4096;

/// The SVG namespace. An element in it, or in none, is the SVG element its name says.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// How many sides [`parsed`] keeps: a pair's two, and the pair before it.
const CACHED_SIDES: usize = 4;

/// An SVG as usvg read it.
pub struct Parsed {
    tree: usvg::Tree,
    /// Whether it draws everything it holds: false for text, a `foreignObject`, or an `<image>`
    /// that is not one nested SVG drawn in full (see the module doc).
    complete: bool,
}

/// usvg's options. `open`: a `data:` URL of an SVG resolves, to one nested SVG parsed closed;
/// every other URL resolves to nothing, and marks the file `incomplete`.
fn options(open: bool, incomplete: Arc<AtomicBool>) -> usvg::Options<'static> {
    let undrawn_data = incomplete.clone();
    usvg::Options {
        resources_dir: None,
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(move |mime: &str, data: Arc<Vec<u8>>, _| {
                let nested = (open && mime == "image/svg+xml")
                    .then(|| parse(&data, false).ok())
                    .flatten();
                if !nested.as_ref().is_some_and(|nested| nested.complete) {
                    undrawn_data.store(true, Ordering::Relaxed);
                }
                nested.map(|nested| usvg::ImageKind::SVG(nested.tree))
            }),
            resolve_string: Box::new(move |_, _| {
                incomplete.store(true, Ordering::Relaxed);
                None
            }),
        },
        ..usvg::Options::default()
    }
}

/// `svg` read as usvg's tree, images nested in it from [`options`], and whether it is drawn in
/// full.
fn parse(svg: &[u8], open: bool) -> Result<Parsed> {
    let text = std::str::from_utf8(svg).context("an SVG that is not UTF-8")?;
    let xml = usvg::roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    let document =
        usvg::roxmltree::Document::parse_with_options(text, xml).context("reading the SVG")?;
    let undrawn = document.descendants().any(|node| {
        let name = node.tag_name();
        node.is_element()
            && matches!(name.namespace(), None | Some(SVG_NAMESPACE))
            && matches!(name.name(), "text" | "foreignObject")
    });
    let incomplete = Arc::new(AtomicBool::new(undrawn));
    let tree = usvg::Tree::from_xmltree(&document, &options(open, incomplete.clone()))
        .context("reading the SVG")?;
    Ok(Parsed {
        tree,
        complete: !incomplete.load(Ordering::Relaxed),
    })
}

/// The last [`CACHED_SIDES`] sides parsed, by their bytes, and what the parse made of them.
type Cache = Vec<(Vec<u8>, Result<Arc<Parsed>, String>)>;
static PARSED: Mutex<Cache> = Mutex::new(Vec::new());

/// `svg` parsed (open), once for every caller that asks of the same bytes while they are among
/// the last few: the bytes are the key, compared whole, so no two files share an entry.
fn parsed(svg: &[u8]) -> Result<Arc<Parsed>> {
    let cache = || {
        PARSED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    };
    if let Some((_, hit)) = cache().iter().find(|(bytes, _)| bytes == svg) {
        return hit.clone().map_err(anyhow::Error::msg);
    }
    let fresh = super::contained("reading an SVG", || parse(svg, true))
        .map(Arc::new)
        .map_err(|error| format!("{error:#}"));
    let mut cache = cache();
    if cache.len() >= CACHED_SIDES {
        drop(cache.remove(0));
    }
    cache.push((svg.to_vec(), fresh.clone()));
    fresh.map_err(anyhow::Error::msg)
}

/// `svg` drawn at its own size times `upscale` (at least 1), scaled down only to fit
/// [`MAX_SVG_SIZE`] if that is larger.
pub fn render(svg: &[u8], upscale: f32) -> Result<RgbaImage> {
    let parsed = parsed(svg)?;
    let (width, height) = (parsed.tree.size().width(), parsed.tree.size().height());
    let fit = MAX_SVG_SIZE as f32 / width.max(height).max(1.0);
    rasterize(&parsed.tree, upscale.max(1.0).min(fit))
}

/// The factor both sides of a pair are drawn up by (see the module doc): 1 unless every side
/// present is an SVG and the larger is under [`MIN_COMPARE_SIZE`].
pub fn pair_scale(before: &[u8], after: &[u8]) -> f32 {
    let sides = [before, after].into_iter().filter(|side| !side.is_empty());
    let mut largest = 0f32;
    for side in sides {
        let Some(parsed) = is_svg(side).then(|| parsed(side).ok()).flatten() else {
            return 1.0;
        };
        let size = parsed.tree.size();
        largest = largest.max(size.width()).max(size.height());
    }
    if largest == 0.0 {
        return 1.0;
    }
    (MIN_COMPARE_SIZE as f32 / largest).max(1.0)
}

/// The size `svg` draws at, `usvg`'s default where the file names none.
pub fn size(svg: &[u8]) -> Result<(u32, u32)> {
    let size = parsed(svg)?.tree.size();
    Ok((
        size.width().round().max(1.0) as u32,
        size.height().round().max(1.0) as u32,
    ))
}

/// `svg` drawn to fit a `size` x `size` square, up or down: a cursor theme's SVG member, drawn at
/// the size its `meta.hl` names. A cursor is drawn even if not in full: it is a preview.
pub fn render_fit(svg: &[u8], size: u32) -> Result<RgbaImage> {
    let parsed = parsed(svg)?;
    let (width, height) = (parsed.tree.size().width(), parsed.tree.size().height());
    let scale = size.min(MAX_SVG_SIZE) as f32 / width.max(height).max(1.0);
    rasterize(&parsed.tree, scale)
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

/// True if `bytes` are an SVG `usvg` opens and draws in full: the word `<svg` within
/// [SNIFF_HEAD_BYTES], then a parse. Element names are case-sensitive in XML, so the root is `svg` small; the parse is what
/// makes the word in a comment no more than text.
pub fn is_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(SNIFF_HEAD_BYTES)];
    let mut windows = head.windows(4);
    let has_svg_root = windows.any(|window| window == b"<svg");
    has_svg_root && parsed(bytes).is_ok_and(|parsed| parsed.complete)
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
        let drawn = render(&svg(10, 10, "#ff0000"), 1.0).expect("a drawing");
        assert_eq!(drawn.dimensions(), (10, 10));
        assert_eq!(drawn.get_pixel(0, 0).0, [255, 0, 0, 255]);
    }

    #[test]
    fn an_svg_draws_no_larger_than_the_cap() {
        let drawn = render(&svg(4096, 2048, "#00ff00"), 4.0).expect("a drawing");
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
        let drawn = render(image(blue.as_bytes()).as_bytes(), 1.0).expect("the nested SVG draws");
        assert_eq!(drawn.get_pixel(0, 0).0, [0, 0, 255, 255]);

        // Two levels: the inner SVG's own data URL is refused, and what is drawn is nothing.
        assert!(is_svg(image(blue.as_bytes()).as_bytes()), "drawn in full");

        // Two levels: the inner SVG's own data URL is refused, and what is drawn is nothing.
        let twice = image(image(blue.as_bytes()).as_bytes());
        let drawn = render(twice.as_bytes(), 1.0).expect("the parse caps the nesting");
        assert!(drawn.pixels().all(|pixel| pixel.0[3] == 0));
        assert!(!is_svg(twice.as_bytes()), "not drawn in full, so text");
    }

    #[test]
    fn an_svg_with_what_this_build_does_not_draw_stays_text() {
        let with = |inner: &str| {
            format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10"/>{inner}</svg>"#
            )
        };
        assert!(is_svg(with("").as_bytes()));
        for undrawn in [
            "<text x=\"1\" y=\"5\">v1.0</text>",
            "<g><text><tspan>label</tspan></text></g>",
            "<foreignObject width=\"5\" height=\"5\"/>",
            "<image href=\"data:image/png;base64,iVBORw0KGgo=\" width=\"5\" height=\"5\"/>",
            "<image href=\"logo.png\" width=\"5\" height=\"5\"/>",
        ] {
            let svg = with(undrawn);
            assert!(!is_svg(svg.as_bytes()), "{undrawn}");
            // Still drawn where only a preview is wanted (a cursor theme's member).
            assert!(render_fit(svg.as_bytes(), 10).is_ok(), "{undrawn}");
        }
    }

    #[test]
    fn a_small_pair_is_drawn_up_by_one_factor() {
        let (small, large) = (svg(16, 16, "red"), svg(64, 32, "red"));
        assert_eq!(pair_scale(&small, &small), 16.0);
        assert_eq!(pair_scale(&small, &large), 4.0, "the larger side decides");
        assert_eq!(pair_scale(b"", &small), 16.0, "an added SVG");
        assert_eq!(pair_scale(&svg(512, 512, "red"), &small), 1.0);
        assert_eq!(
            pair_scale(&small, b"\x89PNG\r\n\x1a\n"),
            1.0,
            "a raster side"
        );
        assert_eq!(
            render(&small, 16.0).expect("draws").dimensions(),
            (256, 256)
        );
    }

    #[test]
    fn a_side_is_parsed_once_for_every_caller() {
        let svg = svg(7, 9, "blue");
        let first = parsed(&svg).expect("parses");
        assert!(Arc::ptr_eq(&first, &parsed(&svg).expect("parses")));
        assert_eq!(size(&svg).expect("a size"), (7, 9));
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
