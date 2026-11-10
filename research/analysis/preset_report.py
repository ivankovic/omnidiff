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
Minimal against Full: what the generous hand painting adds to the sparse one.

A painted fixture answers one of three cases (`human_mapping::graded_cases`): it carries a
`Minimal` and a `Full` painting, the annotator's finding that the change has a sparse and a
generous correct rendering; it carries a lone painting naming no preset (`Only one solution`), the
finding that it has one rendering and both presets must produce it; or it carries a lone painting
named for one preset. The first half of this report counts the cases, from the mapping files.

The second half reads `data/quality/preset_census.csv`, which the `preset_census` test writes for
every fixture with both paintings: per combination of (`Minimal` label, `Full` label, text class),
the runs and bytes carrying it, over both files. From it: how many bytes each preset paints, how
much `Full` adds and of what kind, how often the two paintings are byte-identical, and how the
difference spreads over fixtures and languages. "Adds" is a byte `Full` paints and `Minimal`
leaves unpainted; a byte both paint under different labels is counted apart as relabelled.

Scoped to `_common.PAPER_DATASETS`, like every report here. Writes
`plots/preset_difference.{pdf,png}`, `plots/preset_table.tex` and `plots/variables_presets.tex`.

Usage (from research/):  uv run ./analysis/preset_report.py
"""

import argparse
import json
import statistics
from collections import Counter, defaultdict
from pathlib import Path

from _common import (
    GRIDLINE,
    INK_MUTED,
    INK_PRIMARY,
    INK_SECONDARY,
    PAPER_DATASETS,
    REPO_ROOT,
    SURFACE,
    fixture_datasets,
    latex_number,
    read_rows,
)

CLASSES = ("whitespace", "punctuation", "code")
LABELS = ("insert", "delete", "update", "move")
UNPAINTED = "-"

# A language enters the per-language panel with at least this many two-painting fixtures.
MIN_FIXTURES_PER_LANGUAGE = 15


def pct(part: float, whole: float) -> float:
    return 100.0 * part / whole if whole else 0.0


def designates_preset(name: str, preset: str) -> bool:
    """`human_mapping::designates_preset`: the preset's name, or the name, a space and a
    qualifier (`Minimal (left)`)."""
    return name == preset or name.startswith(preset + " ")


def graded_case(names: list[str]) -> str | None:
    """Which case a fixture's painting names put it in - `human_mapping::graded_cases`. `None`
    for an unpainted fixture."""
    if not names:
        return None
    if len(names) > 1:
        return "both"
    if designates_preset(names[0], "Full"):
        return "full"
    if designates_preset(names[0], "Minimal"):
        return "minimal"
    return "single"


def painting_cases(repo_root: Path) -> Counter:
    """How many of the paper's fixtures fall in each case, `unpainted` included."""
    cases: Counter = Counter()
    for dataset in PAPER_DATASETS:
        pattern = f"src/test/data/diffs/{dataset}/*/human_mapping.json"
        for path in repo_root.glob(pattern):
            paintings = json.loads(path.read_text()).get("text_mappings") or []
            cases[graded_case([p["name"] for p in paintings]) or "unpainted"] += 1
    return cases


def language_of(fixture: str) -> str:
    return fixture.split("-", 1)[0]


def per_fixture(rows: list[dict]) -> dict[str, dict]:
    """The census rows folded per fixture: its size, the bytes each preset paints, and the bytes
    `Full` adds by class and by label."""
    out: dict[str, dict] = defaultdict(
        lambda: {
            "total": 0,
            "minimal": 0,
            "full": 0,
            "both_same": 0,
            "relabelled": 0,
            "minimal_only": 0,
            "added": Counter(),
            "added_runs": Counter(),
        }
    )
    for row in rows:
        fixture = out[row["fixture"]]
        size, runs = int(row["bytes"]), int(row["runs"])
        if row["class"] == "total":
            fixture["total"] = size
            continue
        minimal, full = row["minimal"], row["full"]
        if minimal != UNPAINTED:
            fixture["minimal"] += size
        if full != UNPAINTED:
            fixture["full"] += size
        if minimal == UNPAINTED:
            fixture["added"][(row["class"], full)] += size
            fixture["added_runs"][(row["class"], full)] += runs
        elif full == UNPAINTED:
            fixture["minimal_only"] += size
        elif minimal == full:
            fixture["both_same"] += size
        else:
            fixture["relabelled"] += size
    return dict(out)


def summarize(fixtures: dict[str, dict]) -> dict:
    """Every number the paper quotes about the two presets."""
    added: Counter = Counter()
    added_runs: Counter = Counter()
    for f in fixtures.values():
        added.update(f["added"])
        added_runs.update(f["added_runs"])
    minimal = sum(f["minimal"] for f in fixtures.values())
    full = sum(f["full"] for f in fixtures.values())
    identical = [
        name
        for name, f in fixtures.items()
        if not f["added"] and not f["relabelled"] and not f["minimal_only"]
    ]
    extra = sorted(sum(f["added"].values()) for f in fixtures.values())
    # Of what Full paints, the share Minimal paints as well; a fixture Full paints nothing in
    # has no such share.
    kept = sorted(
        pct(f["full"] - sum(f["added"].values()), f["full"]) for f in fixtures.values() if f["full"]
    )
    languages: dict[str, dict] = defaultdict(lambda: {"fixtures": 0, "full": 0, "added": Counter()})
    for name, f in fixtures.items():
        language = languages[language_of(name)]
        language["fixtures"] += 1
        language["full"] += f["full"]
        for (text_class, _), size in f["added"].items():
            language["added"][text_class] += size
    return {
        "fixtures": len(fixtures),
        "minimal_bytes": minimal,
        "full_bytes": full,
        "added": added,
        "added_runs": added_runs,
        "added_bytes": sum(added.values()),
        "relabelled": sum(f["relabelled"] for f in fixtures.values()),
        "minimal_only": sum(f["minimal_only"] for f in fixtures.values()),
        "identical": len(identical),
        "extra_median": statistics.median(extra) if extra else 0,
        "extra_p90": extra[9 * len(extra) // 10] if extra else 0,
        "extra_max": extra[-1] if extra else 0,
        "kept": kept,
        "kept_median": statistics.median(kept) if kept else 0.0,
        "languages": dict(languages),
    }


def macros(s: dict, cases: dict) -> dict:
    cases = Counter(cases)
    painted = sum(v for k, v in cases.items() if k != "unpainted")
    by_class = {c: sum(v for (cls, _), v in s["added"].items() if cls == c) for c in CLASSES}
    by_label = {
        label: sum(v for (_, lab), v in s["added"].items() if lab == label) for label in LABELS
    }
    out = {
        "PresetPainted": latex_number(painted),
        "PresetBothFixtures": latex_number(cases["both"]),
        "PresetBothPct": f"{pct(cases['both'], painted):.1f}",
        "PresetSingleFixtures": latex_number(cases["single"]),
        "PresetSinglePct": f"{pct(cases['single'], painted):.1f}",
        "PresetMinimalOnlyFixtures": latex_number(cases["minimal"]),
        "PresetFullOnlyFixtures": latex_number(cases["full"]),
        "PresetCensusFixtures": latex_number(s["fixtures"]),
        "PresetMinimalBytes": latex_number(s["minimal_bytes"]),
        "PresetFullBytes": latex_number(s["full_bytes"]),
        "PresetFullOverMinimal": f"{s['full_bytes'] / s['minimal_bytes']:.2f}"
        if s["minimal_bytes"]
        else "0",
        "PresetAddedBytes": latex_number(s["added_bytes"]),
        "PresetAddedPctOfFull": f"{pct(s['added_bytes'], s['full_bytes']):.1f}",
        "PresetRelabelledBytes": latex_number(s["relabelled"]),
        "PresetMinimalOnlyBytes": latex_number(s["minimal_only"]),
        "PresetIdenticalFixtures": latex_number(s["identical"]),
        "PresetIdenticalPct": f"{pct(s['identical'], s['fixtures']):.1f}",
        "PresetExtraMedian": f"{s['extra_median']:.0f}",
        "PresetExtraPNinety": latex_number(s["extra_p90"]),
        "PresetExtraMax": latex_number(s["extra_max"]),
        "PresetKeptMedianPct": f"{s['kept_median']:.1f}",
    }
    for text_class in CLASSES:
        stem = text_class.capitalize()
        out[f"PresetAdded{stem}Bytes"] = latex_number(by_class[text_class])
        out[f"PresetAdded{stem}Pct"] = f"{pct(by_class[text_class], s['added_bytes']):.1f}"
    for label in LABELS:
        stem = label.capitalize()
        out[f"PresetAdded{stem}Bytes"] = latex_number(by_label[label])
        out[f"PresetAdded{stem}Pct"] = f"{pct(by_label[label], s['added_bytes']):.1f}"
    return out


def table_tex(s: dict) -> str:
    """The paper's table of what `Full` adds: label by text class, in bytes."""
    lines = [
        "% Auto-generated by research/analysis/preset_report.py. Do not edit by hand -",
        "% regenerate: make preset-report (from research/).",
        r"\begin{table}",
        r"  \caption{Bytes the \emph{Full} painting paints and the \emph{Minimal} painting of the",
        r"  same change leaves unpainted, by the label \emph{Full} gives them and by what they",
        r"  hold, over the " + latex_number(s["fixtures"]) + r" changes painted both ways.}",
        r"  \label{tab:presets}",
        r"  \small",
        r"  \begin{tabular}{@{}lrrrr@{}}",
        r"    \toprule",
        r"    Label & Whitespace & Punctuation & Code & All \\",
        r"    \midrule",
    ]
    for label in LABELS:
        cells = [s["added"].get((c, label), 0) for c in CLASSES]
        lines.append(
            f"    {label.capitalize()} & "
            + " & ".join(latex_number(c) for c in cells + [sum(cells)])
            + r" \\"
        )
    totals = [sum(v for (cls, _), v in s["added"].items() if cls == c) for c in CLASSES]
    lines += [
        r"    \midrule",
        r"    \emph{All} & " + " & ".join(latex_number(c) for c in totals + [sum(totals)]) + r" \\",
        r"    \emph{Share} & "
        + " & ".join(f"{pct(c, sum(totals)):.1f}\\%" for c in totals)
        + r" & \\",
        r"    \bottomrule",
        r"  \end{tabular}",
        r"\end{table}",
    ]
    return "\n".join(lines) + "\n"


def plot(s: dict, out: Path) -> None:
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    fig, (left, right) = plt.subplots(
        1, 2, figsize=(7.2, 2.9), gridspec_kw={"width_ratios": [1, 1.25], "wspace": 0.5}
    )
    fig.patch.set_facecolor(SURFACE)
    for ax in (left, right):
        ax.set_facecolor(SURFACE)
        for spine in ("top", "right"):
            ax.spines[spine].set_visible(False)
        for spine in ("left", "bottom"):
            ax.spines[spine].set_color(INK_MUTED)
        ax.tick_params(colors=INK_SECONDARY, labelsize=7.5)

    kept = s["kept"]
    shares = [100.0 * (i + 1) / len(kept) for i in range(len(kept))]
    left.step(kept, shares, where="post", color="#2a78d6", linewidth=2)
    left.grid(color=GRIDLINE, zorder=0)
    left.set_xlim(0, 100)
    left.set_ylim(0, 100)
    left.set_xlabel(
        "Painted by Minimal too (% of Full's painted bytes)",
        fontsize=7.5,
        color=INK_SECONDARY,
    )
    left.set_ylabel("Changes at or below (%)", fontsize=8, color=INK_SECONDARY)
    median = s["kept_median"]
    left.axvline(median, color=INK_MUTED, linewidth=1, linestyle="--")
    left.annotate(
        f"median {median:.0f}%",
        (median, 50),
        xytext=(-4, 0),
        textcoords="offset points",
        ha="right",
        fontsize=7.5,
        color=INK_PRIMARY,
    )

    languages = sorted(
        (
            (name, v)
            for name, v in s["languages"].items()
            if v["fixtures"] >= MIN_FIXTURES_PER_LANGUAGE and v["full"]
        ),
        key=lambda item: sum(item[1]["added"].values()) / item[1]["full"],
    )
    styles = {
        "whitespace": ("#c9a227", "///"),
        "punctuation": ("#1a9e96", "..."),
        "code": ("#4a3aa7", ""),
    }
    offsets = [0.0] * len(languages)
    for text_class in CLASSES:
        color, hatch = styles[text_class]
        widths = [pct(v["added"][text_class], v["full"]) for _, v in languages]
        right.barh(
            range(len(languages)),
            widths,
            left=offsets,
            height=0.62,
            color=color,
            hatch=hatch,
            edgecolor=SURFACE,
            linewidth=0.6,
            label=text_class.capitalize(),
            zorder=2,
        )
        offsets = [o + w for o, w in zip(offsets, widths, strict=True)]
    right.set_yticks(range(len(languages)))
    right.set_yticklabels([f"{name} ({v['fixtures']})" for name, v in languages], fontsize=7)
    right.grid(axis="x", color=GRIDLINE, zorder=0)
    right.set_xlabel(
        "Painted by Full only (% of Full's painted bytes)",
        fontsize=7.5,
        color=INK_SECONDARY,
    )
    right.legend(fontsize=7, frameon=False, loc="lower right", labelcolor=INK_SECONDARY)
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out.with_suffix(".pdf"), bbox_inches="tight", facecolor=SURFACE)
    fig.savefig(out.with_suffix(".png"), dpi=170, bbox_inches="tight", facecolor=SURFACE)
    plt.close(fig)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "--csv", type=Path, default=REPO_ROOT / "research/data/quality/preset_census.csv"
    )
    parser.add_argument("--plots-dir", type=Path, default=REPO_ROOT / "research/plots")
    args = parser.parse_args()

    datasets = fixture_datasets()
    rows = [r for r in read_rows(args.csv) if datasets.get(r["fixture"]) in PAPER_DATASETS]
    s = summarize(per_fixture(rows))
    cases = painting_cases(REPO_ROOT)
    values = macros(s, cases)
    print(dict(cases))
    for name, value in values.items():
        print(f"{name:30} {value}")
    plot(s, args.plots_dir / "preset_difference")
    (args.plots_dir / "preset_table.tex").write_text(table_tex(s))
    lines = [
        "% Auto-generated by research/analysis/preset_report.py. Do not edit by hand -",
        "% regenerate: make preset-report (from research/).",
    ] + [f"\\newcommand{{\\{k}}}{{{v}}}" for k, v in values.items()]
    (args.plots_dir / "variables_presets.tex").write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
