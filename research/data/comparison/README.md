# OmniDiff compared with other diff tools

How eight established diff tools, in eleven configurations, read the same real code changes as
OmniDiff, scored against the same human ground truth and timed on the same machine. Also OmniDiff
against a ground truth this project did not write. The [project README](../../../README.md) has the
overview.

[PROVENANCE.md](PROVENANCE.md) records every run: tool versions and how each was built, the traps
that produce a silently wrong number, and what each column means. Read it before quoting a figure.

**These figures are from 2026-11-25**, over the 1,705 fixtures the paper reports, on the laptop
(Intel Core i7-1370P, confined to four performance cores; see PROVENANCE.md), with that day's
OmniDiff. They are at the same corpus state as the paper. [OmniDiff's own current
figures](../../../src/test/README.md#goals-and-measurements) are measured differently, so the two
are not comparable.

## The tools

| Tool | Kind | Version |
|---|---|---|
| Unix `diff` | text, whole lines | system |
| `git diff`: Myers, minimal, patience, histogram | text, whole lines | system |
| BDiff | text, block-aware, reports changed characters | 0.1.0 |
| `nvim -d` | text, reports changed characters | Neovim 0.11.4, shipped defaults |
| GumTree | syntax tree | 4.0.0-beta8, built from source |
| difftastic | syntax tree | 0.69.0 |
| diffsitter | syntax tree | 0.9.0 |
| srcDiff | syntax tree, through srcML | `ef42b33` |

A tool is scored only on the languages it has a parser for, so each has its own number of
fixtures (n). Compare percentages across rows, not counts.

## Accuracy

For every line of both files: did the tool mark it changed, and does the hand painting agree? A
line is changed in the painting when any of its bytes is painted. Where a fixture has a Minimal and
a Maximal painting, a tool agrees when it agrees with either. A fixture is perfect when no line
disagrees. Line level is the one granularity every tool can be scored at, including the ones whose
output carries more detail. Only painted fixtures count: 1,478 of the 1,705.

| Tool | n | Perfect | At least 99% | 95-99% | Under 95% |
|---|---|---|---|---|---|
| Unix `diff` | 1,478 | 881 (60%) | 339 (23%) | 169 (11%) | 89 (6%) |
| `git` (Myers) | 1,478 | 888 (60%) | 334 (23%) | 170 (12%) | 86 (6%) |
| `git` (minimal, patience, histogram) | 1,478 | 888 (60%) | 334 (23%) | 170 (12%) | 86 (6%) |
| BDiff | 1,478 | 825 (56%) | 366 (25%) | 197 (13%) | 90 (6%) |
| `nvim -d` | 1,478 | 902 (61%) | 333 (23%) | 166 (11%) | 77 (5%) |
| diffsitter | 1,194 | 781 (65%) | 261 (22%) | 99 (8%) | 53 (4%) |
| difftastic | 1,451 | 989 (68%) | 263 (18%) | 122 (8%) | 77 (5%) |
| GumTree | 1,258 | 828 (66%) | 198 (16%) | 140 (11%) | 92 (7%) |
| srcDiff | 786 | 667 (85%) | 81 (10%) | 30 (4%) | 8 (1%) |
| **OmniDiff** | 1,478 | **1,193 (81%)** | 171 (12%) | 65 (4%) | 49 (3%) |

Against the lines the human *mapping* touches instead, over all 1,705 fixtures, OmniDiff is perfect
on 91%, srcDiff on 78% of what it parses, difftastic and GumTree on 61%, and the line tools on 57%.
The byte-level comparison with each painting is in the paper's `tab:painting-bytes`.

Three things to know when reading the table:

- **The ground truth is this project's own.** The people who built OmniDiff authored the
  mappings, and the fixtures are its test corpus. The oracle comparison below is the answer to
  that.
- **This is not OmniDiff's accuracy figure.** Line agreement is coarser than the node mapping and
  the byte-by-byte painting OmniDiff is held to, where its numbers are lower. See
  [src/test/README.md](../../../src/test/README.md#accurate).
- The four git algorithms differ on a handful of fixtures at this granularity, and not at all
  against the paintings.

`benchmark_accuracy.csv` also has node-level columns for the tools that report sub-line detail,
and byte-level columns against each painting; PROVENANCE.md explains them.

## Speed

Wall clock per fixture in milliseconds, three repeats, pooled. Every tool's time is its whole
process, OmniDiff's included: spawn, its own parse, its diff.

| Tool | p50 | p90 | p99 |
|---|---|---|---|
| Unix `diff` | 1.4 | 2.1 | 3.4 |
| BDiff, inside a running interpreter | 3.2 | 12.0 | 144 |
| diffsitter | 5.5 | 28.2 | 134 |
| `nvim -d` | 7.8 | 16.1 | 18.9 |
| **OmniDiff** | **11.9** | **67.6** | **360** |
| `git` (histogram) | 12.2 | 13.2 | 15.5 |
| `git` (Myers) | 12.9 | 14.4 | 16.8 |
| GumTree, inside a running JVM | 21.3 | 335 | 3,557 |
| srcDiff | 27.9 | 151 | 772 |
| difftastic | 37.4 | 89.3 | 406 |
| GumTree, per invocation | 300 | 670 | 3,961 |
| BDiff, per invocation | 405 | 472 | 609 |

Not comparable with the server's figures of 2026-09-26: a different machine, and OmniDiff was then
timed without its process start.

## Against an external oracle

Alikhanifard and Tsantalis' node-mapping benchmark (TOSEM 2025) holds 800 Defects4J bug fixes whose
mappings were built by hand from six tools' output, for Java, on Eclipse JDT trees.
[research/external/](../../external/README.md) describes it and how it is fetched.

OmniDiff over all 800 cases (2026-11-25): **99.26% precision, 98.95% recall** at statement and
sub-expression level. The reference figures from that paper are RefactoringMiner 3.0 at 99.7 /
99.3, GumTree 3.0 simple at 98.4 / 97.8 and GumTree 3.0 greedy at 97.5 / 93.1.

The comparison is not like for like. The paper's figures are on JDT trees and OmniDiff's on
tree-sitter trees, matched by byte span: 79% of the oracle's records resolve to a tree-sitter
node, and the rest are left out of OmniDiff's denominator. Quote that rate with the figures.

This project's own human mappings, on the 445 units they cover, agree with the oracle at 99.52%
precision and 99.54% recall.

## The files

| File | Holds | Written by |
|---|---|---|
| `benchmark_accuracy.csv` | per fixture and tool: mismatched lines and nodes against the mapping, lines and bytes against each painting, and a status | `make measure-tools-accuracy` |
| `benchmark_other.csv` | the same run with timings | `make measure-tools-timing` |
| `astdiff_oracle_defects4j.csv` | OmniDiff against the oracle, per compilation unit | `make measure-astdiff-oracle` |
| `astdiff_oracle_defects4j_human.csv` | the human mappings against the oracle | `make measure-astdiff-oracle-human` |

The targets live in `research/Makefile`. The tool runs need all six external tools installed and
their paths set (`GUMTREE_BIN`, `DIFFT_BIN`, `DIFFSITTER_BIN`, `BDIFF_PYTHON`, `NVIM_BIN`,
`SRCDIFF_BIN`): a tool whose variable is unset is skipped without an error. Both tool runs are
chunked and resume after a crash (`research/measure/benchmark_other_chunked.py`). `make
timing-report` regenerates the paper's tables and charts from these files.
