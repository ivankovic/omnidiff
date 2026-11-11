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

"""Renders the Homebrew formula from packaging/homebrew/omnidiff.rb.in and a release's
SHA256SUMS.txt.

The template names the release assets it installs; this fills in the version and each asset's
checksum, and refuses to leave a placeholder behind - a formula naming an asset the release does
not carry would fail at `brew install`, on a user's machine, rather than here. release.yml runs
it after the checksums job and pushes the result to the tap.

Usage:  python3 scripts/render_homebrew_formula.py --version 0.1.0 --sums SHA256SUMS.txt \\
            --out Formula/omnidiff.rb
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
TEMPLATE = REPO_ROOT / "packaging" / "homebrew" / "omnidiff.rb.in"

PLACEHOLDER = re.compile(r"\{\{(VERSION|RELEASE_URL|SHA256:([^}]+))\}\}")


def release_url(version: str) -> str:
    """Where a published release's assets are downloaded from."""
    return f"https://github.com/ivankovic/omnidiff/releases/download/v{version}"


def checksums(sums_text: str) -> dict[str, str]:
    """`sha256sum` output as `{file name: hex digest}`; a `*` binary-mode marker is dropped."""
    out: dict[str, str] = {}
    for line in sums_text.splitlines():
        line = line.strip()
        if not line:
            continue
        digest, name = line.split(maxsplit=1)
        out[name.lstrip("*").strip()] = digest
    return out


def render(template: str, version: str, sums: dict[str, str], base_url: str | None = None) -> str:
    """The formula, or a `KeyError` naming the first asset the checksums do not cover. Assets are
    fetched from `base_url`, by default the release's download URL."""

    def fill(match: re.Match[str]) -> str:
        if match.group(1) == "VERSION":
            return version
        if match.group(1) == "RELEASE_URL":
            return base_url or release_url(version)
        asset = match.group(2)
        if asset not in sums:
            raise KeyError(asset)
        return sums[asset]

    return PLACEHOLDER.sub(fill, template)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--version", required=True, help="release version, without the v")
    parser.add_argument("--sums", type=Path, required=True, help="the release's SHA256SUMS.txt")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument(
        "--base-url",
        help="fetch the assets from here instead of the release (scripts/brew_e2e.py)",
    )
    args = parser.parse_args()

    sums = checksums(args.sums.read_text())
    try:
        formula = render(TEMPLATE.read_text(), args.version, sums, args.base_url)
    except KeyError as missing:
        sys.exit(f"render_homebrew_formula: {args.sums} has no entry for {missing.args[0]}")
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(formula)
    print(f"wrote {args.out} for omnidiff {args.version}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
