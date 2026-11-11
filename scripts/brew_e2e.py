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

"""Installs omnidiff through the Homebrew formula and runs the formula's own test, end to end.

What a user's `brew install ivankovic/omnidiff/omnidiff` and `brew test omnidiff` do, against a
build that is not released yet. `--binary` packages an omnidiff binary exactly as release.yml
packages a release asset, renders packaging/homebrew/omnidiff.rb.in with
scripts/render_homebrew_formula.py so that it downloads that tarball from a local directory, and
installs it from a throwaway tap. `--formula` installs an already rendered formula as it stands,
which is how release.yml checks the real assets before pushing the formula to the tap.

Then: `brew install`, `brew test` (the formula's `test do` block, which is what this exists to
run), the installed binary's version against the formula's, the man page and the completions the
formula generates, and `brew uninstall`. The tap is removed afterwards.

Homebrew is taken from PATH or its default prefixes. Where there is none (a Linux machine without
Linuxbrew) it runs in the `homebrew/brew` container under podman or docker; the binary must then
run on that image's Ubuntu, so build a static one:
`cargo build --profile ci --target x86_64-unknown-linux-musl --bin omnidiff` (`make test-homebrew`
does). A binary that does not run there is a skip, not a failure, outside CI (`$CI` unset), so
`make ci` on a newer glibc than the image's still passes; in CI it fails. The formula's other assets get this tarball's checksum: Homebrew only fetches the one for
the platform it runs on.

Usage:  python3 scripts/brew_e2e.py --binary target/ci/omnidiff
        python3 scripts/brew_e2e.py --formula rendered/omnidiff.rb
"""

from __future__ import annotations

import argparse
import hashlib
import os
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import render_homebrew_formula  # noqa: E402

REPO_ROOT = render_homebrew_formula.REPO_ROOT
# What release.yml's "Package (Unix)" step puts in a tarball, in that order.
PACKAGED = ["LICENSE", "THIRD-PARTY-NOTICES.md", "README.md"]
BREW_PREFIXES = [
    "/home/linuxbrew/.linuxbrew/bin/brew",
    "/opt/homebrew/bin/brew",
    "/usr/local/bin/brew",
]
IMAGE = "docker.io/homebrew/brew:latest"
TAP = "omnidiff-e2e/local"
# Where the work directory is mounted in the container, so the formula's file:// URLs hold there.
CONTAINER_WORK = "/work"

# Runs under Homebrew's own environment, on the host or in the container: `$1` is brew, `$2` the
# rendered formula, `$3` the version it must report.
CHECKS = r"""
set -euo pipefail
brew="$1" formula="$2" version="$3"
export HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_ANALYTICS=1 HOMEBREW_NO_ENV_HINTS=1
export HOMEBREW_NO_INSTALL_CLEANUP=1
tap=omnidiff-e2e/local
"$brew" untap --force "$tap" >/dev/null 2>&1 || true
trap '"$brew" uninstall --force "$tap/omnidiff" >/dev/null 2>&1 || true
      "$brew" untap --force "$tap" >/dev/null 2>&1 || true' EXIT
"$brew" tap-new --no-git "$tap" >/dev/null
cp "$formula" "$("$brew" --repo "$tap")/Formula/omnidiff.rb"
echo "== brew install"
"$brew" install "$tap/omnidiff"
echo "== brew test"
"$brew" test "$tap/omnidiff"
prefix="$("$brew" --prefix)"
echo "== installed files"
got="$("$prefix/bin/omnidiff" --version)"
[ "$got" = "omnidiff $version" ] || { echo "installed binary says '$got', formula says $version" >&2; exit 1; }
for file in share/man/man1/omnidiff.1 etc/bash_completion.d/omnidiff \
            share/zsh/site-functions/_omnidiff share/fish/vendor_completions.d/omnidiff.fish; do
  [ -s "$prefix/$file" ] || { echo "the formula did not install $file" >&2; exit 1; }
done
echo "== brew uninstall"
"$brew" uninstall "$tap/omnidiff"
echo "Homebrew install and test of omnidiff $version passed"
"""


def host_asset() -> str:
    """The release asset Homebrew installs on this machine (or in the Linux container)."""
    machine = platform.machine().lower()
    arch = "aarch64" if machine in ("arm64", "aarch64") else "x86_64"
    if sys.platform == "darwin":
        return f"omnidiff-{arch}-apple-darwin.tar.gz"
    return f"omnidiff-{arch}-unknown-linux-musl.tar.gz"


def find_brew() -> str | None:
    found = shutil.which("brew")
    if found:
        return found
    return next((path for path in BREW_PREFIXES if Path(path).is_file()), None)


def find_container_engine() -> str | None:
    return next((engine for engine in ("podman", "docker") if shutil.which(engine)), None)


def package(binary: Path, work: Path, base_url: str) -> tuple[Path, str]:
    """`binary` packaged as the release would, and the formula rendered to fetch it from `work`."""
    version = tomllib.loads((REPO_ROOT / "Cargo.toml").read_text())["package"]["version"]
    asset = work / host_asset()
    with tarfile.open(asset, "w:gz") as tar:
        tar.add(binary, arcname="omnidiff")
        for name in PACKAGED:
            tar.add(REPO_ROOT / name, arcname=name)
    digest = hashlib.sha256(asset.read_bytes()).hexdigest()
    template = render_homebrew_formula.TEMPLATE.read_text()
    names = {
        m.group(2) for m in render_homebrew_formula.PLACEHOLDER.finditer(template) if m.group(2)
    }
    if asset.name not in names:
        sys.exit(f"brew_e2e: the formula names no asset {asset.name} for this platform")
    formula = render_homebrew_formula.render(
        template, version, dict.fromkeys(names, digest), base_url
    )
    out = work / "omnidiff.rb"
    out.write_text(formula)
    return out, version


def runs_in_container(engine: str, binary: Path) -> bool:
    """Whether `binary` starts on the container image's Ubuntu (a glibc build from a newer distro
    does not)."""
    probe = subprocess.run(
        [engine, "run", "--rm", "-v", f"{binary}:/probe:ro,Z", IMAGE, "/probe", "--version"],
        capture_output=True,
        text=True,
        check=False,
    )
    return probe.returncode == 0


def formula_version(formula: Path) -> str:
    match = re.search(r'^\s*version "([^"]+)"', formula.read_text(), re.MULTILINE)
    if not match:
        sys.exit(f"brew_e2e: {formula} declares no version")
    return match.group(1)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--binary", type=Path, help="an omnidiff binary to package and install")
    source.add_argument("--formula", type=Path, help="a rendered formula to install as it stands")
    parser.add_argument(
        "--container", action="store_true", help=f"run in {IMAGE} even when brew is installed"
    )
    args = parser.parse_args()

    brew = None if args.container else find_brew()
    engine = None
    if brew is None:
        engine = find_container_engine()
        if engine is None or sys.platform == "darwin":
            sys.exit("brew_e2e: no Homebrew here, and no podman or docker to run its container")

    if engine and args.binary and not runs_in_container(engine, args.binary.resolve()):
        reason = (
            f"{args.binary} does not run in {IMAGE} (built against a newer glibc?); build a static"
            " one with --target x86_64-unknown-linux-musl, or install Homebrew"
        )
        if os.environ.get("CI"):
            sys.exit(f"brew_e2e: {reason}")
        print(f"brew_e2e: SKIPPED - {reason}", file=sys.stderr)
        return 0

    with tempfile.TemporaryDirectory(prefix="omnidiff-brew-e2e-") as tmp:
        work = Path(tmp)
        # Homebrew runs as its own user in the container; the mount must be readable to it.
        work.chmod(0o755)
        if args.binary:
            base = CONTAINER_WORK if engine else str(work)
            formula, version = package(args.binary.resolve(), work, f"file://{base}")
        else:
            formula = work / "omnidiff.rb"
            shutil.copy(args.formula, formula)
            version = formula_version(formula)
        for path in work.iterdir():
            path.chmod(0o644)

        if engine is None:
            command = ["bash", "-c", CHECKS, "checks", brew, str(formula), version]
        else:
            command = [
                engine, "run", "--rm", "-v", f"{work}:{CONTAINER_WORK}:ro,Z", IMAGE,
                "bash", "-c", CHECKS, "checks", "brew", f"{CONTAINER_WORK}/{formula.name}", version,
            ]  # fmt: skip
        print(
            f"brew_e2e: omnidiff {version} via {'brew' if engine is None else engine}",
            file=sys.stderr,
        )
        return subprocess.run(command, check=False).returncode


if __name__ == "__main__":
    sys.exit(main())
