#!/usr/bin/env python3
#
#  This file is part of the OmniDiff code diffing tool.
#
#  Copyright (C) 2026 Marko Ivankovic
#
#  This program is free software: you can redistribute it and/or modify
#  it under the terms of the GNU Affero General Public License as published
#  by the Free Software Foundation, either version 3 of the License, or
#  (at your option) any later version.
#
#  This program is distributed in the hope that it will be useful,
#  but WITHOUT ANY WARRANTY; without even the implied warranty of
#  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
#  GNU Affero General Public License for more details.
#
#  You should have received a copy of the GNU Affero General Public License
#  along with this program. If not, see <https://www.gnu.org/licenses/>.

"""Record one GIF per content example: what `git diff` says about the file, what OmniDiff says,
then OmniDiff's viewer stepping through its views.

    make content-gif

Nothing here decides what is painted. The terminal text is git's own line and `omnidiff
--headless`'s real output; the viewer frames are drawn by the TUI's own widgets
(`render_content_stills`, see `tui::screenshot::render_content`) and only rasterized here, with
the pictures the viewer recorded pasted into their cells, scaled to fit as a kitty terminal
scales them. The examples are fixtures under `src/test/data/<family>/`, so the GIFs can be
regenerated from the repository alone.

The GIFs land in `assets/content/`: the README shows some of them and the showcase all of them
(`generate_showcase` copies them, with `assets/content/cases.json`, which this writes too).
"""

from __future__ import annotations

import argparse
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field

from PIL import Image

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import render_tui_screenshot as still  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parent.parent
COLS, ROWS = 120, 32
# The prompt's colour on the terminal frames: Solarized blue.
PROMPT = "#268bd2"


@dataclass
class Case:
    """One example: a fixture, what it shows, and the viewer's keys, one still per step."""

    family: str
    fixture: str
    title: str
    # Each step: the keys pressed before a still, and how long it shows in milliseconds.
    steps: list[tuple[str, int]] = field(default_factory=list)


def picture_steps(swipe: bool = True) -> list[tuple[str, int]]:
    """Side by side, difference, blend, then a swipe across and back."""
    steps = [("", 2600), ("t", 1800), ("t", 1200)]
    if swipe:
        steps.append(("t h h h h h h h h h h", 120))
        steps += [("l", 90)] * 20
        steps += [("h", 90)] * 10
        steps[-1] = ("h", 1200)
    return steps


CASES = [
    Case(
        "documents",
        "pdf-x-governikus-ausweisapp-522d8b0b-communicationmodel_en",
        "A PDF diagram: one box renamed",
        [("j", 2200), ("j", 3400), ("t", 2200)],
    ),
    Case(
        "fonts",
        "woff2-x-fortawesome-font-awesome-b476ed9a-fa-solid-900",
        "A web font: twenty icons redrawn",
        [("j", 2600), ("j", 2200), ("g", 4200)],
    ),
    Case(
        "archives",
        "zip-x-silnrsi-teckit-3d06b4f4-teckit_tools",
        "An OpenDocument file: its text and its thumbnail",
        [("j j j j j", 3400), ("k k", 3000)],
    ),
    Case(
        "catalogs",
        "mo-x-ubernostrum-django-registration-906912aa-django",
        "A compiled translation: three messages reworded",
        [("j", 2400), ("j", 2400), ("j", 2400)],
    ),
    Case(
        "pictures",
        "bmp-x-talamus-solarize-12x29-psf-8a856fdb-solarize-12x29",
        "A bitmap font sheet: one glyph redrawn",
        picture_steps(),
    ),
    Case(
        "pictures",
        "ico-x-golang-tour-16d3ff4a-favicon",
        "A favicon re-exported at a larger size",
        picture_steps(),
    ),
    Case(
        "pictures",
        "gif-x-ascii-boxes-boxes-eea1dfea-readme-0",
        "An animated demo with frames added",
        [("", 1800)] + [(".", 700)] * 8 + [("t", 1600)],
    ),
]


def readme_path(directory: pathlib.Path) -> str:
    """The file's path in its repository, from the fixture's README."""
    for line in (directory / "README.md").read_text().splitlines():
        if line.startswith("- **File:** `"):
            return line.removeprefix("- **File:** `").removesuffix("`")
    raise SystemExit(f"{directory}/README.md names no file")


def pair(directory: pathlib.Path) -> tuple[pathlib.Path, pathlib.Path]:
    before = next(directory.glob("before.*.test"))
    after = next(directory.glob("after.*.test"))
    return before, after


def text_still(lines: list[list[tuple[str, str | None]]]) -> dict:
    """A terminal frame of plain lines, each a list of (text, colour) runs."""
    rows = []
    for runs in lines[:ROWS]:
        rows.append(
            [
                {
                    "text": text,
                    "fg": colour,
                    "bg": None,
                    "bold": colour is not None,
                    "dim": False,
                    "italic": False,
                    "underline": False,
                }
                for text, colour in runs
            ]
        )
    return {"cols": COLS, "rows": ROWS, "lines": rows}


def wrap(text: str) -> list[str]:
    out = []
    for line in text.splitlines():
        while len(line) > COLS:
            out.append(line[:COLS])
            line = "  " + line[COLS:]
        out.append(line)
    return out


def terminal_frames(name: str, report: str) -> list[tuple[dict, int]]:
    """`git diff` alone, then with OmniDiff as its external diff."""
    git = [
        [("$ ", PROMPT), ("git diff", None)],
        [(f"diff --git a/{name} b/{name}", None)],
        [(f"Binary files a/{name} and b/{name} differ", None)],
    ]
    omnidiff = git + [
        [("", None)],
        [("$ ", PROMPT), ("GIT_EXTERNAL_DIFF=omnidiff git diff", None)],
    ]
    omnidiff += [[(line, None)] for line in wrap(report)]
    return [(text_still(git), 2200), (text_still(omnidiff), 3800)]


def rasterize(still_json: pathlib.Path) -> Image.Image:
    """A viewer still, its pictures pasted into their cells."""
    data = json.loads(still_json.read_text())
    image = still.render(data, still.DEFAULT_BG, still.DEFAULT_FG)
    cell_w, row_h, _ = still.metrics()
    for picture in data["pictures"]:
        pixels = Image.open(still_json.parent / picture["file"]).convert("RGB")
        width, height = picture["cols"] * cell_w, picture["rows"] * row_h
        ratio = min(width / pixels.width, height / pixels.height)
        size = (max(1, round(pixels.width * ratio)), max(1, round(pixels.height * ratio)))
        resample = Image.Resampling.NEAREST if ratio >= 1 else Image.Resampling.LANCZOS
        x = still.PAD + picture["col"] * cell_w
        y = still.PAD + picture["row"] * row_h
        image.paste(pixels.resize(size, resample), (x, y))
    return image


def record(case: Case, omnidiff: pathlib.Path, stills_bin: pathlib.Path, out: pathlib.Path) -> dict:
    directory = ROOT / "src" / "test" / "data" / case.family / case.fixture
    path = readme_path(directory)
    name = pathlib.PurePosixPath(path).name
    before, after = pair(directory)
    with tempfile.TemporaryDirectory() as tmp:
        work = pathlib.Path(tmp)
        (work / "a").mkdir()
        (work / "b").mkdir()
        shutil.copy(before, work / "a" / name)
        shutil.copy(after, work / "b" / name)
        report = subprocess.run(
            [omnidiff, "--headless", "--color", "never", f"a/{name}", f"b/{name}"],
            cwd=work,
            capture_output=True,
            text=True,
            check=True,
        ).stdout.rstrip()
        cell_w, row_h, _ = still.metrics()
        args = [
            stills_bin,
            f"a/{name}",
            f"b/{name}",
            "--cols",
            str(COLS),
            "--rows",
            str(ROWS),
            "--font",
            f"{cell_w}x{row_h}",
            "--out",
            "stills",
        ]
        for keys, _ in case.steps:
            args += ["--step", keys]
        subprocess.run(args, cwd=work, check=True)

        frames = []
        durations = []
        for frame, duration in terminal_frames(name, report):
            frames.append(still.render(frame, still.DEFAULT_BG, still.DEFAULT_FG))
            durations.append(duration)
        for index, (_, duration) in enumerate(case.steps):
            frames.append(rasterize(work / "stills" / f"still-{index:02}.json"))
            durations.append(duration)

    target = out / f"{case.fixture}.gif"
    # One palette for every frame, so the pictures' colours do not shimmer between frames, drawn
    # from all of them: an outline or a highlight may be in one frame only.
    montage = Image.new("RGB", (frames[0].width, frames[0].height * len(frames)))
    for index, frame in enumerate(frames):
        montage.paste(frame, (0, index * frame.height))
    palette = montage.quantize(colors=255, method=Image.Quantize.FASTOCTREE)
    quantized = [frame.quantize(palette=palette, dither=Image.Dither.NONE) for frame in frames]
    quantized[0].save(
        target,
        save_all=True,
        append_images=quantized[1:],
        duration=durations,
        loop=0,
        optimize=True,
    )
    print(f"wrote {target} ({frames[0].width}x{frames[0].height}, {len(frames)} frames)")
    return {
        "family": case.family,
        "fixture": case.fixture,
        "title": case.title,
        "path": path,
        "gif": target.name,
        "report": report,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--omnidiff", type=pathlib.Path, required=True)
    parser.add_argument("--stills", type=pathlib.Path, required=True, help="render_content_stills")
    parser.add_argument("--out", type=pathlib.Path, default=ROOT / "assets" / "content")
    parser.add_argument("--only", help="record only the case whose fixture contains this")
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    cases = [case for case in CASES if not args.only or args.only in case.fixture]
    index = [
        record(case, args.omnidiff.resolve(), args.stills.resolve(), args.out.resolve())
        for case in cases
    ]
    if not args.only:
        (args.out / "cases.json").write_text(json.dumps(index, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
