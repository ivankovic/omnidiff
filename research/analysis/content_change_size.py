#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
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
#  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#  GNU Affero General Public License for more details.
#
#  You should have received a copy of the GNU Affero General Public License
#  along with this program.  If not, see <https://www.gnu.org/licenses/>.
"""
How large a change to content other than code is: for every sampled content pair (the pairs of
`data/performance/content_speed/<machine>.csv`, materialised under `src/test/data/samples/`),
the size of both files in bytes and how much of the content changed, in the content's own unit,
read from `omnidiff --mode json`. The unit is the pixel for pictures and cursors (`changed_pixels`
of `total_pixels`, where the comparison is a still picture at one size or a resized one), and
the member for containers: the entries of an archive, the tables and glyphs of a font, the
messages of a catalog, the pages and parts of a PDF (members listed as changed, added or removed,
of those and the unchanged ones). A pair OmniDiff does not decode, or a picture comparison with no
single pixel count (an animation), has empty unit columns and still has its byte sizes.

Writes `data/corpus_stats/content_change_size.csv`, one row per pair. `make
measure-content-change-size`; `paper_variables.content_change_size` reads it.
"""

import argparse
import csv
import glob
import json
import os
import subprocess
import sys

RESEARCH_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ROOT = os.path.dirname(RESEARCH_DIR)


def changed_share(report):
    """(unit, changed, total) of one JSON report, or (unit, None, None) when it has no count."""
    content = report.get("content") or {}
    if content.get("kind") == "container":
        changed = len(content.get("members", []))
        return "members", changed, changed + content.get("unchanged", 0)
    if content.get("kind") == "picture":
        comparison = content.get("comparison") or {}
        for counted in (comparison, comparison.get("scaled") or {}):
            if "changed_pixels" in counted and "total_pixels" in counted:
                return "pixels", counted["changed_pixels"], counted["total_pixels"]
        return "pixels", None, None
    return "", None, None


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "--machine", default="server", help="whose content_speed CSV lists the pairs"
    )
    parser.add_argument("--omnidiff", default=os.path.join(ROOT, "target", "release", "omnidiff"))
    args = parser.parse_args()

    pairs = os.path.join(
        RESEARCH_DIR, "data", "performance", "content_speed", f"{args.machine}.csv"
    )
    samples = os.path.join(ROOT, "src", "test", "data", "samples")
    out_path = os.path.join(RESEARCH_DIR, "data", "corpus_stats", "content_change_size.csv")
    rows = []
    with open(pairs, newline="") as f:
        for pair in csv.DictReader(f):
            directory = os.path.join(samples, pair["sample"])
            before = glob.glob(os.path.join(glob.escape(directory), "before.*"))
            after = glob.glob(os.path.join(glob.escape(directory), "after.*"))
            if len(before) != 1 or len(after) != 1:
                sys.exit(
                    f"{directory}: expected one before.* and one after.* (materialize_test_diffs)"
                )
            done = subprocess.run(
                [args.omnidiff, "--mode", "json", before[0], after[0]],
                capture_output=True,
                text=True,
                check=False,
            )
            try:
                unit, changed, total = changed_share(json.loads(done.stdout))
            except json.JSONDecodeError:
                unit, changed, total = "", None, None
            rows.append(
                [
                    pair["family"],
                    pair["sample"],
                    os.path.getsize(before[0]),
                    os.path.getsize(after[0]),
                    unit,
                    "" if changed is None else changed,
                    "" if total is None else total,
                ]
            )
    with open(out_path, "w", newline="") as f:
        writer = csv.writer(f, lineterminator="\n")
        writer.writerow(
            [
                "family",
                "sample",
                "before_bytes",
                "after_bytes",
                "unit",
                "changed_units",
                "total_units",
            ]
        )
        writer.writerows(rows)
    counted = sum(1 for row in rows if row[5] != "")
    print(f"{len(rows)} pairs, {counted} with a unit count -> {out_path}")


if __name__ == "__main__":
    main()
