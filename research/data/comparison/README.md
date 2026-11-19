# OmniDiff compared with other diff tools

How eight established diff tools, in eleven configurations, read the same real code changes as
OmniDiff, scored against the same human ground truth and timed on the same machine. Also OmniDiff
against a ground truth this project did not write. The [project README](../../../README.md) has the
overview.

[PROVENANCE.md](PROVENANCE.md) records every run: tool versions and how each was built, the traps
that produce a silently wrong number, and what each column means. Read it before quoting a figure.

**These figures are not at the current corpus or engine.** The tool comparison was last run on
2026-09-26, over the 1,217 fixtures the paper reported then, on the server (Intel Xeon E3-1275
v5), with the OmniDiff of that day. The fixture corpus has grown since and
[OmniDiff's own current figures](../../../src/test/README.md#goals-and-measurements) are measured
differently, so the two are not comparable.

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

For every line of both files: did the tool mark it changed, and does the human mapping agree? A
fixture is perfect when no line disagrees. Line level is the one granularity every tool can be
scored at, including the ones whose output carries more detail.

| Tool | n | Perfect | At least 99% | 95-99% | Under 95% |
|---|---|---|---|---|---|
| Unix `diff` | 1,217 | 713 (59%) | 249 (20%) | 160 (13%) | 95 (8%) |
| `git` (Myers) | 1,217 | 718 (59%) | 248 (20%) | 159 (13%) | 92 (8%) |
| `git` (minimal) | 1,217 | 718 (59%) | 248 (20%) | 159 (13%) | 92 (8%) |
| `git` (patience) | 1,217 | 717 (59%) | 247 (20%) | 160 (13%) | 93 (8%) |
| `git` (histogram) | 1,217 | 717 (59%) | 247 (20%) | 161 (13%) | 92 (8%) |
| BDiff | 1,217 | 688 (57%) | 254 (21%) | 179 (15%) | 96 (8%) |
| `nvim -d` | 1,217 | 728 (60%) | 247 (20%) | 156 (13%) | 86 (7%) |
| diffsitter | 891 | 482 (54%) | 204 (23%) | 115 (13%) | 90 (10%) |
| difftastic | 1,182 | 755 (64%) | 197 (17%) | 135 (11%) | 95 (8%) |
| GumTree | 1,005 | 621 (62%) | 158 (16%) | 128 (13%) | 98 (10%) |
| srcDiff | 519 | 409 (79%) | 68 (13%) | 32 (6%) | 10 (2%) |
| **OmniDiff** | 1,217 | **1,094 (90%)** | 66 (5%) | 33 (3%) | 24 (2%) |

Three things to know when reading the table:

- **The ground truth is this project's own.** The people who built OmniDiff authored the
  mappings, and the fixtures are its test corpus. The oracle comparison below is the answer to
  that.
- **This is not OmniDiff's accuracy figure.** Line agreement is coarser than the node mapping and
  the byte-by-byte painting OmniDiff is held to, where its numbers are lower. See
  [src/test/README.md](../../../src/test/README.md#accurate).
- The four git algorithms differ on a handful of fixtures at this granularity.

`benchmark_accuracy.csv` also has node-level columns for the tools that report sub-line detail;
PROVENANCE.md explains the three node denominators.

## Speed

Wall clock per fixture in milliseconds, three repeats, pooled. An external tool's time is its whole
process: spawn, its own parse, its diff. OmniDiff's is its parse and its diff with no process
start, which every other tool pays.

| Tool | p50 | p90 | p99 |
|---|---|---|---|
| Unix `diff` | 1.9 | 2.4 | 4.1 |
| `git` (Myers) | 2.0 | 2.7 | 5.4 |
| `git` (histogram) | 1.9 | 2.6 | 5.3 |
| BDiff, inside a running interpreter | 3.6 | 14.0 | 277 |
| **OmniDiff** | **4.6** | **77.4** | **420** |
| diffsitter | 5.4 | 38.4 | 189 |
| GumTree, inside a running JVM | 14.0 | 435 | 5,389 |
| srcDiff | 31.0 | 230 | 818 |
| `nvim -d` | 33.6 | 70.9 | 150 |
| difftastic | 35.0 | 114 | 637 |
| BDiff, per invocation | 307 | 331 | 605 |
| GumTree, per invocation | 535 | 1,027 | 6,571 |

The machine was mostly idle, not verifiably idle throughout: the maxima are noisy, the medians
are not.

## Against an external oracle

Alikhanifard and Tsantalis' node-mapping benchmark (TOSEM 2025) holds 800 Defects4J bug fixes whose
mappings were built by hand from six tools' output, for Java, on Eclipse JDT trees.
[research/external/](../../external/README.md) describes it and how it is fetched.

OmniDiff over all 800 cases (2026-10-22): **99.43% precision, 98.93% recall** at statement and
sub-expression level. The reference figures from that paper are RefactoringMiner 3.0 at 99.7 /
99.3, GumTree 3.0 simple at 98.4 / 97.8 and GumTree 3.0 greedy at 97.5 / 93.1.

The comparison is not like for like. The paper's figures are on JDT trees and OmniDiff's on
tree-sitter trees, matched by byte span: 78% of the oracle's records resolve to a tree-sitter
node, and the rest are left out of OmniDiff's denominator. Quote that rate with the figures.

This project's own human mappings, on the 440 units they cover, agree with the oracle at 99.62%
precision and 99.64% recall.

## The files

| File | Holds | Written by |
|---|---|---|
| `benchmark_accuracy.csv` | per fixture and tool: mismatched lines and nodes, and a status | `make measure-tools-accuracy` |
| `benchmark_other.csv` | the same run with timings | `make measure-tools-timing` |
| `astdiff_oracle_defects4j.csv` | OmniDiff against the oracle, per compilation unit | `make measure-astdiff-oracle` |
| `astdiff_oracle_defects4j_human.csv` | the human mappings against the oracle | `make measure-astdiff-oracle-human` |

The targets live in `research/Makefile`. The tool runs need all six external tools installed and
their paths set (`GUMTREE_BIN`, `DIFFT_BIN`, `DIFFSITTER_BIN`, `BDIFF_PYTHON`, `NVIM_BIN`,
`SRCDIFF_BIN`): a tool whose variable is unset is skipped without an error. `make timing-report`
regenerates the paper's tables and charts from these files.
