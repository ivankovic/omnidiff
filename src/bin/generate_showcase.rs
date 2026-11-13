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

//! The GitHub Pages showcase: the real browser viewer, running on static files.
//!
//! The viewer (`assets/viewer/`) only paints the JSON it is sent, so this bakes that JSON for the
//! cases in [`CASES`] and serves the viewer with a `fetch` shim (`assets/showcase/showcase.js`)
//! answering `/api/*` from files. Each case is baked twice: as omnidiff maps it and as GNU `diff`
//! marks it. Published by `.github/workflows/pages.yml` under `showcase/`; nothing is committed.
//!
//! Content that is not code - a PDF, a font, an archive, pictures - is on the same page, in the
//! same picker ([`CONTENT_CASES`]). The browser viewer is the code viewer, so each of these is
//! baked as what the terminal shows instead: `diff`'s own line on the pair, the TUI's content
//! viewer drawn offscreen (`tui::screenshot::render_content`) once per step, as styled cells and
//! the pictures to lay over them, which `showcase.js` puts on the page, and `omnidiff
//! --headless`'s report. Each step is baked once per theme.
//!
//! The page's theme picker lists the overlay themes; [`syntax_theme_for`] pairs each with a syntax
//! theme, so a light overlay sits on a light page. Every case's highlighting is baked under every
//! syntax theme (`cases/<name>.syntax.json`), which is what makes the picker, and the viewer's own
//! `c` dialog, recolour the code.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use crossterm::event::KeyCode;
use serde::Serialize;
use strum::IntoEnumIterator;

use omnidiff::diff::text::{RangeMatch, RenderOptions, TextOperation};
use omnidiff::diff::text_range::TextRange;
use omnidiff::showcase::payload::{
    DiffPayload, RangePayload, SpanPayload, SyntaxPayload, diff_payload, syntax_payload,
};
use omnidiff::showcase::state::default_state;
use omnidiff::test::helper;
use omnidiff::test::helper::human_mapping;
use omnidiff::tui::actions::DiffSessionData;
use omnidiff::tui::app::compute_diff_with_options;
use omnidiff::tui::screenshot::{Screenshot, render_content};
use omnidiff::tui::theme::{self, OverlayTheme, PanelLayout};
use omnidiff::tui::widgets::code_viewer::syntax_theme_names;

#[derive(Parser)]
struct Args {
    /// Directory to write the showcase into. Wiped and recreated on every run.
    #[arg(long, default_value = "site/showcase")]
    out: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Group {
    /// `diff` marks whole lines a reader has to re-diff by eye; omnidiff's mapping matches the
    /// human one line for line.
    DiffWrong,
    /// A plain line diff is already the right answer, and omnidiff says the same thing.
    BothRight,
    /// Not code: `diff` says only that the files differ.
    Content,
}

struct Case {
    /// A fixture directory name under `src/test/data/diffs/<dataset>/`.
    name: &'static str,
    group: Group,
    title: &'static str,
}

/// The twenty. Order is display order within each group.
const CASES: &[Case] = &[
    Case {
        name: "rust-add-if",
        group: Group::DiffWrong,
        title: "Wrap existing code in a new branch",
    },
    Case {
        name: "python-refactoring",
        group: Group::DiffWrong,
        title: "Replace two loops with built-ins",
    },
    Case {
        name: "typescript-add-type-annotations",
        group: Group::DiffWrong,
        title: "Add type annotations",
    },
    Case {
        name: "c-genymobile-scrcpy-rename-defines",
        group: Group::DiffWrong,
        title: "Prefix five macro names",
    },
    Case {
        name: "lua-neovim-neovim-rename",
        group: Group::DiffWrong,
        title: "Rename a field in three tables",
    },
    Case {
        name: "csharp-radarr-radarr-remove-import-and-func",
        group: Group::DiffWrong,
        title: "Delete a method and its import",
    },
    Case {
        name: "python-nvbn-thefuck-add-three-arguments",
        group: Group::DiffWrong,
        title: "Add a parameter and pass it through",
    },
    Case {
        name: "yaml-twbs-bootstrap-remove-v-semicolon-from-version-numbers",
        group: Group::DiffWrong,
        title: "Drop a key from every list item",
    },
    Case {
        name: "ruby-jekyll-jekyll-whitespace-only",
        group: Group::DiffWrong,
        title: "Change line endings only",
    },
    Case {
        name: "xml-antlr-antlr3-comment-out-part-of-code-interesting-case",
        group: Group::DiffWrong,
        title: "Comment out a block of XML",
    },
    Case {
        name: "go-gin-gonic-gin-update-version-string",
        group: Group::BothRight,
        title: "Bump a version string",
    },
    Case {
        name: "java-genymobile-scrcpy-char-to-string-bugfix",
        group: Group::BothRight,
        title: "Fix a char literal that should have been a string",
    },
    Case {
        name: "kotlin-fix-loop-bug",
        group: Group::BothRight,
        title: "Change a loop's range operator",
    },
    Case {
        name: "cpp-ladybirdbrowser-ladybird-change-inherited-class-name",
        group: Group::BothRight,
        title: "Change a base class",
    },
    Case {
        name: "swift-nextcloud-ios-different-func",
        group: Group::BothRight,
        title: "Rename an overridden method",
    },
    Case {
        name: "tsx-mitmproxy-mitmproxy-array-to-object",
        group: Group::BothRight,
        title: "Export an object instead of an array",
    },
    Case {
        name: "rust-tauri-apps-tauri-add-use-and-function",
        group: Group::BothRight,
        title: "Add a field and the import for its type",
    },
    Case {
        name: "python-nvbn-thefuck-stdout-stderr-change",
        group: Group::BothRight,
        title: "Switch four reads from stderr to output",
    },
    Case {
        name: "css-wordpress-wordpress-rename-attribute",
        group: Group::BothRight,
        title: "Change a CSS property",
    },
    Case {
        name: "shellscript-genymobile-scrcpy-insert-only",
        group: Group::BothRight,
        title: "Add one file to a release list",
    },
];

/// One entry of `cases.json`.
#[derive(Serialize)]
struct CaseIndex {
    name: &'static str,
    group: Group,
    title: &'static str,
    dataset: String,
    language: String,
    /// Lines in the after-side file.
    lines: usize,
    /// Lines GNU `diff` marks, both sides together.
    diff_marked: usize,
    /// What omnidiff paints under the default options, per operation. Counted from the baked
    /// ranges, not the payload's `change_counts`, which says "0 updates" for a pure in-line
    /// deletion like `buffer` -> `buf`.
    omnidiff: PaintedCounts,
    /// omnidiff's one-line reading of the whole change, e.g. "Whitespace changes only".
    summary: Option<String>,
    /// The upstream commit the change was taken from, when the fixture is a sampled one.
    upstream: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.out.exists() {
        fs::remove_dir_all(&args.out)
            .with_context(|| format!("removing existing {:?}", args.out))?;
    }
    let cases_dir = args.out.join("cases");
    fs::create_dir_all(&cases_dir)?;

    for (name, contents) in [
        (
            "index.html",
            include_str!("../../assets/showcase/index.html"),
        ),
        (
            "showcase.js",
            include_str!("../../assets/showcase/showcase.js"),
        ),
        (
            "showcase.css",
            include_str!("../../assets/showcase/showcase.css"),
        ),
        // Where the content examples used to be: a redirect to the one page.
        (
            "content.html",
            include_str!("../../assets/showcase/content.html"),
        ),
        ("model.js", include_str!("../../assets/viewer/model.js")),
        ("app.js", include_str!("../../assets/viewer/app.js")),
        ("style.css", include_str!("../../assets/viewer/style.css")),
    ] {
        fs::write(args.out.join(name), contents)?;
    }
    // The stills are drawn from a scratch directory (see `bake_content`), so `--out` must not
    // depend on the working directory.
    let out = fs::canonicalize(&args.out)?;

    // What `/api/state` answers: the defaults, never the local config, so CI and a laptop generate
    // the same site. Dual, not Auto: Auto's 200-column cut-over is single-panel on most browser
    // windows. The paths are placeholders; the shim answers with the selected case.
    let mut state = default_state();
    state.before = Some("before".to_string());
    state.after = Some("after".to_string());
    state.settings.layout = PanelLayout::Dual;
    fs::write(out.join("state.json"), serde_json::to_vec(&state)?)?;
    fs::write(out.join("themes.json"), serde_json::to_vec(&page_themes())?)?;

    let provenance = helper::sample_provenance()?;
    let repository_urls = helper::repository_urls()?;

    let mut index = Vec::with_capacity(CASES.len() + CONTENT_CASES.len());
    for case in CASES {
        let baked = bake(case, &provenance, &repository_urls)
            .with_context(|| format!("baking {}", case.name))?;
        for (suffix, payload) in [
            ("omnidiff", &baked.omnidiff),
            ("minimal", &baked.minimal),
            ("full", &baked.full),
            ("diff", &baked.unix),
        ] {
            fs::write(
                cases_dir.join(format!("{}.{suffix}.json", case.name)),
                serde_json::to_vec(payload)?,
            )?;
        }
        fs::write(
            cases_dir.join(format!("{}.syntax.json", case.name)),
            serde_json::to_vec(&baked.syntax)?,
        )?;
        index.push(serde_json::to_value(&baked.index)?);
    }

    // Opening a pair in the viewer records it in the config, like the live viewer: keep that out
    // of the real one.
    let scratch_config =
        std::env::temp_dir().join(format!("omnidiff-showcase-{}.toml", std::process::id()));
    if std::env::var_os(theme::CONFIG_ENV).is_none() {
        // SAFETY: the generator has one thread.
        unsafe { std::env::set_var(theme::CONFIG_ENV, &scratch_config) };
    }
    for case in CONTENT_CASES {
        let entry = bake_content(case, &out, &repository_urls)
            .with_context(|| format!("baking {}", case.fixture))?;
        index.push(serde_json::to_value(&entry)?);
    }
    let _ = fs::remove_file(&scratch_config);
    fs::write(out.join("cases.json"), serde_json::to_vec(&index)?)?;

    println!(
        "Showcase written to {:?}: {} code changes, {} content examples",
        args.out,
        CASES.len(),
        CONTENT_CASES.len()
    );
    Ok(())
}

/// One theme of the page's picker.
#[derive(Serialize)]
struct PageTheme {
    /// serde's name for the overlay theme, what `/api/settings` carries.
    id: String,
    label: String,
    /// The syntax theme the picker selects with it.
    syntax_theme: &'static str,
}

/// The syntax theme the page pairs with an overlay theme: its own namesake where syntect bundles
/// one, else a dark one, as the overlay presets but Solarized Light are dark.
fn syntax_theme_for(overlay: OverlayTheme) -> &'static str {
    match overlay {
        OverlayTheme::SolarizedLight => "Solarized (light)",
        OverlayTheme::SolarizedDark => "Solarized (dark)",
        OverlayTheme::GruvboxDark => "base16-eighties.dark",
        OverlayTheme::Monokai => "base16-mocha.dark",
        _ => "base16-ocean.dark",
    }
}

/// serde's name for `overlay`, the id `state.json` lists it under.
fn theme_id(overlay: OverlayTheme) -> String {
    serde_json::to_value(overlay)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// The overlay presets. Custom is the user's own palette, which only the viewer's `c` dialog
/// can make, so the picker does not list it.
fn preset_themes() -> impl Iterator<Item = OverlayTheme> {
    OverlayTheme::iter().filter(|overlay| *overlay != OverlayTheme::Custom)
}

/// `themes.json`: the picker's themes, and every syntax theme's page colours, which a content
/// example needs and has no highlighted code to take them from.
#[derive(Serialize)]
struct PageThemes {
    themes: Vec<PageTheme>,
    syntax: Vec<SyntaxPayload>,
}

fn page_themes() -> PageThemes {
    PageThemes {
        themes: preset_themes()
            .map(|overlay| PageTheme {
                id: theme_id(overlay),
                label: overlay.to_string(),
                syntax_theme: syntax_theme_for(overlay),
            })
            .collect(),
        syntax: syntax_theme_names()
            .iter()
            .map(|name| syntax_payload(Some(name)))
            .collect(),
    }
}

/// One content example: a fixture under `src/test/data/<family>/`, and the content viewer's keys,
/// one still per step.
struct ContentCase {
    family: &'static str,
    fixture: &'static str,
    title: &'static str,
    /// The keys pressed before each still, separated by spaces, and what the still shows.
    steps: &'static [(&'static str, &'static str)],
}

/// A picture pair's views, in the order `t` cycles them, then the swipe's divider moved across.
const PICTURE_STEPS: &[(&str, &str)] = &[
    ("", "Side by side, the changed regions outlined"),
    ("t", "The changed areas alone"),
    ("t", "The difference over the faded picture"),
    ("t", "Blend"),
    ("t h h h h h h h h h h", "Swipe: the after picture"),
    ("l l l l l", "Swipe: a quarter across"),
    ("l l l l l", "Swipe: half across"),
    ("l l l l l", "Swipe: three quarters across"),
    ("l l l l l", "Swipe: the before picture"),
];

/// Display order. The steps walk each example's changed members and the views they have.
const CONTENT_CASES: &[ContentCase] = &[
    ContentCase {
        family: "documents",
        fixture: "pdf-x-governikus-ausweisapp-522d8b0b-communicationmodel_en",
        title: "A PDF diagram: one box renamed",
        steps: &[
            ("j", "The metadata, as text"),
            ("j", "Page 1 before and after, the renamed box outlined"),
            ("t", "Page 1: the changed area alone"),
        ],
    },
    ContentCase {
        family: "fonts",
        fixture: "woff2-x-fortawesome-font-awesome-b476ed9a-fa-solid-900",
        title: "A web font: twenty icons redrawn",
        steps: &[
            ("j", "The name table: version and style changed"),
            ("j", "One glyph before and after"),
            ("g", "Every changed glyph at once"),
        ],
    },
    ContentCase {
        family: "archives",
        fixture: "zip-x-silnrsi-teckit-3d06b4f4-teckit_tools",
        title: "An OpenDocument file: its text and its thumbnail",
        steps: &[
            ("j j j j j", "The document's text, a member of the archive"),
            ("k k", "Its thumbnail, a picture inside the archive"),
        ],
    },
    ContentCase {
        family: "catalogs",
        fixture: "mo-x-ubernostrum-django-registration-906912aa-django",
        title: "A compiled translation: three messages reworded",
        steps: &[
            ("j", "The first reworded message"),
            ("j", "The second"),
            ("j", "The third"),
        ],
    },
    ContentCase {
        family: "pictures",
        fixture: "bmp-x-talamus-solarize-12x29-psf-8a856fdb-solarize-12x29",
        title: "A bitmap font sheet: one glyph redrawn",
        steps: PICTURE_STEPS,
    },
    ContentCase {
        family: "pictures",
        fixture: "ico-x-golang-tour-16d3ff4a-favicon",
        title: "A favicon re-exported at a larger size",
        steps: PICTURE_STEPS,
    },
    ContentCase {
        family: "pictures",
        fixture: "gif-x-ascii-boxes-boxes-eea1dfea-readme-0",
        title: "An animated demo with frames added",
        steps: &[
            ("", "The first frame, side by side"),
            (". . .", "Three frames on"),
            (". . .", "Six frames on"),
            ("t", "The changed areas of that frame"),
        ],
    },
];

/// The terminal the content viewer is drawn into, in cells, and a cell's size in pixels: about
/// what the page's 13px monospace cell is, so the pictures are sized as the page shows them.
const CONTENT_TERMINAL: (u16, u16) = (120, 32);
const CONTENT_CELL: (u16, u16) = (8, 18);

/// A content example's entry of `cases.json`.
#[derive(Serialize)]
struct ContentIndex {
    name: &'static str,
    group: Group,
    title: &'static str,
    /// The content family: `pictures`, `fonts`, ...
    family: &'static str,
    /// The file's path in its repository.
    path: String,
    /// What GNU `diff` prints for the pair.
    diff_output: String,
    /// `omnidiff --headless`'s report on the pair.
    report: String,
    /// What OmniDiff calls the change, and what a human does (`None` while unjudged).
    verdict: String,
    human_verdict: Option<String>,
    upstream: Option<String>,
    /// What each still shows, in order: `content/<name>/<theme>/<index>.json`.
    steps: Vec<&'static str>,
}

/// A still as the page reads it: the cells, and where its pictures go.
#[derive(Serialize)]
struct StillFile<'a> {
    #[serde(flatten)]
    shot: &'a Screenshot,
    pictures: Vec<PictureFile>,
}

#[derive(Serialize)]
struct PictureFile {
    col: u16,
    row: u16,
    cols: u16,
    rows: u16,
    /// Relative to `content/<name>/`.
    file: String,
    /// The picture's own size in pixels, so the page can scale it by whole factors.
    width: u32,
    height: u32,
}

/// `- **Label:** value` of a fixture's README, backticks dropped.
fn readme_field(readme: &str, label: &str) -> Option<String> {
    let prefix = format!("- **{label}:** ");
    readme
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(|value| value.trim().trim_matches('`').to_string())
}

/// The upstream commit a content fixture's README names, when it names a repository URL and a
/// commit id.
fn content_upstream(readme: &str) -> Option<String> {
    let repository = readme_field(readme, "Repository")?;
    let url = repository.split_whitespace().next()?;
    let commit = readme_field(readme, "Commit")?;
    let is_id = commit.len() >= 7 && commit.chars().all(|c| c.is_ascii_hexdigit());
    (url.starts_with("https://") && is_id)
        .then(|| format!("{}/commit/{commit}", url.trim_end_matches(".git")))
}

fn parse_key(name: &str) -> Result<KeyCode> {
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Ok(KeyCode::Char(c)),
        _ => Err(anyhow!("{name:?} is not a single-character key")),
    }
}

/// Bakes one content example under `out/content/<fixture>/` and returns its index entry.
///
/// The pair is copied to `a/<name>` and `b/<name>` in a scratch directory and everything is run
/// from there, so `diff`, the report and the viewer's titles name the file as git would.
fn bake_content(
    case: &ContentCase,
    out: &Path,
    repository_urls: &HashMap<String, String>,
) -> Result<ContentIndex> {
    use omnidiff::test::helper::human_content;
    let family = omnidiff::diff::content::Family::from_name(case.family)
        .ok_or_else(|| anyhow!("{} is not a content family", case.family))?;
    let dir = human_content::root(family).join(case.fixture);
    let readme = fs::read_to_string(dir.join("README.md"))
        .with_context(|| format!("reading {dir:?}'s README.md"))?;
    let path = readme_field(&readme, "File")
        .ok_or_else(|| anyhow!("{dir:?}'s README.md names no file"))?;
    let name = Path::new(&path)
        .file_name()
        .ok_or_else(|| anyhow!("{path:?} has no file name"))?;
    let (before_path, after_path) = side_files(&dir)?;

    let scratch = tempfile::tempdir()?;
    let before = Path::new("a").join(name);
    let after = Path::new("b").join(name);
    for (source, target) in [(&before_path, &before), (&after_path, &after)] {
        let target = scratch.path().join(target);
        fs::create_dir_all(target.parent().unwrap_or(scratch.path()))?;
        fs::copy(source, &target)?;
    }
    let home = std::env::current_dir()?;
    std::env::set_current_dir(scratch.path())?;
    let baked = (|| -> Result<ContentIndex> {
        let diff = std::process::Command::new("diff")
            .args([&before, &after])
            .env("LC_ALL", "C")
            .output()
            .context("running diff")?;
        let diff_output = String::from_utf8_lossy(&diff.stdout).trim_end().to_string();

        let (before_bytes, after_bytes) = (fs::read(&before)?, fs::read(&after)?);
        let content = omnidiff::diff::content::diff(&before_bytes, &after_bytes)?
            .ok_or_else(|| anyhow!("{path} is not a pair omnidiff diffs by content"))?;
        let report = omnidiff::tui::positional::content_notice(
            &[before.clone(), after.clone()],
            &before,
            &after,
            &content,
        );
        let human_verdict = human_content::load(family, case.fixture)?
            .verdict
            .map(|judgement| judgement.label());

        let steps = case
            .steps
            .iter()
            .map(|(keys, _)| keys.split_whitespace().map(parse_key).collect())
            .collect::<Result<Vec<Vec<KeyCode>>>>()?;
        let case_out = out.join("content").join(case.fixture);
        fs::create_dir_all(&case_out)?;
        // A picture is the same under most themes: one file per distinct image.
        let mut pictures: HashMap<Vec<u8>, String> = HashMap::new();
        for overlay in preset_themes() {
            let stills = render_content(
                &before,
                &after,
                CONTENT_TERMINAL,
                (overlay, Some(syntax_theme_for(overlay))),
                CONTENT_CELL,
                &steps,
            )?;
            let theme_out = case_out.join(theme_id(overlay));
            fs::create_dir_all(&theme_out)?;
            for (index, still) in stills.iter().enumerate() {
                let mut placed = Vec::with_capacity(still.pictures.len());
                for picture in &still.pictures {
                    let mut png = Vec::new();
                    picture
                        .pixels
                        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)?;
                    let next = format!("picture-{:03}.png", pictures.len());
                    let file = match pictures.get(&png) {
                        Some(file) => file.clone(),
                        None => {
                            fs::write(case_out.join(&next), &png)?;
                            pictures.insert(png, next.clone());
                            next
                        }
                    };
                    placed.push(PictureFile {
                        col: picture.area.x,
                        row: picture.area.y,
                        cols: picture.area.width,
                        rows: picture.area.height,
                        file,
                        width: picture.pixels.width(),
                        height: picture.pixels.height(),
                    });
                }
                fs::write(
                    theme_out.join(format!("{index}.json")),
                    serde_json::to_vec(&StillFile {
                        shot: &still.shot,
                        pictures: placed,
                    })?,
                )?;
            }
        }

        Ok(ContentIndex {
            name: case.fixture,
            group: Group::Content,
            title: case.title,
            family: case.family,
            path: path.clone(),
            diff_output,
            report: report.trim_end().to_string(),
            verdict: content.verdict().label(),
            human_verdict,
            upstream: content_upstream(&readme).or_else(|| {
                helper::readme_provenance(case.fixture)
                    .and_then(|sample| helper::upstream_commit_url(&sample, repository_urls))
            }),
            steps: case.steps.iter().map(|(_, caption)| *caption).collect(),
        })
    })();
    std::env::set_current_dir(home)?;
    baked
}

struct Baked {
    /// omnidiff's diff under the default render options and under each preset the `M` panel
    /// can switch to.
    omnidiff: DiffPayload,
    minimal: DiffPayload,
    full: DiffPayload,
    /// The same two files as GNU `diff` marks them.
    unix: DiffPayload,
    /// The two sides' highlighting under every syntax theme, by theme name: what `/api/highlight`
    /// answers. The same for all four payloads, which differ only in their ranges.
    syntax: HashMap<String, SyntaxSpans>,
    index: CaseIndex,
}

/// `/api/highlight`'s answer for one syntax theme.
#[derive(Serialize)]
struct SyntaxSpans {
    before: Vec<Vec<SpanPayload>>,
    after: Vec<Vec<SpanPayload>>,
    syntax: SyntaxPayload,
}

fn bake(
    case: &Case,
    provenance: &HashMap<String, helper::SampleProvenance>,
    repository_urls: &HashMap<String, String>,
) -> Result<Baked> {
    let dir = helper::diffs_case_dir(case.name)
        .ok_or_else(|| anyhow!("no fixture directory named {}", case.name))?;
    let dataset = dir
        .parent()
        .and_then(Path::file_name)
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (before_path, after_path) = side_files(&dir)?;

    // The diff `/api/diff` and `/api/render_options` answer with, under each preset.
    let (data, large_residual) =
        compute_diff_with_options(&before_path, &after_path, RenderOptions::default())?;
    let omnidiff = diff_payload(&data, large_residual, RenderOptions::default(), None);
    let minimal = diff_payload(&data, large_residual, RenderOptions::MINIMAL, None);
    let full = diff_payload(&data, large_residual, RenderOptions::FULL, None);
    let syntax = syntax_theme_names()
        .into_iter()
        .map(|name| {
            let themed = diff_payload(&data, large_residual, RenderOptions::default(), Some(&name));
            let spans = SyntaxSpans {
                before: themed.before.spans,
                after: themed.after.spans,
                syntax: themed.syntax,
            };
            (name, spans)
        })
        .collect();

    let pair = helper::handmade_test_code_pair(case.name)?;
    let (before_touched, after_touched) = human_mapping::unix_diff_line_labels(&pair.0, &pair.1)?;
    let (before_ranges, after_ranges) = unix_ranges(&before_touched, &after_touched);
    let unix_data = DiffSessionData {
        before_ranges,
        after_ranges,
        comment_only: false,
        plain_text_fallback: false,
        ..data.clone()
    };
    let mut unix = diff_payload(&unix_data, false, RenderOptions::FULL, None);
    unix.summary = None;

    let diff_marked = before_touched.iter().filter(|t| **t).count()
        + after_touched.iter().filter(|t| **t).count();
    let upstream = provenance
        .get(case.name)
        .and_then(|sample| helper::upstream_commit_url(sample, repository_urls));

    let index = CaseIndex {
        name: case.name,
        group: case.group,
        title: case.title,
        dataset,
        language: omnidiff.after.language.clone(),
        lines: omnidiff.after.lines.len(),
        diff_marked,
        omnidiff: PaintedCounts::of(&omnidiff),
        summary: omnidiff.summary.as_ref().map(|s| s.label.to_string()),
        upstream,
    };

    Ok(Baked {
        omnidiff,
        minimal,
        full,
        unix,
        syntax,
        index,
    })
}

/// omnidiff's painted ranges by operation. Updates and moves count the larger side, not the sum:
/// they paint both sides, except a pure in-line deletion (`buffer` -> `buf`), which paints only
/// the before side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
struct PaintedCounts {
    insertions: usize,
    deletions: usize,
    updates: usize,
    moves: usize,
}

impl PaintedCounts {
    fn of(payload: &DiffPayload) -> Self {
        let count =
            |ranges: &[RangePayload], op: &str| ranges.iter().filter(|r| r.op == op).count();
        let (before, after) = (&payload.before.ranges[..], &payload.after.ranges[..]);
        Self {
            insertions: count(after, "insert"),
            deletions: count(before, "delete"),
            updates: count(before, "update").max(count(after, "update")),
            moves: count(before, "move").max(count(after, "move")),
        }
    }
}

/// The paths of a fixture directory's `before.<ext>.test` and `after.<ext>.test` files.
fn side_files(dir: &Path) -> Result<(PathBuf, PathBuf)> {
    let mut before = None;
    let mut after = None;
    for entry in fs::read_dir(dir).with_context(|| format!("reading {dir:?}"))? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with("before.") {
            before = Some(path);
        } else if name.starts_with("after.") {
            after = Some(path);
        }
    }
    match (before, after) {
        (Some(b), Some(a)) => Ok((b, a)),
        _ => Err(anyhow!("{dir:?} lacks a before.*/after.* pair")),
    }
}

/// `diff`'s per-line verdict as ranges. Untouched lines pair up in order; each run of touched
/// lines is one deletion or insertion, anchored at the row the other side has reached (where the
/// viewer's cross-panel cursor lands), as `diff::text::plain_text_diff` does for an unpaired gap.
fn unix_ranges(
    before_touched: &[bool],
    after_touched: &[bool],
) -> (Vec<RangeMatch>, Vec<RangeMatch>) {
    let mut before_ranges = Vec::new();
    let mut after_ranges = Vec::new();
    let (mut b, mut a) = (0, 0);
    loop {
        let b0 = b;
        while b < before_touched.len() && before_touched[b] {
            b += 1;
        }
        let a0 = a;
        while a < after_touched.len() && after_touched[a] {
            a += 1;
        }
        if b > b0 {
            before_ranges.push(RangeMatch {
                source: TextRange::new(b0, 0, b, 0),
                destination: TextRange::new(a, 0, a, 0),
                operation: TextOperation::Delete,
            });
        }
        if a > a0 {
            after_ranges.push(RangeMatch {
                source: TextRange::new(a0, 0, a, 0),
                destination: TextRange::new(b, 0, b, 0),
                operation: TextOperation::Insert,
            });
        }
        if b >= before_touched.len() || a >= after_touched.len() {
            break;
        }
        let before_line = TextRange::new(b, 0, b + 1, 0);
        let after_line = TextRange::new(a, 0, a + 1, 0);
        before_ranges.push(RangeMatch {
            source: before_line.clone(),
            destination: after_line.clone(),
            operation: TextOperation::Identical,
        });
        after_ranges.push(RangeMatch {
            source: after_line,
            destination: before_line,
            operation: TextOperation::Identical,
        });
        b += 1;
        a += 1;
    }
    // Both sides have the same number of untouched lines, so a leftover means bad labels; it is
    // still shown rather than dropped.
    if b < before_touched.len() {
        before_ranges.push(RangeMatch {
            source: TextRange::new(b, 0, before_touched.len(), 0),
            destination: TextRange::new(a, 0, a, 0),
            operation: TextOperation::Delete,
        });
    }
    if a < after_touched.len() {
        after_ranges.push(RangeMatch {
            source: TextRange::new(a, 0, after_touched.len(), 0),
            destination: TextRange::new(b, 0, b, 0),
            operation: TextOperation::Insert,
        });
    }
    (before_ranges, after_ranges)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_case_names_a_fixture_that_exists_and_each_group_has_ten() {
        let mut diff_wrong = 0;
        let mut both_right = 0;
        for case in CASES {
            assert!(
                helper::diffs_case_dir(case.name).is_some(),
                "{} is not a fixture directory",
                case.name
            );
            match case.group {
                Group::DiffWrong => diff_wrong += 1,
                Group::BothRight => both_right += 1,
                Group::Content => panic!("{} is code", case.name),
            }
        }
        assert_eq!((diff_wrong, both_right), (10, 10));
    }

    #[test]
    fn every_content_example_names_a_fixture_and_keys_the_viewer_takes() {
        use omnidiff::test::helper::human_content;
        assert!(!CONTENT_CASES.is_empty());
        for case in CONTENT_CASES {
            let family = omnidiff::diff::content::Family::from_name(case.family)
                .unwrap_or_else(|| panic!("{} is not a content family", case.family));
            let dir = human_content::root(family).join(case.fixture);
            let readme = fs::read_to_string(dir.join("README.md")).unwrap();
            assert!(
                readme_field(&readme, "File").is_some(),
                "{}: its README names no file",
                case.fixture
            );
            side_files(&dir).unwrap();
            assert!(!case.steps.is_empty(), "{}", case.fixture);
            for (keys, caption) in case.steps {
                assert!(!caption.is_empty(), "{}", case.fixture);
                for key in keys.split_whitespace() {
                    parse_key(key).unwrap();
                }
            }
        }
    }

    #[test]
    fn baking_a_content_example_writes_a_still_per_step_and_theme() {
        // nextest gives each test its own process, which `bake_content`'s change of working
        // directory needs.
        let case = CONTENT_CASES
            .iter()
            .find(|case| case.family == "catalogs")
            .unwrap();
        let out = tempfile::tempdir().unwrap();
        // SAFETY: this test's process runs nothing else.
        unsafe { std::env::set_var(theme::CONFIG_ENV, out.path().join("config.toml")) };
        let entry = bake_content(case, out.path(), &HashMap::new()).unwrap();

        assert_eq!(entry.group, Group::Content);
        assert_eq!(entry.steps.len(), case.steps.len());
        assert_eq!(
            entry.diff_output,
            "Binary files a/django.mo and b/django.mo differ"
        );
        assert!(
            entry
                .report
                .starts_with("Catalogs a/django.mo and b/django.mo"),
            "{}",
            entry.report
        );
        assert!(entry.upstream.is_some());
        let case_out = out.path().join("content").join(case.fixture);
        for theme in page_themes().themes {
            for index in 0..case.steps.len() {
                let path = case_out.join(&theme.id).join(format!("{index}.json"));
                let still: serde_json::Value =
                    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                assert_eq!(still["cols"], CONTENT_TERMINAL.0, "{path:?}");
                assert_eq!(
                    still["lines"].as_array().unwrap().len(),
                    usize::from(CONTENT_TERMINAL.1)
                );
            }
        }
    }

    #[test]
    fn every_preset_theme_is_paired_with_a_syntax_theme_that_exists() {
        let names = syntax_theme_names();
        let themes = page_themes().themes;
        assert_eq!(themes.len(), OverlayTheme::iter().count() - 1);
        for theme in &themes {
            assert!(
                names.iter().any(|name| name == theme.syntax_theme),
                "{}: no syntax theme named {}",
                theme.id,
                theme.syntax_theme
            );
            assert_ne!(theme.id, "Custom");
        }
        assert_eq!(
            syntax_theme_for(OverlayTheme::SolarizedLight),
            "Solarized (light)"
        );
    }

    #[test]
    fn a_fixture_readme_gives_the_file_and_the_upstream_commit() {
        let readme = "# Sample provenance\n\n\
            - **Repository:** https://github.com/dzaima/BQN (`dzaima-BQN`)\n\
            - **Commit:** `0bbe096fc07d278b679a8479318f1722d096a03e`\n\
            - **File:** `app/data/BQN386.ttf`\n";
        assert_eq!(
            readme_field(readme, "File").as_deref(),
            Some("app/data/BQN386.ttf")
        );
        assert_eq!(
            content_upstream(readme).as_deref(),
            Some("https://github.com/dzaima/BQN/commit/0bbe096fc07d278b679a8479318f1722d096a03e")
        );
        // A handmade fixture names no commit.
        let handmade = "- **Repository:** https://github.com/pytorch/xla.git (`pytorch-xla.git`)\n\
            - **Commit:** None. Reported as a test case by a user.\n";
        assert_eq!(content_upstream(handmade), None);
    }

    #[test]
    fn case_names_are_unique() {
        let mut names: Vec<_> = CASES.iter().map(|c| c.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), CASES.len());
    }

    #[test]
    fn unix_ranges_pair_untouched_lines_and_run_touched_ones_together() {
        // before: keep, DEL, DEL, keep ; after: keep, INS, keep
        let (before, after) = unix_ranges(&[false, true, true, false], &[false, true, false]);
        let ops = |ranges: &[RangeMatch]| {
            ranges
                .iter()
                .map(|r| r.operation.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ops(&before),
            [
                TextOperation::Identical,
                TextOperation::Delete,
                TextOperation::Identical
            ]
        );
        assert_eq!(
            ops(&after),
            [
                TextOperation::Identical,
                TextOperation::Insert,
                TextOperation::Identical
            ]
        );
        assert_eq!(before[1].source, TextRange::new(1, 0, 3, 0));
        // The deletion is anchored where the after side stands after its own run: row 2.
        assert_eq!(before[1].destination, TextRange::new(2, 0, 2, 0));
        assert_eq!(after[1].source, TextRange::new(1, 0, 2, 0));
        assert_eq!(before[2].source, TextRange::new(3, 0, 4, 0));
        assert_eq!(before[2].destination, TextRange::new(2, 0, 3, 0));
    }

    #[test]
    fn painted_counts_take_each_two_sided_operation_once() {
        let range = |op: &'static str| RangePayload {
            op,
            source: [0; 4],
            destination: [0; 4],
        };
        let side = |ops: &[&'static str], lines: usize| omnidiff::showcase::payload::SidePayload {
            path: String::new(),
            name: String::new(),
            language: String::new(),
            lines: vec![String::new(); lines],
            ranges: ops.iter().map(|op| range(op)).collect(),
            spans: Vec::new(),
        };
        let mut payload = diff_payload(
            &DiffSessionData {
                before_path: PathBuf::new(),
                after_path: PathBuf::new(),
                before_contents: String::new(),
                after_contents: String::new(),
                before_ranges: Vec::new(),
                after_ranges: Vec::new(),
                comment_only: false,
                plain_text_fallback: false,
            },
            false,
            RenderOptions::default(),
            None,
        );
        payload.before = side(
            &["identical", "delete", "update", "update", "update", "move"],
            1,
        );
        payload.after = side(&["identical", "insert", "insert", "move"], 1);
        assert_eq!(
            PaintedCounts::of(&payload),
            PaintedCounts {
                insertions: 2,
                deletions: 1,
                updates: 3,
                moves: 1
            }
        );
    }

    #[test]
    fn unix_ranges_of_a_whole_file_rewrite_is_one_range_per_side() {
        let (before, after) = unix_ranges(&[true, true], &[true, true, true]);
        assert_eq!(before.len(), 1);
        assert_eq!(after.len(), 1);
        assert_eq!(before[0].source, TextRange::new(0, 0, 2, 0));
        assert_eq!(after[0].source, TextRange::new(0, 0, 3, 0));
    }

    #[test]
    fn unix_ranges_of_identical_files_are_all_identical_pairs() {
        let (before, after) = unix_ranges(&[false, false], &[false, false]);
        assert!(
            before
                .iter()
                .all(|r| r.operation == TextOperation::Identical)
        );
        assert_eq!(before.len(), 2);
        assert_eq!(after.len(), 2);
    }

    #[test]
    fn baking_every_case_matches_the_published_scores() {
        // The list promises omnidiff matches the human mapping on every case; a matcher
        // regression must fail here before it is published.
        let provenance = helper::sample_provenance().unwrap();
        let repository_urls = helper::repository_urls().unwrap();
        for case in CASES {
            let baked = bake(case, &provenance, &repository_urls).unwrap();
            assert_eq!(baked.unix.summary, None, "{}", case.name);
            assert!(baked.index.lines > 0, "{}", case.name);
            assert!(
                baked.index.diff_marked > 0,
                "{}: diff marks nothing",
                case.name
            );
            let mismatches = human_mapping::compute_mismatches(case.name).unwrap();
            assert_eq!(
                mismatches.len(),
                0,
                "{}: omnidiff disagrees with the human mapping",
                case.name
            );
        }
    }
}
