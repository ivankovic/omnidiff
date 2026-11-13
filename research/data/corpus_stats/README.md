# What real-world changes look like

What a diff tool is asked about, measured on real repositories and independent of any diff
algorithm: what the files are, what kind of file a commit changes, and how large a change is. These
figures set OmniDiff's goals: which content it must diff, and how large an input it must stay
fast on. The [project README](../../../README.md) has the overview.

Every figure here has a date and a command behind it in [PROVENANCE.md](PROVENANCE.md), with the
traps each measurement met. Quote a number from here together with its date.

## The corpus

The Full List: the repositories of the packages in the Gentoo Linux distribution,
`list_of_repositories.csv` at the repository root. 7,444 of them cloned (2026-09-07), holding
7,045,754 files. Change figures are over the most recent 50 non-merge commits of each repository.
The cap matters: without it, `torvalds-linux.git` alone carries more than half of the corpus's
reachable commits.

## What the files are

Files at rest, by what OmniDiff's classifier (`code::tip`) says they are (2026-09-24):

| Category | Files | Share |
|---|---|---|
| Code | 4,688,419 | 66.5% |
| Data | 1,593,317 | 22.6% |
| Unknown | 434,318 | 6.2% |
| Configuration | 176,475 | 2.5% |
| Documentation | 153,225 | 2.2% |

JSON, XML and YAML count as Code, because OmniDiff has grammars for them.

## How large a code file is

| Per code file | p50 | p90 | p99 | Largest |
|---|---|---|---|---|
| Lines | 43 | 375 | 2,511 | 13M |
| Bytes | 2,238 | 17,842 | 132,967 | 101M |
| Syntax-tree nodes | 407 | 3,791 | 27,546 | 23.6M |

Lines and bytes are over every code file. Nodes are over the 3,586,682 (76.5%) that parse: the
rest are in a language without a grammar, flagged as generated, or empty. A file has about one
node per five bytes. The largest files are generated data, not code anyone wrote.

## What a commit changes

Over 247,076 commits and 2,021,166 changed files (2026-10-02):

- **9.1% of changed files are binary** by git's own verdict. For these, `git diff` prints only
  that they differ.
- By changed files, Code is 43.9%, Data 41.3%, Documentation 6.6%, Unknown 5.3% and
  Configuration 2.9%. A few repositories dominate the Data share: UFO font sources are 411,000
  changes from 9 repositories. To say how common a format is, count the repositories that change
  it, not the changes.
- The most widespread binary format is PNG, changed in 629 repositories (8.4%), then PDF (109),
  GIF (95), JPEG (89), `.gz` (88), ICO (81) and TrueType fonts (81). Pictures are 39% of binary
  changes.
- The most widespread text formats without a grammar in OmniDiff are Markdown (changed in 65.3% of
  repositories), `.txt` (36.3%), TOML (13.7%), reStructuredText (10.5%) and gettext `.po` (4.9%).

**OmniDiff diffs 98.8% of these changes as text or by their content** (2026-10-04, over 2,018,010
changes in 7,352 repositories), 98.2% when each repository counts at most 1,000 times, and every
change in 6,481 of the repositories. What is left is mostly text in legacy encodings, compiled
objects, audio, and EOT fonts compressed with MicroType Express.

## How large an edit is

Over 435,239 modifications of code files in 114,817 commits (2026-09-07). File creations and
deletions are not edits and are left out: 75.2% of code file changes are modifications.

| | p50 | p90 | p99 |
|---|---|---|---|
| Lines changed per file edit | 5 | 63 | 989 |
| Lines changed per commit | 11 | 139 | 2,712 |
| Files changed per commit | 1 | 5 | 39 |
| Share of the file rewritten | 3.2% | 33.3% | 100% |

Most edits are small: 64.8% of file edits change fewer than ten lines, and 59.9% rewrite less
than 5% of the file. The tail is long: one file edit in a hundred changes about a thousand lines
or more.

![Six cumulative curves on logarithmic axes. Top row: lines, bytes and syntax-tree nodes per code
file. Bottom row: lines changed per file edit, lines changed per commit, and the share of the file
rewritten. Each curve marks its 50th, 90th and 99th percentile, the values in the tables
above.](../../plots/corpus_shape.png)

## The files

| File | Holds | Written by |
|---|---|---|
| `code_percentiles.csv` | size percentiles per language | `analysis/file_stats.py` |
| `code_file_size_distribution.csv` | the whole size distribution, one row per distinct size | `analysis/file_stats.py` |
| `top_node_kinds_by_language.md` | the most common syntax-tree node kinds per language | `analysis/file_stats.py` |
| `code_pair_diff_stats_rust.csv` | size and lines changed of the sampled Rust pairs | `analysis/code_pair_diff_stats.py` |
| `change_census.csv` | changes per extension: category, binary share, lines, repositories | `make measure-change-census MODE=full` |
| `content_census.csv`, `content_census_repositories.csv` | how OmniDiff diffs each change: as text, by content, or not at all | `make measure-content-census MODE=full` |
| `edit_shape.csv` | edit size per language | `make measure-edit-shape MODE=full` |
| `edit_shape_distribution.csv` | the whole edit-size distributions | `make measure-edit-shape MODE=full` |

The `make` targets live in `research/Makefile` and need the corpus cloned under
`/var/tmp/research/full/` (`make fetch MODE=full`, hours). [research/README.md](../../README.md)
describes the pipeline.
