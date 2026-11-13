# This file is part of the OmniDiff code diffing tool.
#
# Copyright (C) 2026 Marko Ivankovic
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, version 3 of the License.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.

"""Takes the README's pictures from the showcase page itself, in a headless browser.

    make readme-pictures            # all of them
    make readme-screenshot          # assets/readme-screenshot.png
    make diff-gif                   # assets/diff-vs-omnidiff.gif
    make content-gif                # assets/content/*.gif

The showcase (`generate_showcase`) is the product replayed in a browser: code in the viewer's
port, other content as the TUI's own widgets drew it. The pictures are screenshots of that page,
so they cannot drift from it, and through it from the product. Nothing here decides what is
painted. This file only says which addresses of the page to take, and puts frames together:

* the screenshot is the viewer on one code change, under a light theme;
* the diff GIF is that viewer twice on one code change, as Unix `diff` marks it and as OmniDiff
  maps it, with a bar wiping from one to the other. Both are the same two files in the same
  viewer and only the painting differs, so the code does not move under the bar and the reader's
  eye stays on the line it was on;
* a content GIF is one content example's screens in order: `diff`'s line, each step of
  OmniDiff's viewer, then its report.

The browser is Playwright's Chromium in a podman container (scripts/showcase_shots/Containerfile),
driven by record.js, so the pictures do not depend on the browser or the fonts of the machine
that records them. The frames come back as PNGs and are assembled here with Pillow, which is why
this runs in research/'s uv environment. Needs podman, and the network the first time, for the
image.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile

import matplotlib
from PIL import Image, ImageDraw, ImageFont

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
HERE = pathlib.Path(__file__).resolve().parent
IMAGE = "omnidiff-showcase-shots"

# The code change the screenshot and the diff GIF show.
CODE_CASE = "python-refactoring"
# The light theme the screenshot and the content GIFs are taken in; the diff GIF is in the page's
# default theme, so the README shows both a light and a dark one.
LIGHT_THEME = "SolarizedLight"
# The content examples the README shows, by fixture name.
CONTENT_CASES = [
    "pdf-x-governikus-ausweisapp-522d8b0b-communicationmodel_en",
    "woff2-x-fortawesome-font-awesome-b476ed9a-fa-solid-900",
]
# The terminal the generator drew the content viewer in (`CONTENT_TERMINAL` in
# src/bin/generate_showcase.rs), in cells.
TERMINAL = {"cols": 120, "rows": 32}

# The viewer's window for a code change: wide enough for both panels of CODE_CASE without
# wrapping, tall enough for its longer side.
VIEWER_WIDTH = 1180
VIEWER_HEIGHT = 760

# How long a content GIF shows each screen, in milliseconds.
DIFF_LINE_MS = 2200
REPORT_MS = 3800
STEP_MS = 2600
LAST_STEP_MS = 3600

# The band above the diff GIF's panels carrying the tool's name. It is drawn at both ends of every
# frame so that a bar anywhere in the middle leaves one copy on each side: the reader sees
# "OmniDiff" over the revealed half and "Unix diff" over the half still to be swept. A single
# centred label would sit exactly under the bar at the halfway frame.
LABEL_H = 34
LABEL_PAD = 12
LABEL_FONT_SIZE = 15
BAR_W = 3
BAR_COLOR = "#00cdcd"
# Frames for one sweep. The GIF plays the sweep out and back, so the file holds twice this many
# frames plus the two holds - the dominant term in its size, and the first thing to cut if it grows.
SWEEP_FRAMES = 34
# GIF stores delays in hundredths of a second, so anything not a multiple of 10 is floored on the
# way out.
FRAME_MS = 40
HOLD_MS = 1500
# Every frame of a GIF is quantised against one shared palette, drawn from all of its frames: the
# frames compress as deltas, and a colour that is in one frame only (an outline, a highlight) does
# not shimmer in the others.
DIFF_PALETTE_COLORS = 96
CONTENT_PALETTE_COLORS = 255


def run(*command: str | pathlib.Path, **kwargs) -> None:
    subprocess.run([str(part) for part in command], check=True, **kwargs)


def bake(site: pathlib.Path) -> None:
    """The showcase, as the Pages workflow builds it."""
    run(
        "cargo",
        "run",
        "--release",
        "--features",
        "test-fixtures",
        "--bin",
        "generate_showcase",
        "--",
        "--out",
        site,
        cwd=ROOT,
        stdout=subprocess.DEVNULL,
    )


def record(site: pathlib.Path, jobs: list[dict], frames: pathlib.Path) -> None:
    """One PNG per job, taken by record.js in the container."""
    if shutil.which("podman") is None:
        sys.exit("showcase_shots: needs podman")
    run("podman", "build", "-q", "-t", IMAGE, HERE, stdout=subprocess.DEVNULL)
    (frames / "jobs.json").write_text(json.dumps(jobs))
    run(
        "podman",
        "run",
        "--rm",
        "-v",
        f"{site}:/site:ro",
        "-v",
        f"{HERE}:/scripts:ro",
        "-v",
        f"{frames}:/frames",
        IMAGE,
        "node",
        "/scripts/record.js",
        "/site",
        "/frames/jobs.json",
        "/frames",
        stdout=subprocess.DEVNULL,
    )


def query(case: str, view: str, theme: str | None = None, step: int = 0) -> str:
    """The showcase address of one screen (`buildQuery` in assets/showcase/showcase.js)."""
    parts = [f"case={case}", f"view={view}"]
    if theme:
        parts.append(f"theme={theme}")
    if step:
        parts.append(f"step={step}")
    return "?" + "&".join(parts)


def viewer_job(name: str, view: str, theme: str | None, scale: int) -> dict:
    return {
        "name": name,
        "query": query(CODE_CASE, view, theme),
        "selector": "#app",
        "width": VIEWER_WIDTH,
        "height": VIEWER_HEIGHT,
        "scale": scale,
    }


def content_jobs(case: dict) -> tuple[list[dict], list[int]]:
    """A content example's screens in order, and how long the GIF shows each."""
    name = case["name"]
    screens = [("diff", 0, DIFF_LINE_MS)]
    steps = len(case["steps"])
    for index in range(steps):
        screens.append(("omnidiff", index, LAST_STEP_MS if index == steps - 1 else STEP_MS))
    screens.append(("omnidiff", steps, REPORT_MS))
    jobs = [
        {
            "name": f"{name}-{number:02}",
            "query": query(name, view, LIGHT_THEME, step),
            "terminal": TERMINAL,
            "width": 1100,
            "scale": 1,
        }
        for number, (view, step, _) in enumerate(screens)
    ]
    return jobs, [duration for _, _, duration in screens]


def label_font() -> ImageFont.FreeTypeFont:
    """matplotlib's bundled DejaVu Sans Mono, the face the container's browser draws the page in."""
    path = pathlib.Path(matplotlib.get_data_path()) / "fonts" / "ttf" / "DejaVuSansMono-Bold.ttf"
    return ImageFont.truetype(str(path), LABEL_FONT_SIZE)


def labelled(shot: Image.Image, label: str, colour: str, background: str) -> Image.Image:
    """`shot` under a band naming the tool at both ends, on the page's own background."""
    image = Image.new("RGB", (shot.width, shot.height + LABEL_H), background)
    image.paste(shot, (0, LABEL_H))
    draw = ImageDraw.Draw(image)
    font = label_font()
    top = (LABEL_H - LABEL_FONT_SIZE) // 2
    draw.text((LABEL_PAD, top), label, font=font, fill=colour)
    draw.text(
        (image.width - LABEL_PAD - round(font.getlength(label)), top), label, font=font, fill=colour
    )
    return image


def wipe(left: Image.Image, right: Image.Image) -> tuple[list[Image.Image], list[int]]:
    """A bar sweeping right then left; `left` is revealed behind it, `right` is what it covers."""
    width, height = left.size
    out: list[Image.Image] = []
    durations: list[int] = []
    positions = [round(width * i / SWEEP_FRAMES) for i in range(SWEEP_FRAMES + 1)]
    for sweep in (positions, list(reversed(positions))):
        for index, x in enumerate(sweep):
            frame = right.copy()
            if x > 0:
                frame.paste(left.crop((0, 0, x, height)), (0, 0))
            if 0 < x < width:
                ImageDraw.Draw(frame).rectangle([x - BAR_W, 0, x + BAR_W, height], fill=BAR_COLOR)
            out.append(frame)
            # Hold at each end so a reader can read the state before it is swept away.
            durations.append(HOLD_MS if index in (0, len(sweep) - 1) else FRAME_MS)
    return out, durations


def save_gif(
    target: pathlib.Path, images: list[Image.Image], durations: list[int], colours: int
) -> None:
    montage = Image.new("RGB", (images[0].width, images[0].height * len(images)))
    for index, image in enumerate(images):
        montage.paste(image, (0, index * image.height))
    palette = montage.quantize(colors=colours, method=Image.Quantize.FASTOCTREE)
    quantized = [image.quantize(palette=palette, dither=Image.Dither.NONE) for image in images]
    target.parent.mkdir(parents=True, exist_ok=True)
    quantized[0].save(
        target,
        save_all=True,
        append_images=quantized[1:],
        duration=durations,
        loop=0,
        optimize=True,
    )
    size = target.stat().st_size / 1024
    width, height = images[0].size
    print(f"{target}: {len(images)} frames, {width}x{height}, {size:.0f} KiB")


def default_colours(site: pathlib.Path) -> tuple[dict, str]:
    """The default overlay theme's palette and the page's background under it, as baked."""
    state = json.loads((site / "state.json").read_text())
    themes = json.loads((site / "themes.json").read_text())
    theme = state["settings"]["theme"]
    palette = next(entry["palette"] for entry in state["themes"] if entry["id"] == theme)
    syntax = state["settings"]["syntax_theme"] or state["default_syntax_theme"]
    background = next(entry["background"] for entry in themes["syntax"] if entry["theme"] == syntax)
    return palette, background


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--only",
        choices=["screenshot", "diff-gif", "content-gif"],
        action="append",
        help="which pictures to take (default: all)",
    )
    parser.add_argument(
        "--showcase",
        type=pathlib.Path,
        help="a generate_showcase output directory to take them from (default: bake one)",
    )
    parser.add_argument(
        "--out", type=pathlib.Path, default=ROOT / "assets", help="where assets/ is"
    )
    args = parser.parse_args()
    wanted = set(args.only or ["screenshot", "diff-gif", "content-gif"])

    with tempfile.TemporaryDirectory() as tmp:
        work = pathlib.Path(tmp)
        site = args.showcase.resolve() if args.showcase else work / "site"
        if not args.showcase:
            bake(site)
        cases = {case["name"]: case for case in json.loads((site / "cases.json").read_text())}

        jobs: list[dict] = []
        if "screenshot" in wanted:
            jobs.append(viewer_job("screenshot", "omnidiff", LIGHT_THEME, 2))
        if "diff-gif" in wanted:
            jobs.append(viewer_job("wipe-diff", "diff", None, 1))
            jobs.append(viewer_job("wipe-omnidiff", "omnidiff", None, 1))
        content: dict[str, tuple[list[dict], list[int]]] = {}
        if "content-gif" in wanted:
            for name in CONTENT_CASES:
                if name not in cases:
                    sys.exit(f"showcase_shots: the showcase has no case named {name}")
                content[name] = content_jobs(cases[name])
                jobs += content[name][0]

        frames = work / "frames"
        frames.mkdir()
        record(site, jobs, frames)
        shot = lambda name: Image.open(frames / f"{name}.png").convert("RGB")  # noqa: E731

        if "screenshot" in wanted:
            target = args.out / "readme-screenshot.png"
            shot("screenshot").save(target, optimize=True)
            print(f"{target}: {target.stat().st_size / 1024:.0f} KiB")
        if "diff-gif" in wanted:
            palette, background = default_colours(site)
            images, durations = wipe(
                labelled(shot("wipe-omnidiff"), "OmniDiff", palette["insert_bg"], background),
                labelled(shot("wipe-diff"), "Unix diff", palette["delete_bg"], background),
            )
            save_gif(args.out / "diff-vs-omnidiff.gif", images, durations, DIFF_PALETTE_COLORS)
        for name, (case_jobs, durations) in content.items():
            images = [shot(job["name"]) for job in case_jobs]
            save_gif(
                args.out / "content" / f"{name}.gif", images, durations, CONTENT_PALETTE_COLORS
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
