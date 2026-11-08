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

"""The nextest filter `make test-quick` runs: every test outside the fixtures except the corpus
sweeps, the fixtures `src/test/data/quick_fixtures.txt` lists, and every fixture added since that
list was measured.

The list is chosen from per-test line coverage (`make update-quick-fixtures`), so it only knows the
fixtures that existed when it was measured. A fixture added since is run as well, until the next
measurement decides whether it earns its place: the quick tier grows with the corpus on its own
and is pruned back by re-measuring. "Added since" is what git says was added to
`src/test/fixtures/` after the commit the list's header names, plus stubs not committed yet.

A listed fixture whose stub no longer exists is an error, not a silent skip: a renamed fixture
would otherwise drop out of the quick tier unnoticed.

Usage: scripts/quick_fixtures_filter.py    (prints the expression for `cargo nextest run -E`)
"""

import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
LIST = REPO / "src/test/data/quick_fixtures.txt"
FIXTURES = REPO / "src/test/fixtures"

# The corpus sweeps, kept out of the quick tier; the same pattern as `QUICK_EXCLUDED` in
# research/analysis/coverage_sets.py, which leaves their coverage out of what the list must add to.
EXCLUDED = "corpus"


def read_list(path: Path) -> tuple[str | None, list[str]]:
    """The measured-at commit from the header, and the `dataset::stub` names listed."""
    measured_at = None
    names = []
    for line in path.read_text().splitlines():
        found = re.match(r"#\s*measured-at:\s*(\S+)", line)
        if found:
            measured_at = found.group(1)
        elif line.strip() and not line.startswith("#"):
            names.append(line.strip())
    return measured_at, names


def stub_of(name: str) -> Path:
    dataset, stub = name.split("::", 1)
    return FIXTURES / dataset / f"{stub}.rs"


def added_since(commit: str) -> list[str]:
    """Fixture stubs added after `commit`, committed or not, as `dataset::stub`."""

    def git(*args: str) -> list[str]:
        out = subprocess.run(
            ["git", "-C", str(REPO), *args], capture_output=True, text=True, check=True
        ).stdout
        return [line for line in out.splitlines() if line]

    paths = git("diff", "--name-only", "--diff-filter=A", commit, "--", "src/test/fixtures/")
    paths += git("ls-files", "--others", "--exclude-standard", "--", "src/test/fixtures/")
    names = []
    for path in paths:
        parts = Path(path).parts
        # src/test/fixtures/<dataset>/<stub>.rs; the per-dataset `<dataset>.rs` files are not stubs.
        if len(parts) == 5 and parts[4].endswith(".rs"):
            names.append(f"{parts[3]}::{parts[4][: -len('.rs')]}")
    return names


def expression(names: list[str]) -> str:
    outside = f"(not test(/^test::fixtures::/) and not test(/{EXCLUDED}/))"
    if not names:
        return outside
    alternatives = "|".join(sorted(set(names)))
    return f"{outside} or test(/^test::fixtures::({alternatives})::/)"


def main() -> int:
    if not LIST.exists():
        sys.exit(
            f"{LIST.relative_to(REPO)} is missing: measure per-test coverage and write it with"
            " `make update-quick-fixtures` (see the Makefile)"
        )
    measured_at, names = read_list(LIST)
    missing = [name for name in names if not stub_of(name).exists()]
    if missing:
        sys.exit(
            f"{LIST.relative_to(REPO)} lists fixtures whose stubs are gone - re-run `make"
            " update-quick-fixtures`, or remove them by hand:\n  " + "\n  ".join(missing)
        )
    if measured_at and measured_at != "unknown":
        names += added_since(measured_at)
    print(expression(names))
    return 0


if __name__ == "__main__":
    sys.exit(main())
