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
The ground-truth invariants as a measure of the gap between an edit script and a visualization.

`src/test/helper/human_mapping/invariants.rs` states 31 rules a fixture's ground truth must obey
(its module doc lists them). They are rules about how a change is *shown*: nothing in them is
needed for a delta to convert one file into the other. So the number of them an account of a
change breaks is a distance from being a visualization, and this report measures it for five
accounts, from `data/quality/invariant_gap_census.csv` (the `invariant_gap_census` test):

* `human` - the hand paintings themselves. What is left here is ground-truth error.
* `line_script` - GNU diff's edit script, each deleted or inserted line painted whole.
* `tree_script` - the human tree mapping read as a tree edit script and painted leaf by leaf.
* `human_mapping_rendered` and `tool_mapping_rendered` - the human mapping and OmniDiff's own,
  through OmniDiff's renderer. Judged by the label-reading rules only; see the test's doc.

`GROUPS` sorts the 31 rules into the five families the paper's table prints. `MAPPING_ONLY` are
the rules that never read a painting: they judge the tree mapping, which every account shares,
so they are reported for `human` alone.

The comparison population is the painted fixtures of `_common.PAPER_DATASETS` that have a tree
and no embedded layer, so that every account is judged on the same changes and by the same
rules. `human`'s own totals are also given over every paper fixture.

Writes `plots/invariants_table.tex` and `plots/variables_invariants.tex`.

Usage (from research/):  uv run ./analysis/invariants_report.py
"""

import argparse
from pathlib import Path

from _common import PAPER_DATASETS, REPO_ROOT, fixture_datasets, latex_number, read_rows

INVARIANTS = range(1, 32)

# (key, the table's row label, the rules, an example the table prints)
GROUPS = [
    (
        "Form",
        "Form of a painting",
        (1, 7, 19),
        r"no highlight ends in trailing whitespace; \texttt{<=} is painted whole",
    ),
    (
        "Presets",
        "What each preset paints",
        (2, 4, 5, 6, 8, 16, 29, 30, 31),
        (
            r"\emph{Minimal} never paints indentation; \emph{Full} paints a renamed identifier "
            r"entire; the space before an insertion follows its count"
        ),
    ),
    (
        "Agreement",
        "Painting agrees with mapping",
        (9, 10, 11, 12, 13, 17, 21, 22, 23, 27),
        r"a leaf paired with an identical leaf is never painted deleted and inserted",
    ),
    (
        "Units",
        "Syntactic units stay together",
        (3, 24, 25, 26),
        r"a \texttt{;} goes with its statement, a call's dot with its member",
    ),
    (
        "Mapping",
        "Form of a mapping",
        (14, 15, 18, 20, 28),
        (
            r"\texttt{NULL} $\rightarrow$ \texttt{nullptr} is one element changing, not a delete "
            r"beside an insert"
        ),
    ),
]

# Rules that read the tree mapping and no painting.
MAPPING_ONLY = (3, 14, 15, 18, 20, 26, 28)

# A leaf the human mapping pairs, painted as deleted on one side and inserted on the other (10,
# 27) or as wholly removed (22): what a script does to code that survived.
SURVIVOR_RULES = (10, 22, 27)
# The rules about which whitespace a painting covers.
WHITESPACE_RULES = (4, 5, 6, 30)

SUBJECTS = (
    "human",
    "line_script",
    "tree_script",
    "human_mapping_rendered",
    "tool_mapping_rendered",
)
STEMS = {
    "human": "Human",
    "line_script": "LineScript",
    "tree_script": "TreeScript",
    "human_mapping_rendered": "HumanRendered",
    "tool_mapping_rendered": "ToolRendered",
}


def pct(part: int, whole: int) -> float:
    return 100.0 * part / whole if whole else 0.0


def count(row: dict, invariant: int) -> int | None:
    """The violations of `invariant` in one census row, `None` where the account is not judged
    by it."""
    value = row[f"i{invariant}"]
    return int(value) if value != "" else None


def tally(rows: list[dict], invariants) -> tuple[int, int]:
    """`(violations, fixtures with at least one)` of `invariants` over `rows`."""
    violations = fixtures = 0
    for row in rows:
        found = sum(count(row, i) or 0 for i in invariants)
        violations += found
        fixtures += found > 0
    return violations, fixtures


def judged_by(rows: list[dict]) -> tuple[int, ...]:
    """The invariants every one of `rows` carries a count for."""
    return tuple(i for i in INVARIANTS if rows and all(count(r, i) is not None for r in rows))


def by_subject(rows: list[dict]) -> dict[str, list[dict]]:
    return {s: [r for r in rows if r["subject"] == s] for s in SUBJECTS}


def summarize(rows: list[dict]) -> dict:
    """Per-account totals over the comparison population, and `human`'s over every fixture."""
    comparable = [r for r in rows if r["painted"] == "1" and r["embedded"] == "0"]
    subjects = by_subject(comparable)
    painting_rules = tuple(i for i in INVARIANTS if i not in MAPPING_ONLY)
    label_rules = tuple(i for i in judged_by(subjects["tool_mapping_rendered"]))
    out = {
        "fixtures": len(subjects["human"]),
        "human_all": tally([r for r in rows if r["subject"] == "human"], INVARIANTS),
        "human_all_fixtures": sum(r["subject"] == "human" for r in rows),
        "label_rules": label_rules,
        "painting_rules": painting_rules,
        "groups": {},
        "scripts": {},
        "label": {},
        "broken_rules": {},
    }
    for subject in ("human", "line_script", "tree_script"):
        out["scripts"][subject] = tally(subjects[subject], painting_rules)
        out["broken_rules"][subject] = sum(
            tally(subjects[subject], (i,))[0] > 0 for i in painting_rules
        )
        for key, _, rules, _ in GROUPS:
            scoped = rules if subject == "human" else [i for i in rules if i not in MAPPING_ONLY]
            out["groups"][(subject, key)] = tally(subjects[subject], scoped) if scoped else None
    for subject in SUBJECTS:
        out["label"][subject] = tally(subjects[subject], label_rules)
    out["line_survivors"] = tally(subjects["line_script"], SURVIVOR_RULES)
    out["tree_whitespace"] = tally(subjects["tree_script"], WHITESPACE_RULES)
    out["tree_agreement"] = tally(subjects["tree_script"], GROUPS[2][2])
    return out


def macros(s: dict) -> dict:
    n = s["fixtures"]
    out = {
        "InvariantCount": len(INVARIANTS),
        "InvariantMappingOnlyCount": len(MAPPING_ONLY),
        "InvariantPaintingCount": len(s["painting_rules"]),
        "InvariantLabelCount": len(s["label_rules"]),
        "InvariantFixtures": latex_number(n),
        "InvariantHumanAllViolations": latex_number(s["human_all"][0]),
        "InvariantHumanAllFixtures": latex_number(s["human_all"][1]),
        "InvariantHumanAllScored": latex_number(s["human_all_fixtures"]),
        "InvariantHumanAllCleanPct": f"{100 - pct(s['human_all'][1], s['human_all_fixtures']):.1f}",
    }
    for subject in ("human", "line_script", "tree_script"):
        violations, fixtures = s["scripts"][subject]
        stem = STEMS[subject]
        out[f"Invariant{stem}Violations"] = latex_number(violations)
        out[f"Invariant{stem}Fixtures"] = latex_number(fixtures)
        out[f"Invariant{stem}FixturesPct"] = f"{pct(fixtures, n):.1f}"
        out[f"Invariant{stem}RulesBroken"] = s["broken_rules"][subject]
    for stem, key in (
        ("LineScriptSurvivor", "line_survivors"),
        ("TreeScriptWhitespace", "tree_whitespace"),
        ("TreeScriptAgreement", "tree_agreement"),
    ):
        violations, fixtures = s[key]
        out[f"Invariant{stem}Violations"] = latex_number(violations)
        out[f"Invariant{stem}Fixtures"] = latex_number(fixtures)
        out[f"Invariant{stem}FixturesPct"] = f"{pct(fixtures, n):.1f}"
    for subject in SUBJECTS:
        violations, fixtures = s["label"][subject]
        stem = STEMS[subject]
        out[f"InvariantLabel{stem}Violations"] = latex_number(violations)
        out[f"InvariantLabel{stem}Fixtures"] = latex_number(fixtures)
        out[f"InvariantLabel{stem}FixturesPct"] = f"{pct(fixtures, n):.1f}"
    return out


def table_tex(s: dict) -> str:
    """The paper's table: one row per family of rules, violations and violating changes for the
    hand paintings and for the two edit scripts painted as they stand."""

    def cell(value) -> str:
        if value is None:
            return "-- & --"
        return f"{latex_number(value[0])} & {latex_number(value[1])}"

    lines = [
        "% Auto-generated by research/analysis/invariants_report.py. Do not edit by hand -",
        "% regenerate: make invariants-report (from research/).",
        r"\begin{table*}",
        r"  \caption{The " + str(len(INVARIANTS)) + r" ground-truth invariants by family, and how "
        r"often three accounts of the same " + latex_number(s["fixtures"]) + r" changes break "
        r"them: the hand paintings, the line edit script of \texttt{diff} painted as it stands, "
        r"and the human tree mapping painted leaf by leaf as a tree edit script. \emph{V} is "
        r"violations and \emph{C} the changes with at least one. The scripts are not judged by "
        r"the " + str(len(MAPPING_ONLY)) + r" rules that read the mapping alone, which the last "
        r"row leaves out for all three.}",
        r"  \label{tab:invariants}",
        r"  \small",
        r"  \begin{tabular}{@{}lr>{\raggedright\arraybackslash}p{0.36\linewidth}rrrrrr@{}}",
        r"    \toprule",
        (
            r"    & & & \multicolumn{2}{c}{Hand painting} & \multicolumn{2}{c}{Line script} & "
            r"\multicolumn{2}{c}{Tree script} \\"
        ),
        r"    \cmidrule(lr){4-5}\cmidrule(lr){6-7}\cmidrule(l){8-9}",
        r"    Family & Rules & Example & V & C & V & C & V & C \\",
        r"    \midrule",
    ]
    for key, label, rules, example in GROUPS:
        cells = " & ".join(
            cell(s["groups"][(subject, key)]) for subject in ("human", "line_script", "tree_script")
        )
        lines.append(f"    {label} & {len(rules)} & {example} & {cells} \\\\ \\addlinespace")
    lines[-1] = lines[-1].replace(r" \addlinespace", "")
    lines += [
        r"    \midrule",
        r"    \emph{Rules that read a painting} & "
        + str(len(s["painting_rules"]))
        + " & & "
        + " & ".join(
            cell(s["scripts"][subject]) for subject in ("human", "line_script", "tree_script")
        )
        + r" \\",
        r"    \bottomrule",
        r"  \end{tabular}",
        r"\end{table*}",
    ]
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "--csv", type=Path, default=REPO_ROOT / "research/data/quality/invariant_gap_census.csv"
    )
    parser.add_argument("--plots-dir", type=Path, default=REPO_ROOT / "research/plots")
    args = parser.parse_args()

    datasets = fixture_datasets()
    rows = [r for r in read_rows(args.csv) if datasets.get(r["fixture"]) in PAPER_DATASETS]
    s = summarize(rows)
    values = macros(s)
    for name, value in values.items():
        print(f"{name:40} {value}")
    for subject, rows_of in by_subject(
        [r for r in rows if r["painted"] == "1" and r["embedded"] == "0"]
    ).items():
        per_rule = {i: tally(rows_of, (i,)) for i in judged_by(rows_of)}
        print(subject, {i: v for i, v in per_rule.items() if v[0]})
    (args.plots_dir / "invariants_table.tex").write_text(table_tex(s))
    lines = [
        "% Auto-generated by research/analysis/invariants_report.py. Do not edit by hand -",
        "% regenerate: make invariants-report (from research/).",
    ] + [f"\\newcommand{{\\{k}}}{{{v}}}" for k, v in values.items()]
    (args.plots_dir / "variables_invariants.tex").write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
