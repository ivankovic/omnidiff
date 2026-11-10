#!/usr/bin/env python3
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

"""
How the ground-truth dataset grew: when each fixture was first solved, and when it was first
painted, read from the repository's own history.

The dataset is the test corpus of a project under active development, so it is not a snapshot:
fixtures are added as sampling rounds are annotated, as hand-curated cases are written and as
reported misbehaviour is turned into a test. The paper states that with numbers, and this script
is where they come from.

Two steps, kept apart because the first walks the whole history and takes minutes:

* `--measure` asks git when each ground-truth file first appeared and writes
  `data/quality/dataset_growth.csv`, one row per fixture on disk today: its kind (`code` or
  `content`), dataset or content family, language, the date its ground truth
  (`human_mapping.json` or `human_content.json`) was first committed, and for code the date a
  painting (`text_mappings`) was first committed in it.
* Without the flag, the CSV is turned into `plots/dataset_growth.{pdf,png}`,
  `plots/dataset_growth_table.tex` and `plots/variables_growth.tex`.

A fixture is keyed by its directory name, not its path: the fixtures were moved between layouts
(`diffs/<name>` to `diffs/<dataset>/<name>`), and a move is an add under `--no-renames`, so the
earliest add of a name is when the fixture was made. A fixture that was renamed is dated by its
current name, which makes these dates upper bounds. Fixtures deleted since are not counted: the
figure shows the history of what the dataset holds today.

`ISSUE_FIXTURE_PREFIXES` records which fixtures trace to a reported issue. It is authored: the
link between an issue and the fixtures written for it is in the issue tracker and the commit
log, not in the files.

Usage (from research/):  uv run ./analysis/dataset_growth.py [--measure]
"""

import argparse
import csv
import datetime
import json
import subprocess
from collections import Counter
from pathlib import Path

from _common import (
    GRIDLINE,
    INK_MUTED,
    INK_PRIMARY,
    INK_SECONDARY,
    PAPER_DATASETS,
    REPO_ROOT,
    SURFACE,
    latex_number,
    read_rows,
)

# The content families of src/test/data/, as paper_variables.CONTENT_FAMILIES lists them.
CONTENT_FAMILIES = ("pictures", "cursors", "fonts", "archives", "catalogs", "documents")

DATASET_LABELS = {
    "small": "Curated",
    "full": "Full",
    "stratified": "Stratified",
    "defects4j": "Defects4J",
}

# Fixtures written because a user reported the behaviour, by issue number in
# github.com/ivankovic/omnidiff. #69 (2026-11-03) asked for structural SQL diffing; the grammar
# and the SQL fixtures followed it (2026-11-09, PR #90). Checked against all 18 issues open or
# closed on 2026-11-10: no other issue from outside the project led to a fixture.
ISSUE_FIXTURE_PREFIXES = {69: ("sql-",)}

# Issues in the tracker on 2026-11-10 (`gh issue list --state all`, pull requests excluded), and
# how many of them were filed by someone other than the paper's authors.
ISSUES_TOTAL = 18
ISSUES_EXTERNAL = 10

DATA_ROOT = "src/test/data"


def fixture_name(path: str) -> str:
    """The fixture a ground-truth file's path belongs to: its parent directory's name."""
    return path.rsplit("/", 2)[-2]


def earliest_by_fixture(log_output: str, suffixes: tuple[str, ...]) -> dict[str, str]:
    """`{fixture: earliest date}` from `git log --name-only --format=@%ad --date=short` output:
    a `@date` line, then the paths that commit touched. Only paths ending in one of `suffixes`
    count."""
    earliest: dict[str, str] = {}
    date = ""
    for line in log_output.splitlines():
        if line.startswith("@"):
            date = line[1:]
        elif line.endswith(suffixes) and "/" in line:
            name = fixture_name(line)
            if name not in earliest or date < earliest[name]:
                earliest[name] = date
    return earliest


def language_of(name: str) -> str:
    """A code fixture's language: the part of its name before the first hyphen."""
    return name.split("-", 1)[0]


def fixtures_on_disk(repo_root: Path) -> list[dict]:
    """Every fixture that carries ground truth today, as `{fixture, kind, dataset, language}`."""
    out = []
    data = repo_root / DATA_ROOT
    for dataset in ("handmade", *PAPER_DATASETS):
        for path in sorted((data / "diffs" / dataset).glob("*/human_mapping.json")):
            name = path.parent.name
            out.append(
                {"fixture": name, "kind": "code", "dataset": dataset, "language": language_of(name)}
            )
    for family in CONTENT_FAMILIES:
        for path in sorted((data / family).glob("*/human_content.json")):
            # An empty object is a sampled pair nobody has judged yet.
            if not json.loads(path.read_text()):
                continue
            out.append(
                {"fixture": path.parent.name, "kind": "content", "dataset": family, "language": ""}
            )
    return out


def git_log(repo_root: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(repo_root), "log", "--no-renames", "--name-only", "--format=@%ad"]
        + ["--date=short", *args],
        check=True,
        capture_output=True,
        text=True,
    ).stdout


def measure(repo_root: Path, csv_path: Path) -> None:
    """Writes `csv_path` from the repository's history. Minutes: the pickaxe reads every version
    of every mapping file."""
    solved = earliest_by_fixture(
        git_log(repo_root, "--diff-filter=A", "--", DATA_ROOT),
        ("/human_mapping.json", "/human_content.json"),
    )
    painted = earliest_by_fixture(
        git_log(repo_root, '-S"text_mappings"', "--diff-filter=AM", "--", "*human_mapping.json"),
        ("/human_mapping.json",),
    )
    rows = fixtures_on_disk(repo_root)
    with open(csv_path, "w", newline="") as f:
        writer = csv.writer(f, lineterminator="\n")
        writer.writerow(["fixture", "kind", "dataset", "language", "solved", "painted"])
        for row in rows:
            name = row["fixture"]
            # Only a fixture painted today has a painting date: one painted and since cleared
            # is not painted.
            is_painted = row["kind"] == "code" and has_painting(repo_root, row)
            writer.writerow(
                [
                    name,
                    row["kind"],
                    row["dataset"],
                    row["language"],
                    solved.get(name, ""),
                    painted.get(name, "") if is_painted else "",
                ]
            )
    print(f"{len(rows)} fixtures written to {csv_path}")


def has_painting(repo_root: Path, row: dict) -> bool:
    path = repo_root / DATA_ROOT / "diffs" / row["dataset"] / row["fixture"] / "human_mapping.json"
    return bool(json.loads(path.read_text()).get("text_mappings"))


def month_of(date: str) -> str:
    return date[:7]


def week_start(date: str) -> datetime.date:
    """The Monday of `date`'s week."""
    day = datetime.date.fromisoformat(date)
    return day - datetime.timedelta(days=day.weekday())


def in_paper(row: dict) -> bool:
    """Whether the paper reports on the fixture: a sampled code dataset or a content family."""
    return row["dataset"] in PAPER_DATASETS or row["kind"] == "content"


def summarize(rows: list[dict]) -> dict:
    """The growth figures the paper quotes, from the measured rows."""
    rows = [r for r in rows if in_paper(r) and r["solved"]]
    code = [r for r in rows if r["kind"] == "code"]
    content = [r for r in rows if r["kind"] == "content"]
    painted = [r for r in code if r["painted"]]
    dates = sorted(r["solved"] for r in rows)
    first, last = dates[0], max(dates + [r["painted"] for r in painted])
    weeks = (week_start(last) - week_start(first)).days // 7 + 1
    active_weeks = {week_start(r["solved"]) for r in rows} | {
        week_start(r["painted"]) for r in painted
    }
    per_week = Counter(week_start(r["solved"]) for r in rows)
    months = sorted(
        {month_of(r["solved"]) for r in rows} | {month_of(r["painted"]) for r in painted}
    )
    monthly = []
    seen_languages: set[str] = set()
    for month in months:
        new_code = [r for r in code if month_of(r["solved"]) == month]
        seen_languages |= {r["language"] for r in new_code}
        monthly.append(
            {
                "month": month,
                **{
                    dataset: sum(r["dataset"] == dataset for r in new_code)
                    for dataset in PAPER_DATASETS
                },
                "painted": sum(month_of(r["painted"]) == month for r in painted),
                "content": sum(month_of(r["solved"]) == month for r in content),
                "languages": len(seen_languages),
            }
        )
    issue = {
        number: [r for r in code if r["fixture"].startswith(prefixes)]
        for number, prefixes in ISSUE_FIXTURE_PREFIXES.items()
    }
    return {
        "code": len(code),
        "content": len(content),
        "painted": len(painted),
        "first": first,
        "last": last,
        "weeks": weeks,
        "active_weeks": len(active_weeks),
        "max_week": max(per_week.values()),
        "median_week": sorted(
            per_week.get(week_start(first) + datetime.timedelta(weeks=i), 0) for i in range(weeks)
        )[weeks // 2],
        "monthly": monthly,
        "issue_fixtures": sum(len(v) for v in issue.values()),
        "issues_with_fixtures": sum(1 for v in issue.values() if v),
        "languages": len({r["language"] for r in code}),
    }


def cumulative(dates: list[str]) -> tuple[list[datetime.date], list[int]]:
    """Step-plot coordinates of a cumulative count over the sorted `dates`."""
    counts = Counter(dates)
    xs, ys, total = [], [], 0
    for date in sorted(counts):
        total += counts[date]
        xs.append(datetime.date.fromisoformat(date))
        ys.append(total)
    return xs, ys


MONTH_NAMES = "Jan Feb Mar Apr May Jun Jul Aug Sep Oct Nov Dec".split()


def month_label(month: str) -> str:
    """`2026-09` -> `Sep 2026`."""
    year, number = month.split("-")
    return f"{MONTH_NAMES[int(number) - 1]} {year}"


def table_tex(s: dict) -> str:
    """The paper's table of fixtures added per month, as a complete `table` environment."""
    lines = [
        "% Auto-generated by research/analysis/dataset_growth.py. Do not edit by hand -",
        "% regenerate: make dataset-growth-report (from research/).",
        r"\begin{table}",
        r"  \caption{Ground truth added per month, by the date it was first committed. The four",
        r"  dataset columns count code changes with a node mapping; \emph{Painted} counts code",
        r"  changes whose first painting was committed that month, \emph{Content} judged changes",
        r"  to other content, and \emph{Lang.} the languages the code changes cover by then.}",
        r"  \label{tab:growth}",
        r"  \small",
        r"  \setlength{\tabcolsep}{3.5pt}",
        r"  \begin{tabular}{@{}lrrrrrrr@{}}",
        r"    \toprule",
        r"    Month & Cur. & Full & Strat. & D4J & Painted & Content & Lang. \\",
        r"    \midrule",
    ]
    for m in s["monthly"]:
        cells = [m[d] for d in PAPER_DATASETS] + [m["painted"], m["content"], m["languages"]]
        lines.append(
            f"    {month_label(m['month'])} & "
            + " & ".join(latex_number(c) if c else "--" for c in cells)
            + r" \\"
        )
    totals = [sum(m[d] for m in s["monthly"]) for d in PAPER_DATASETS] + [
        s["painted"],
        s["content"],
        s["languages"],
    ]
    lines += [
        r"    \midrule",
        r"    \emph{All} & " + " & ".join(latex_number(c) for c in totals) + r" \\",
        r"    \bottomrule",
        r"  \end{tabular}",
        r"\end{table}",
    ]
    return "\n".join(lines) + "\n"


def macros(s: dict) -> dict:
    first = datetime.date.fromisoformat(s["first"])
    return {
        "GrowthCodeFixtures": latex_number(s["code"]),
        "GrowthContentFixtures": latex_number(s["content"]),
        "GrowthPaintedFixtures": latex_number(s["painted"]),
        "GrowthFirstMonth": f"{first.strftime('%B')} {first.year}",
        "GrowthWeeks": s["weeks"],
        "GrowthActiveWeeks": s["active_weeks"],
        "GrowthMedianPerWeek": s["median_week"],
        "GrowthMaxPerWeek": s["max_week"],
        "GrowthIssueFixtures": s["issue_fixtures"],
        "GrowthIssuesWithFixtures": s["issues_with_fixtures"],
        "GrowthIssuesTotal": ISSUES_TOTAL,
        "GrowthIssuesExternal": ISSUES_EXTERNAL,
    }


def plot(rows: list[dict], out: Path) -> None:
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.dates as mdates
    import matplotlib.pyplot as plt

    rows = [r for r in rows if in_paper(r) and r["solved"]]
    series = [
        (
            "Code changes with a node mapping",
            [r["solved"] for r in rows if r["kind"] == "code"],
            "#2a78d6",
            "-",
        ),
        (
            "Code changes with a painting",
            [r["painted"] for r in rows if r["kind"] == "code" and r["painted"]],
            "#b85c1e",
            "--",
        ),
        (
            "Judged changes to other content",
            [r["solved"] for r in rows if r["kind"] == "content"],
            "#1a9e96",
            ":",
        ),
    ]
    end = max(datetime.date.fromisoformat(d) for _, dates, _, _ in series for d in dates)
    fig, ax = plt.subplots(figsize=(3.6, 2.5))
    fig.patch.set_facecolor(SURFACE)
    ax.set_facecolor(SURFACE)
    for label, dates, color, style in series:
        xs, ys = cumulative(dates)
        ax.step(
            xs + [end],
            ys + [ys[-1]],
            where="post",
            color=color,
            linestyle=style,
            linewidth=2,
            label=label,
        )
        ax.annotate(
            latex_number(ys[-1]).replace("{,}", ","),
            (end, ys[-1]),
            xytext=(3, 0),
            textcoords="offset points",
            va="center",
            fontsize=7.5,
            color=INK_PRIMARY,
        )
    ax.grid(axis="y", color=GRIDLINE, zorder=0)
    for spine in ("top", "right"):
        ax.spines[spine].set_visible(False)
    for spine in ("left", "bottom"):
        ax.spines[spine].set_color(INK_MUTED)
    ax.tick_params(colors=INK_SECONDARY, labelsize=7.5)
    ax.xaxis.set_major_locator(mdates.MonthLocator(interval=2))
    ax.xaxis.set_major_formatter(mdates.DateFormatter("%b"))
    ax.set_ylabel("Changes with ground truth", fontsize=8, color=INK_SECONDARY)
    ax.set_xlabel(str(end.year), fontsize=8, color=INK_SECONDARY)
    ax.set_ylim(0, None)
    ax.legend(
        fontsize=7,
        frameon=False,
        loc="lower left",
        bbox_to_anchor=(0, 1.0),
        labelcolor=INK_SECONDARY,
        handlelength=3,
    )
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out.with_suffix(".pdf"), bbox_inches="tight", facecolor=SURFACE)
    fig.savefig(out.with_suffix(".png"), dpi=170, bbox_inches="tight", facecolor=SURFACE)
    plt.close(fig)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--measure", action="store_true", help="re-read git history (minutes)")
    parser.add_argument(
        "--csv", type=Path, default=REPO_ROOT / "research/data/quality/dataset_growth.csv"
    )
    parser.add_argument("--plots-dir", type=Path, default=REPO_ROOT / "research/plots")
    args = parser.parse_args()

    if args.measure:
        measure(REPO_ROOT, args.csv)
    rows = read_rows(args.csv)
    s = summarize(rows)
    for m in s["monthly"]:
        print(m)
    values = macros(s)
    for name, value in values.items():
        print(f"{name:28} {value}")
    plot(rows, args.plots_dir / "dataset_growth")
    (args.plots_dir / "dataset_growth_table.tex").write_text(table_tex(s))
    lines = [
        "% Auto-generated by research/analysis/dataset_growth.py. Do not edit by hand -",
        "% regenerate: make dataset-growth-report (from research/).",
    ] + [f"\\newcommand{{\\{k}}}{{{v}}}" for k, v in values.items()]
    (args.plots_dir / "variables_growth.tex").write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
