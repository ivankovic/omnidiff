#!/usr/bin/env python3
#
#  This file is part of the OmniDiff code diffing tool.
#
#  Copyright (C) 2026 Marko Ivankovic
#
#  This program is free software: you can redistribute it and/or modify
#  it under the terms of the GNU Affero General Public License as published
#  by the Free Software Foundation, version 3 of the License.
#
#  This program is distributed in the hope that it will be useful,
#  but WITHOUT ANY WARRANTY; without even the implied warranty of
#  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
#  GNU Affero General Public License for more details.
#
#  You should have received a copy of the GNU Affero General Public License
#  along with this program. If not, see <https://www.gnu.org/licenses/>.

"""Drives the real lazygit in a podman container, with omnidiff and others as its diff renderer.

lazygit (0.66+) shows a file's diff through a configured renderer and, when the user focuses the
main view to stage single lines, needs to know which line of the patch each rendered row is. A
renderer that says so over the OSC 1717 protocol keeps its rendering while lines are staged;
one that does not makes lazygit swap in git's raw diff instead. This harness checks both, for
each renderer, by typing into lazygit as a user would.

On the host it builds the image (scripts/lazygit_e2e/Containerfile: git, tmux, lazygit, delta and
a Rust toolchain), starts a container with the omnidiff checkout mounted read-only, builds
omnidiff in it (into a named volume per checkout, so reruns are incremental; a binary built on
the host may need a newer glibc than the image has), then reruns this script inside with
`--inside`. Inside, each scenario gets a fresh repository - a commit and an unstaged change to
two lines - and a lazygit config selecting one renderer, and runs lazygit in its own detached
tmux server at a fixed size. Screens are read with `tmux capture-pane`, polled until they stop
changing, never slept on; each step's screen is saved under the artifacts directory.

The keys, after lazygit settles on the Files panel with the changed file selected (it is the only
one): Enter focuses that file's diff in the main view ("Focus file diff" in 0.66's Files
keybindings), in line mode because the config sets `gui.useHunkModeInDiffView: false`; Down moves
the selection one row until the selected row shows the target line - the selected row is found
by the background colour the config gives it, magenta, which no renderer here uses; space stages
the selection; q quits. The check is `git diff --cached`: exactly the target line, nothing else.

Scenarios are SCENARIOS below: each renderer gets `render` (its diff of the file is on screen
within a timeout - an extDiff renderer that opens a TUI under lazygit's pty never gets there),
`stage-add` and `stage-del` (one added, one deleted line staged from the focused view), and
every renderer but git's own also `osc1717` (focusing the view keeps the renderer's rendering,
rather than falling back to git's raw diff).

Usage:  python3 scripts/lazygit_e2e/run.py [--omnidiff-src PATH] [--scenario NAME]... [--keep]
        python3 scripts/lazygit_e2e/run.py --list
"""

from __future__ import annotations

import argparse
import fnmatch
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parent.parent
IMAGE = "omnidiff-lazygit-e2e"
CARGO_VOLUME = "omnidiff-lazygit-e2e-cargo"
# Where things are mounted in the container.
SRC, HARNESS, ARTIFACTS, TARGET = "/src", "/harness", "/artifacts", "/target"
OMNIDIFF_BIN = f"{TARGET}/ci/omnidiff"

COLS, ROWS = 200, 50
# A screen counts as settled once it has not changed for this long.
SETTLE_SECONDS = 0.4
POLL_SECONDS = 0.05
# The SGR background code of the selected row: `gui.theme.selectedLineBgColor: [magenta]`.
SELECTION_BG = "45"

BEFORE = [
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel",
    "india", "juliet", "kilo", "lima", "mike", "november", "oscar",
]  # fmt: skip
CHANGES = {"charlie": "CHARLIE-changed", "lima": "LIMA-changed"}
FILE = "notes.txt"

# A moved function: omnidiff matches `def total(numbers):` across the move and paints neither copy,
# while git's patch deletes it at old line 1 and adds it at new line 5. Staging either copy from
# omnidiff's view must stage git's line - what OSC 1717's patch-space types are for.
MOVED = (
    "move.py",
    (
        "def total(numbers):\n    s = 0\n    for n in numbers:\n        s += n\n    return s\n\n\n"
        "def mean(numbers):\n    return total(numbers) / len(numbers)\n"
    ),
    "def mean(numbers):\n    return sum(numbers) / len(numbers)\n\n\ndef total(numbers):\n    return sum(numbers)\n",
)


@dataclass(frozen=True)
class Renderer:
    name: str
    # lazygit's `git.diffRenderers`; empty is git's own diff.
    config: tuple[dict, ...] = ()

    @property
    def is_raw_git(self) -> bool:
        return not self.config


RENDERERS = [
    Renderer("rawgit"),
    Renderer("delta", ({"command": "delta --dark --paging=never"},)),
    Renderer("omnidiff", ({"type": "extDiff", "command": "omnidiff --context {{diffContext}}"},)),
    Renderer(
        "omnidiff-headless",
        ({"type": "extDiff", "command": "omnidiff --headless --context {{diffContext}}"},),
    ),
]


@dataclass(frozen=True)
class Scenario:
    name: str
    renderer: Renderer
    # "render": the diff shows `target` within the timeout. "stage": focus the diff, select the
    # row showing `target`, stage it, and expect `staged` (the `+`/`-` lines of
    # `git diff --cached -U0`). "osc1717": focus the diff and expect no fallback to git's raw diff.
    check: str
    target: str = ""
    staged: tuple[str, ...] = ()
    # Select the n-th row showing `target`, counting from the top of the diff.
    occurrence: int = 1
    # (file name, committed contents, working-tree contents); default the notes.txt pair.
    files: tuple[str, str, str] | None = None


def scenarios() -> list[Scenario]:
    out = []
    for r in RENDERERS:
        out.append(Scenario(f"{r.name}-render", r, "render", target="LIMA-changed"))
        out.append(
            Scenario(f"{r.name}-stage-add", r, "stage", "LIMA-changed", staged=("+LIMA-changed",))
        )
        # Case matters: "charlie" is only on the deleted row, never on "CHARLIE-changed".
        out.append(Scenario(f"{r.name}-stage-del", r, "stage", "charlie", staged=("-charlie",)))
        if not r.is_raw_git:
            out.append(Scenario(f"{r.name}-osc1717", r, "osc1717", target="LIMA-changed"))
        # The first row naming `def total` is the old copy, the second the new one, in git's
        # rendering and omnidiff's alike.
        moved = "def total(numbers):"
        out.append(
            Scenario(f"{r.name}-stage-moved-del", r, "stage", moved, ("-" + moved,), 1, files=MOVED)
        )
        out.append(
            Scenario(f"{r.name}-stage-moved-add", r, "stage", moved, ("+" + moved,), 2, files=MOVED)
        )
    return out


SCENARIOS = scenarios()


class Failure(Exception):
    pass


# ---------------------------------------------------------------------------------------------
# Inside the container: one tmux server per scenario.
# ---------------------------------------------------------------------------------------------

TOKEN = re.compile(r"\x1b\[([0-9;:]*)([A-Za-z])|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b.|(.)", re.S)


def cells(line: str) -> list[tuple[str, bool]]:
    """One row of `capture-pane -e` as (character, has the selection background) per cell."""
    out, bg = [], None
    for m in TOKEN.finditer(line):
        if m.group(3) is not None:
            out.append((m.group(3), bg == SELECTION_BG))
            continue
        if m.group(2) != "m":
            continue
        params = (m.group(1) or "0").replace(":", ";").split(";")
        i = 0
        while i < len(params):
            p = params[i] or "0"
            if p in ("38", "48"):
                skip = 2 if params[i + 1 : i + 2] == ["5"] else 4
                if p == "48":
                    bg = "extended"
                i += skip
            elif p in ("0", "49"):
                bg = None
            elif (p.isdigit() and 40 <= int(p) <= 47) or (p.isdigit() and 100 <= int(p) <= 107):
                bg = p
            i += 1
    return out


@dataclass
class Screen:
    ansi: str
    rows: list[list[tuple[str, bool]]] = field(init=False)

    def __post_init__(self):
        self.rows = [cells(line) for line in self.ansi.split("\n")]

    def text(self, row: list[tuple[str, bool]], start: int = 0) -> str:
        return "".join(c for c, _ in row[start:]).rstrip()

    @property
    def plain(self) -> str:
        return "\n".join(self.text(r) for r in self.rows)

    def main_column(self) -> int:
        """Where the main view starts: its box corner, on the row holding its `[0]` title."""
        for row in self.rows:
            text = self.text(row)
            at = text.find("[0]")
            if at >= 0:
                return max(text.rfind("╭", 0, at), 0)
        return 0

    def main_rows(self) -> list[str]:
        col = self.main_column()
        return [self.text(r, col) for r in self.rows]

    def main_text(self) -> str:
        return "\n".join(self.main_rows())

    def selected(self) -> list[str]:
        """The main view's rows that carry the selection background."""
        col = self.main_column()
        return [self.text(r, col) for r in self.rows if any(sel for _, sel in r[col:])]


class Lazygit:
    def __init__(self, scenario: Scenario, root: Path, artifacts: Path, keep: bool):
        self.scenario, self.root, self.artifacts, self.keep = scenario, root, artifacts, keep
        self.socket = f"lg-{scenario.name}"
        self.step = 0

    def tmux(self, *args: str, check: bool = True) -> subprocess.CompletedProcess:
        return subprocess.run(
            ["tmux", "-L", self.socket, *args], capture_output=True, text=True, check=check
        )

    def start(self, repo: Path, config: Path) -> None:
        env = {
            "LG_CONFIG_FILE": str(config),
            "HOME": str(self.root / "home"),
            "XDG_STATE_HOME": str(self.root / "state"),
            "PATH": f"{Path(OMNIDIFF_BIN).parent}:{os.environ['PATH']}",
        }
        command = " ".join(f"{k}={v}" for k, v in env.items()) + " exec lazygit"
        self.tmux("-f", "/dev/null", "new-session", "-d", "-s", "lg", "-x", str(COLS), "-y",
                  str(ROWS), "-c", str(repo), command)  # fmt: skip

    def capture(self) -> Screen:
        result = self.tmux("capture-pane", "-p", "-e", "-t", "lg", check=False)
        if result.returncode != 0:
            raise Failure(f"lazygit is gone (its tmux session ended): {result.stderr.strip()}")
        return Screen(result.stdout)

    def save(self, label: str, screen: Screen | None = None) -> Screen:
        screen = screen or self.capture()
        self.step += 1
        stem = self.artifacts / f"{self.step:02d}-{label}"
        stem.with_suffix(".ansi").write_text(screen.ansi)
        stem.with_suffix(".txt").write_text(screen.plain + "\n")
        return screen

    def settle(self, until=lambda s: True, timeout: float = 10.0, what: str = "") -> Screen:
        """The screen once `until` holds and it has stopped changing; Failure after `timeout`."""
        deadline = time.monotonic() + timeout
        last, since = None, time.monotonic()
        while True:
            screen = self.capture()
            now = time.monotonic()
            if last is None or screen.ansi != last.ansi:
                last, since = screen, now
            elif until(screen) and now - since >= SETTLE_SECONDS:
                return screen
            if now > deadline:
                self.save("timeout", last)
                raise Failure(f"after {timeout:.0f}s: {what or 'screen did not settle'}")
            time.sleep(POLL_SECONDS)

    def changed_from(self, before: Screen, timeout: float = 2.0) -> Screen:
        """The settled screen after a key, or the same screen if the key changed nothing."""
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if self.capture().ansi != before.ansi:
                return self.settle()
            time.sleep(POLL_SECONDS)
        return before

    def keys(self, *keys: str) -> None:
        self.tmux("send-keys", "-t", "lg", *keys)

    def quit(self) -> None:
        self.keys("q")
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if self.tmux("has-session", "-t", "lg", check=False).returncode != 0:
                return
            time.sleep(POLL_SECONDS)
        raise Failure("lazygit did not quit on q within 5s")

    def stop(self) -> None:
        if not self.keep:
            self.tmux("kill-server", check=False)


def git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True, text=True, check=True
    ).stdout


def make_repo(root: Path, scenario: Scenario) -> tuple[Path, Path]:
    repo = root / "repo"
    repo.mkdir(parents=True)
    git(repo, "init", "-q", "-b", "main")
    git(repo, "config", "user.name", "e2e")
    git(repo, "config", "user.email", "e2e@example.invalid")
    name, committed, changed = scenario.files or (
        FILE,
        "\n".join(BEFORE) + "\n",
        "\n".join(CHANGES.get(w, w) for w in BEFORE) + "\n",
    )
    (repo / name).write_text(committed)
    git(repo, "add", name)
    git(repo, "commit", "-q", "-m", "init")
    (repo / name).write_text(changed)
    config = {
        "gui": {
            "showRandomTip": False,
            "showCommandLog": False,
            "useHunkModeInDiffView": False,
            # Asking the terminal costs a round trip tmux may not answer.
            "colorScheme": "dark",
            "theme": {"selectedLineBgColor": ["magenta"]},
        },
        "disableStartupPopups": True,
        "update": {"method": "never"},
        "git": {"diffRenderers": list(scenario.renderer.config)},
    }
    path = root / "config.yml"
    # JSON is YAML.
    path.write_text(json.dumps(config, indent=2) + "\n")
    for sub in ("home", "state"):
        (root / sub).mkdir()
    return repo, path


def raw_fallback(screen: Screen) -> bool:
    """Whether the main view shows git's own diff (its `diff --git` header) - what lazygit swaps
    in for a renderer that cannot say which line a row is."""
    return any(re.match(r".?.?diff --git a/", row) for row in screen.main_rows())


def run_scenario(s: Scenario, timeout: float, artifacts: Path, keep: bool) -> str:
    """Runs one scenario; returns a note on what was seen, raises Failure when it fails."""
    root = Path("/work") / s.name
    shutil.rmtree(root, ignore_errors=True)
    shutil.rmtree(artifacts, ignore_errors=True)
    artifacts.mkdir(parents=True)
    repo, config = make_repo(root, s)
    lg = Lazygit(s, root, artifacts, keep)
    lg.start(repo, config)
    notes = []
    try:
        started = time.monotonic()
        try:
            screen = lg.settle(
                lambda sc: s.target in sc.main_text(),
                timeout,
                f"the diff pane never showed {s.target!r} (renderer hung?)",
            )
            notes.append(f"rendered in {time.monotonic() - started:.1f}s")
        except Failure as e:
            if s.check == "render":
                raise
            # Acting on lines is a scenario of its own: lazygit may well replace a rendering
            # that never arrived with git's raw diff once the view is focused.
            notes.append(f"not rendered ({e})")
            screen = lg.capture()
        finally:
            lg.save("start")
        if s.check == "render":
            lg.quit()
            return "; ".join(notes)

        unfocused_raw = raw_fallback(screen)
        lg.keys("Enter")
        screen = lg.settle(lambda sc: bool(sc.selected()), timeout, "Enter selected no diff line")
        lg.save("focused", screen)
        fell_back = raw_fallback(screen) and not unfocused_raw
        if not s.renderer.is_raw_git:
            notes.append(
                "focused view: git raw diff (fallback)" if fell_back else "focused view: rendered"
            )
        if s.check == "osc1717":
            if fell_back:
                raise Failure("focusing the diff fell back to git's raw diff: no OSC 1717")
            lg.quit()
            return "; ".join(notes)

        presses = 0
        seen = 1 if any(s.target in row for row in screen.selected()) else 0
        while seen < s.occurrence:
            if presses >= 60:
                raise Failure(f"no row showing {s.target!r} after {presses} Down presses")
            before = screen
            lg.keys("Down")
            presses += 1
            screen = lg.changed_from(before)
            if screen is before:
                raise Failure(f"Down stopped moving before a row showing {s.target!r}")
            if any(s.target in row for row in screen.selected()):
                seen += 1
        lg.save("selected", screen)
        notes.append(f"{presses} Down")
        lg.keys("Space")
        deadline = time.monotonic() + timeout
        while not git(repo, "diff", "--cached", "--name-only").strip():
            if time.monotonic() > deadline:
                lg.save("not-staged")
                raise Failure("space staged nothing")
            time.sleep(POLL_SECONDS)
        lg.save("staged", lg.settle())
        lg.quit()
        cached = git(repo, "diff", "--cached", "-U0", "--no-color")
        (artifacts / "staged.diff").write_text(cached)
        got = tuple(
            line
            for line in cached.splitlines()
            if line[:1] in "+-" and not line.startswith(("+++", "---"))
        )
        if got != s.staged:
            raise Failure(f"staged {list(got)}, expected {list(s.staged)}")
        return "; ".join(notes)
    except Failure as e:
        raise Failure("; ".join([*notes, str(e)])) from None
    finally:
        lg.stop()
        if not keep:
            # A renderer that hung under lazygit's pty can outlive it.
            subprocess.run(["pkill", "-KILL", "-x", "omnidiff"], check=False)


def inside(args: argparse.Namespace) -> int:
    chosen = select(args.scenario)
    failed = []
    for s in chosen:
        started = time.monotonic()
        try:
            note = run_scenario(s, args.timeout, Path(ARTIFACTS) / s.name, args.keep)
            print(f"PASS  {s.name:28} {time.monotonic() - started:5.1f}s  {note}", flush=True)
        except Failure as e:
            failed.append(s.name)
            print(f"FAIL  {s.name:28} {time.monotonic() - started:5.1f}s  {e}", flush=True)
    print(f"\n{len(chosen) - len(failed)} of {len(chosen)} scenarios passed", flush=True)
    if failed:
        print(f"failed: {' '.join(failed)}", flush=True)
    return 1 if failed else 0


def select(patterns: list[str] | None) -> list[Scenario]:
    if not patterns:
        return SCENARIOS
    chosen = [s for s in SCENARIOS if any(fnmatch.fnmatchcase(s.name, p) for p in patterns)]
    if not chosen:
        sys.exit(f"lazygit_e2e: no scenario matches {patterns}; --list shows them")
    return chosen


# ---------------------------------------------------------------------------------------------
# On the host: image, container, build.
# ---------------------------------------------------------------------------------------------


def podman(*args: str, check: bool, **kwargs) -> subprocess.CompletedProcess:
    return subprocess.run(["podman", *args], check=check, **kwargs)


def timed(label: str, *args: str) -> None:
    started = time.monotonic()
    print(f"lazygit_e2e: {label}", file=sys.stderr, flush=True)
    result = podman(*args, check=False)
    if result.returncode != 0:
        sys.exit(f"lazygit_e2e: {label} failed (exit {result.returncode})")
    print(f"lazygit_e2e: {label}: {time.monotonic() - started:.1f}s", file=sys.stderr, flush=True)


def host(args: argparse.Namespace) -> int:
    if shutil.which("podman") is None:
        sys.exit("lazygit_e2e: needs podman")
    select(args.scenario)  # fail on a bad pattern before building anything
    src = args.omnidiff_src.resolve()
    if not (src / "Cargo.toml").is_file():
        sys.exit(f"lazygit_e2e: {src} is not an omnidiff checkout")
    artifacts = args.artifacts.resolve()
    artifacts.mkdir(parents=True, exist_ok=True)
    # One target volume per checkout, so switching between two keeps both builds incremental.
    target_volume = f"{IMAGE}-target-{hashlib.sha256(str(src).encode()).hexdigest()[:10]}"
    container = f"{IMAGE}-{os.getpid()}"

    timed("building image", "build", "-q", "-t", IMAGE, "-f", str(HERE / "Containerfile"),
          str(HERE))  # fmt: skip
    for volume in (CARGO_VOLUME, target_volume):
        if podman("volume", "exists", volume, check=False).returncode != 0:
            podman("volume", "create", volume, check=True, stdout=subprocess.DEVNULL)
    podman(
        "run", "-d", "--name", container, "--security-opt", "label=disable",
        "-v", f"{src}:{SRC}:ro", "-v", f"{HERE}:{HARNESS}:ro", "-v", f"{artifacts}:{ARTIFACTS}",
        "-v", f"{CARGO_VOLUME}:/cargo", "-v", f"{target_volume}:{TARGET}",
        IMAGE, "sleep", "infinity",
        check=True, stdout=subprocess.DEVNULL,
    )  # fmt: skip
    try:
        build = ["cargo", "build", "--locked", "-q", "--profile", "ci", "--bin", "omnidiff"]
        manifest = ["--manifest-path", f"{SRC}/Cargo.toml"]
        timed(f"building omnidiff from {src}", "exec", container, *build, *manifest)
        version = podman(
            "exec",
            container,
            OMNIDIFF_BIN,
            "--version",
            capture_output=True,
            text=True,
            check=False,
        ).stdout.strip()
        head = subprocess.run(
            ["git", "-C", str(src), "log", "-1", "--format=%h %s"],
            capture_output=True,
            text=True,
            check=False,
        ).stdout.strip()
        print(f"lazygit_e2e: {version} at {head or '?'}; artifacts in {artifacts}", file=sys.stderr)
        inner = ["python3", "-I", f"{HARNESS}/run.py", "--inside", "--timeout", str(args.timeout)]
        for pattern in args.scenario or []:
            inner += ["--scenario", pattern]
        if args.keep:
            inner.append("--keep")
        started = time.monotonic()
        code = podman("exec", container, *inner, check=False).returncode
        print(f"lazygit_e2e: scenarios: {time.monotonic() - started:.1f}s", file=sys.stderr)
        if code not in (0, 1):
            sys.exit(f"lazygit_e2e: the in-container driver failed (exit {code})")
        return code
    finally:
        if args.keep:
            print(
                f"lazygit_e2e: kept container {container}; a scenario's lazygit is at\n"
                f"    podman exec -it {container} tmux -L lg-<scenario> attach\n"
                f"  remove it with: podman rm -f {container}",
                file=sys.stderr,
            )
        else:
            podman("rm", "-f", "-t", "0", container, check=False, stdout=subprocess.DEVNULL)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument(
        "--omnidiff-src",
        type=Path,
        default=REPO_ROOT,
        help="the omnidiff checkout to build and test, mounted read-only (default: this one)",
    )
    parser.add_argument(
        "--scenario",
        action="append",
        metavar="NAME",
        help="run only this scenario (a glob, e.g. 'delta-*'); repeatable",
    )
    parser.add_argument("--keep", action="store_true", help="leave the container and sessions")
    parser.add_argument(
        "--timeout", type=float, default=10.0, help="seconds to wait for each screen (default 10)"
    )
    parser.add_argument(
        "--artifacts",
        type=Path,
        default=REPO_ROOT / "target" / "lazygit_e2e",
        help="where each scenario's screens are saved (default: target/lazygit_e2e)",
    )
    parser.add_argument("--list", action="store_true", help="list the scenarios and exit")
    parser.add_argument("--inside", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.list:
        for s in SCENARIOS:
            print(f"{s.name:28} {s.check}")
        return 0
    return inside(args) if args.inside else host(args)


if __name__ == "__main__":
    sys.exit(main())
