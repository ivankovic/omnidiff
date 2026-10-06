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
//! Draws a content pair's viewer (a picture, an archive, a font, ...) into an offscreen terminal,
//! once per `--step`, for `scripts/record_content_gif.py` to rasterize. See
//! `tui::screenshot::render_content`.
//!
//! Each still is `still-NN.json`: the styled runs `render_tui_screenshot` prints, and `pictures`,
//! the pictures the viewer shows, each `{col, row, cols, rows, file}` with its pixels in `file`
//! (a PNG beside it) - an offscreen terminal has cells, not pixels, so the rasterizer pastes them
//! in, scaled to fit their cells as a kitty terminal would.
//!
//!     make content-gif

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Parser;
use crossterm::event::KeyCode;
use serde::Serialize;

use omnidiff::tui::screenshot::{Screenshot, overlay_theme_named, render_content};
use omnidiff::tui::theme;

#[derive(Parser)]
#[command(about = "Render a content pair's viewer offscreen, step by step, as JSON and PNGs")]
struct Args {
    before: PathBuf,
    after: PathBuf,
    /// Terminal width.
    #[arg(long, default_value_t = 120)]
    cols: u16,
    /// Terminal height.
    #[arg(long, default_value_t = 32)]
    rows: u16,
    /// Overlay theme, by its picker label or variant name.
    #[arg(long, default_value = "Solarized Light")]
    theme: String,
    /// A cell's size in pixels, `WIDTHxHEIGHT`: the rasterizer's, so pictures and outlines are
    /// sized as the GIF shows them.
    #[arg(long, default_value = "9x19")]
    font: String,
    /// The keys to press before a still, separated by spaces (`t`, `l l l`, `Down`, `Enter`); an
    /// empty step is a still of the view as it is. One still per step, in order.
    #[arg(long = "step", required = true)]
    steps: Vec<String>,
    /// Directory to write the stills into.
    #[arg(long)]
    out: PathBuf,
}

/// A still as written: its runs, and where its pictures go.
#[derive(Serialize)]
struct StillFile {
    #[serde(flatten)]
    shot: Screenshot,
    pictures: Vec<PictureFile>,
}

#[derive(Serialize)]
struct PictureFile {
    col: u16,
    row: u16,
    cols: u16,
    rows: u16,
    file: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let overlay = overlay_theme_named(&args.theme)
        .with_context(|| format!("unknown overlay theme {:?}", args.theme))?;
    let font = parse_font(&args.font)?;
    let steps = args
        .steps
        .iter()
        .map(|step| step.split_whitespace().map(parse_key).collect())
        .collect::<Result<Vec<Vec<KeyCode>>>>()?;

    // Opening a pair records it in the config, like the live viewer: keep that out of the real one.
    let scratch_config =
        std::env::temp_dir().join(format!("omnidiff-stills-{}.toml", std::process::id()));
    if std::env::var_os(theme::CONFIG_ENV).is_none() {
        // SAFETY: before any other thread exists.
        unsafe { std::env::set_var(theme::CONFIG_ENV, &scratch_config) };
    }
    let stills = render_content(
        &args.before,
        &args.after,
        (args.cols, args.rows),
        overlay,
        font,
        &steps,
    );
    let _ = std::fs::remove_file(&scratch_config);

    std::fs::create_dir_all(&args.out)?;
    for (index, still) in stills?.into_iter().enumerate() {
        let mut pictures = Vec::with_capacity(still.pictures.len());
        for (number, picture) in still.pictures.into_iter().enumerate() {
            let file = format!("still-{index:02}-{number}.png");
            picture
                .pixels
                .save(args.out.join(&file))
                .with_context(|| format!("writing {file}"))?;
            pictures.push(PictureFile {
                col: picture.area.x,
                row: picture.area.y,
                cols: picture.area.width,
                rows: picture.area.height,
                file,
            });
        }
        let json = serde_json::to_string(&StillFile {
            shot: still.shot,
            pictures,
        })?;
        std::fs::write(args.out.join(format!("still-{index:02}.json")), json)?;
    }
    Ok(())
}

fn parse_font(text: &str) -> Result<(u16, u16)> {
    let (width, height) = text
        .split_once('x')
        .with_context(|| format!("--font is WIDTHxHEIGHT, not {text:?}"))?;
    Ok((width.parse()?, height.parse()?))
}

fn parse_key(name: &str) -> Result<KeyCode> {
    Ok(match name {
        "Enter" => KeyCode::Enter,
        "Backspace" => KeyCode::Backspace,
        "Down" => KeyCode::Down,
        "Up" => KeyCode::Up,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Space" => KeyCode::Char(' '),
        _ => {
            let mut chars = name.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => KeyCode::Char(c),
                _ => bail!("unknown key {name:?}"),
            }
        }
    })
}
