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

"""Records the README's screencast from the showcase page itself, in a headless browser.

    make readme-pictures            # assets/omnidiff-tour.gif

The showcase (`generate_showcase`) is the product replayed in a browser: code in the viewer's
port, other content as the TUI's own widgets drew it. The screencast is screenshots of that page,
so it cannot drift from it, and through it from the product. Nothing here decides what is
painted. This file only says which addresses of the page to take, in which order, and what to
write over each.

The screencast is one GIF of what OmniDiff does with each kind of file, a caption over every
screen: a code change as Unix `diff` marks it, wiped over by OmniDiff's reading, then a PDF, a
font, a picture and an archive in its viewer, and the plain-text report git and scripts get. The
two code screens are the same two files in the same viewer and only the painting differs, so the
code does not move under the wipe's bar and the reader's eye stays on the line it was on. Every
screen is taken at the size of the terminal the content viewer was drawn in, so the frames are
one size.

The browser is Playwright's Chromium in a podman container (scripts/showcase_shots/Containerfile),
driven by record.js, so the screencast does not depend on the browser or the fonts of the machine
that records it. The frames come back as PNGs and are assembled here with Pillow, which is why
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

# The code change the screencast opens on.
CODE_CASE = "python-refactoring"
DIFF_CAPTION = "Code, as Unix diff marks it: whole lines"
OMNIDIFF_CAPTION = "Code, as OmniDiff maps it: only what changed"
# The screens after the code change, in order: a case, the step of its OmniDiff view (the report
# is the step after the last), the caption over it, and how long it shows, in milliseconds.
PDF = "pdf-x-governikus-ausweisapp-522d8b0b-communicationmodel_en"
CONTENT = [
    (PDF, 1, "A PDF: page 1 before and after, the renamed box outlined", 3000),
    (PDF, 2, "A PDF: the changed area alone", 2200),
    (
        "woff2-x-fortawesome-font-awesome-b476ed9a-fa-solid-900",
        2,
        "A web font: every changed glyph, each change outlined",
        3000,
    ),
    (
        "bmp-x-talamus-solarize-12x29-psf-8a856fdb-solarize-12x29",
        0,
        "A picture: before and after, the changed region outlined",
        2600,
    ),
    (
        "zip-x-silnrsi-teckit-3d06b4f4-teckit_tools",
        1,
        "An archive: a picture inside an OpenDocument file",
        2600,
    ),
    (PDF, 3, "For git, scripts and pipes: the PDF's change as a plain-text report", 3200),
]

# The terminal the generator drew the content viewer in (`CONTENT_TERMINAL` in
# src/bin/generate_showcase.rs), in cells.
TERMINAL = {"cols": 120, "rows": 32}
# That terminal as the recorder takes it, in pixels: what TERMINAL's cells come to in the image's
# 13px DejaVu Sans Mono with the page's padding. The code viewer is taken at this size too, and
# `main` checks that the two agree.
SCREEN = (968, 592)

# The band above every screen carrying its caption.
LABEL_H = 34
LABEL_PAD = 12
LABEL_FONT_SIZE = 15
# The wipe from `diff`'s reading to OmniDiff's: a bar sweeping across once, in this many frames,
# with each reading held before and after. The sweep is the dominant term in the file's size.
BAR_W = 3
BAR_COLOR = "#00cdcd"
SWEEP_FRAMES = 22
# GIF stores delays in hundredths of a second, so anything not a multiple of 10 is floored on the
# way out.
FRAME_MS = 40
HOLD_MS = 2600
# Every frame is quantised against one shared palette, drawn from all of them: the frames compress
# as deltas, and a colour that is in one frame only (an outline, a highlight) does not shimmer in
# the others.
PALETTE_COLORS = 255


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


def query(case: str, view: str, step: int = 0) -> str:
    """The showcase address of one screen (`buildQuery` in assets/showcase/showcase.js)."""
    parts = [f"case={case}", f"view={view}"]
    if step:
        parts.append(f"step={step}")
    return "?" + "&".join(parts)


def jobs() -> list[dict]:
    """The screens to take: the code change twice, then CONTENT, all in the page's default theme."""
    out = [
        {
            "name": name,
            "query": query(CODE_CASE, view),
            "selector": "#app",
            "width": SCREEN[0],
            "fit": SCREEN[1],
            "scale": 1,
        }
        for name, view in (("code-diff", "diff"), ("code-omnidiff", "omnidiff"))
    ]
    for index, (case, step, _, _) in enumerate(CONTENT):
        out.append(
            {
                "name": f"content-{index:02}",
                "query": query(case, "omnidiff", step),
                "terminal": TERMINAL,
                "width": 1100,
                "scale": 1,
            }
        )
    return out


def captioned(shot: Image.Image, caption: str, colour: str, background: str) -> Image.Image:
    """`shot` under a band carrying `caption`."""
    image = Image.new("RGB", (shot.width, shot.height + LABEL_H), background)
    image.paste(shot, (0, LABEL_H))
    top = (LABEL_H - LABEL_FONT_SIZE) // 2
    ImageDraw.Draw(image).text((LABEL_PAD, top), caption, font=label_font(), fill=colour)
    return image


def label_font() -> ImageFont.FreeTypeFont:
    """matplotlib's bundled DejaVu Sans Mono, the face the container's browser draws the page in."""
    path = pathlib.Path(matplotlib.get_data_path()) / "fonts" / "ttf" / "DejaVuSansMono-Bold.ttf"
    return ImageFont.truetype(str(path), LABEL_FONT_SIZE)


def wipe(left: Image.Image, right: Image.Image) -> tuple[list[Image.Image], list[int]]:
    """A bar sweeping right; `left` is revealed behind it, `right` is what it covers."""
    width, height = left.size
    out: list[Image.Image] = []
    durations: list[int] = []
    for index in range(SWEEP_FRAMES + 1):
        x = round(width * index / SWEEP_FRAMES)
        frame = right.copy()
        if x > 0:
            frame.paste(left.crop((0, 0, x, height)), (0, 0))
        if 0 < x < width:
            ImageDraw.Draw(frame).rectangle([x - BAR_W, 0, x + BAR_W, height], fill=BAR_COLOR)
        out.append(frame)
        # Hold at each end so a reader can read the state before it is swept away.
        durations.append(HOLD_MS if index in (0, SWEEP_FRAMES) else FRAME_MS)
    return out, durations


def save_gif(target: pathlib.Path, images: list[Image.Image], durations: list[int]) -> None:
    montage = Image.new("RGB", (images[0].width, images[0].height * len(images)))
    for index, image in enumerate(images):
        montage.paste(image, (0, index * image.height))
    palette = montage.quantize(colors=PALETTE_COLORS, method=Image.Quantize.FASTOCTREE)
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
        "--showcase",
        type=pathlib.Path,
        help="a generate_showcase output directory to record from (default: bake one)",
    )
    parser.add_argument(
        "--out",
        type=pathlib.Path,
        default=ROOT / "assets" / "omnidiff-tour.gif",
        help="the GIF to write",
    )
    args = parser.parse_args()

    with tempfile.TemporaryDirectory() as tmp:
        work = pathlib.Path(tmp)
        site = args.showcase.resolve() if args.showcase else work / "site"
        if not args.showcase:
            bake(site)
        frames = work / "frames"
        frames.mkdir()
        record(site, jobs(), frames)

        def shot(name: str) -> Image.Image:
            return Image.open(frames / f"{name}.png").convert("RGB")

        palette, background = default_colours(site)
        foreground = palette["overlay_fg"]
        screens = [shot(f"content-{index:02}") for index in range(len(CONTENT))]
        size = screens[0].size
        code = {}
        for name in ("code-diff", "code-omnidiff"):
            # The viewer's box can come out a pixel taller than asked: the strip above it ends
            # on a fraction of a pixel. More than that is a real mismatch.
            taken = shot(name)
            if abs(taken.width - size[0]) > 2 or abs(taken.height - size[1]) > 2:
                sys.exit(
                    f"showcase_shots: {name} is {taken.size}, the content screens {size}: "
                    "SCREEN no longer matches the terminal"
                )
            code[name] = taken.crop((0, 0, *size))
        images, durations = wipe(
            captioned(code["code-omnidiff"], OMNIDIFF_CAPTION, foreground, background),
            captioned(code["code-diff"], DIFF_CAPTION, foreground, background),
        )
        for screen, (_, _, caption, duration) in zip(screens, CONTENT, strict=True):
            images.append(captioned(screen, caption, foreground, background))
            durations.append(duration)
        save_gif(args.out, images, durations)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
