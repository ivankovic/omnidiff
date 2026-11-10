# Provenance

Measured against the human-authored ground-truth fixtures in `src/test/data/diffs/` (NOT the
sampled corpus in `../samples/` - quality is scored on hand-verified mappings, so re-sampling the
corpus does not stale these files). `quality_baseline.csv` is updated only via
`make update-quality-baseline` at the repository root.

`timing/` holds the gate's wall-clock baselines, one pair of files per machine registered in
`../machines.csv`: `<machine>.csv`, each fixture's `elapsed_ms`, and `<machine>.txt`, the
ms/fixture `make check-quality` warns against. Written by `make update-quality-baseline` or, timing
alone, `make update-timing-baseline`, on that machine. `server.*` are the times that were the
`elapsed_ms` column of `quality_baseline.csv` and `quality_baseline.txt` until 2026-10-11, split out
unchanged.
`laptop.*` were first recorded 2026-10-11 by `make update-timing-baseline`, on AC power with the
`performance` platform profile, idle: 16.5 ms/fixture against the server's 41.3, a median fixture
0.42x the server's time.

`human_mapping_analysis.csv` also defines *which* fixtures are in scope for
`analysis/ambiguity_report.py` (the paper's RA1.1): that script reads the `human_mapping.json`
files directly but restricts itself to the names listed here, so every number it produces
describes one corpus state. Refresh order after adding fixtures: `analyze_human_mappings --csv`, then
`make ambiguity-report`.

## Refresh of 2026-09-26

`optimal_solutions_benchmark.csv` (`benchmark_optimal_solutions --csv`) and
`human_mapping_analysis.csv` (`analyze_human_mappings --csv`) re-run, in that order, alongside a full `benchmark_other` refresh that added
srcDiff, so the paper's comparison, node accuracy, ambiguity and rendering blocks describe one corpus
state again. Annotation had moved on since 2026-09-16: 161 more fixtures in scope, every one of
them a Defects4J unit (274 solved, from 113).

| | fixtures | with a `human_mapping.json` |
|---|---|---|
| `handmade` | 63 | 62 |
| `small` (Curated) | 220 | 220 |
| `full` (Full) | 232 | 232 |
| `stratified` | 491 | 491 |
| `defects4j` | 996 | 274 |
| **total** | **2002** | **1279** |

The paper's scope is **1217** fixtures (1056 before). Over them omnidiff maps 10,325,792 of
10,333,777 nodes (99.92%), and 7,030,055 of 7,035,541 visible nodes; `paper_variables.py`'s
authored CORPUS block carries these totals.

The trigger was a crash, which is worth knowing about: `benchmark_other_report.py` joins its
per-dataset node columns against this CSV, and a fixture solved after this CSV was written reads
`-` there. The refreshed comparison CSV scored 143 such fixtures, and the report died on
`int('-')`. A crash was the right outcome - the other reading was a comparison section over 1217
fixtures inside a paper whose every other block said 1056.

## Refresh of 2026-10-17

`optimal_solutions_benchmark.csv` and `human_mapping_analysis.csv` re-run, in that order, on the
laptop, with the CORPUS block, the ambiguity, rendering and shape reports and both AST-diff oracle
runs. Unlike 2026-09-26 this is a partial refresh: the tool comparison (`../comparison/`
`benchmark_other.csv` and `benchmark_accuracy.csv`) and the fixture robustness run were not re-run,
because the external tools were not installed on the laptop and the timings belong to the server.
They still describe 1,217 fixtures, and the paper says so in footnote `fn:in-progress`.
282 more fixtures in scope: 221 stratified and 61 Defects4J.

| | fixtures | with a `human_mapping.json` |
|---|---|---|
| `handmade` | 63 | 62 |
| `small` (Curated) | 220 | 220 |
| `full` (Full) | 232 | 232 |
| `stratified` | 712 | 712 |
| `defects4j` | 996 | 335 |
| **total** | **2223** | **1561** |

The paper's scope is **1499** fixtures. Over them omnidiff maps 16,322,226 of 16,329,845 nodes
(99.95%), and 11,013,577 of 11,018,678 visible nodes.

## Refresh of 2026-10-22

The same partial refresh as 2026-10-17, on the laptop: `optimal_solutions_benchmark.csv` and
`human_mapping_analysis.csv` re-run in that order, with the CORPUS block, the ambiguity, rendering and
shape reports and both AST-diff oracle runs. The tool comparison and the fixture robustness run are
still at the 1,217 fixtures of 2026-09-26. 184 more fixtures in scope: 79 stratified and 105
Defects4J.

| | fixtures | with a `human_mapping.json` |
|---|---|---|
| `handmade` | 63 | 62 |
| `small` (Curated) | 220 | 220 |
| `full` (Full) | 232 | 232 |
| `stratified` | 791 | 791 |
| `defects4j` | 996 | 440 |
| **total** | **2302** | **1745** |

The paper's scope is **1683** fixtures. Over them omnidiff maps 18,597,127 of 18,605,165 nodes
(99.96%), and 12,569,053 of 12,574,430 visible nodes.

## Refresh of 2026-11-07

The same partial refresh as 2026-10-22, on the laptop: `optimal_solutions_benchmark.csv` and
`human_mapping_analysis.csv` re-run in that order, with the CORPUS block and the ambiguity,
rendering and shape reports. Not re-run: the tool comparison and the fixture robustness run (still
at the 1,217 fixtures of 2026-09-26), and the two AST-diff oracle runs, which need Java and the
pinned RefactoringMiner, neither installed on the laptop - they stay at the 1,683 fixtures of
2026-10-22. 4 more fixtures in scope, all Defects4J; the rest of the change since 2026-10-22 is
repainted and newly painted ground truth (1,479 fixtures carry a painting, from 1,419).

| | fixtures | with a `human_mapping.json` |
|---|---|---|
| `handmade` | 63 | 62 |
| `small` (Curated) | 220 | 220 |
| `full` (Full) | 232 | 232 |
| `stratified` | 791 | 791 |
| `defects4j` | 996 | 444 |
| **total** | **2302** | **1749** |

The paper's scope is **1687** fixtures. Over them omnidiff maps 18,616,022 of 18,623,801 nodes
(99.96%), and 12,581,856 of 12,587,060 visible nodes. `optimal_solutions_benchmark.csv` is the
third of three runs; the README's code-speed figures are each fixture's median `elapsed_ms` over
the three.

## Refresh of 2026-11-10

After the accuracy and speed commit `b87928e8` (repaired fixtures, new matching and painting rules,
the three-case painting grading), on the laptop: `optimal_solutions_benchmark.csv` and
`human_mapping_analysis.csv` re-run in that order, with the CORPUS block and the ambiguity,
rendering and shape reports. Not re-run, for want of the external tools and of the server: the tool
comparison and the fixture robustness run (still at the 1,217 fixtures of 2026-09-26) and the two
AST-diff oracle runs (still at the 1,683 fixtures of 2026-10-22). 12 more fixtures in scope, the SQL
fixtures of 2026-11-09, all in `full`.

| | fixtures | with a `human_mapping.json` |
|---|---|---|
| `handmade` | 63 | 62 |
| `small` (Curated) | 220 | 220 |
| `full` (Full) | 244 | 244 |
| `stratified` | 791 | 791 |
| `defects4j` | 996 | 444 |
| **total** | **2314** | **1761** |

The paper's scope is **1699** fixtures. Over them omnidiff maps 18,631,192 of 18,638,860 nodes
(99.96%), and 12,590,174 of 12,595,261 visible nodes; 1,427 fixtures have no visible node mapped
differently. `analyze_human_mappings` now parses one fixture at a time, so it runs under a 10 GB
memory cap.

Three files joined this directory the same day, each written by an `#[ignore]`d test or a script
and read by one report of `research/analysis/`:

- `preset_census.csv` - what the `Full` hand painting adds to the `Minimal` one, per fixture with
  both: the `preset_census` test (`make measure-presets`), read by `preset_report.py`.
- `invariant_gap_census.csv` - the violation count of each of the 31 ground-truth invariants for
  five accounts of each fixture (the hand paintings, GNU diff's line script and the human tree
  mapping painted as they stand, and the human and omnidiff mappings through the renderer): the
  `invariant_gap_census` test (`make measure-invariants`), read by `invariants_report.py`.
- `dataset_growth.csv` - the date each fixture's ground truth and first painting were committed,
  from git history: `dataset_growth.py --measure` (`make measure-dataset-growth`).

## Scope

The paper's scope is `_common.PAPER_DATASETS`: every dataset above except `handmade`, which stays
a regression suite. Only fixtures with a `human_mapping.json` are ever scored; the unsolved
Defects4J directories are invisible to every report. Which Defects4J units are solved is how far
annotation has reached, not a draw - do not read a Defects4J figure as an estimate over Defects4J.

## `content_accuracy.csv` (2026-11-09)

The introductory paper's accuracy figures for content other than code: one row per judged fixture
in `src/test/data/<family>/` (pictures, archives, cursors, fonts, catalogs, documents), with how
many verdicts the human gave on it (the pair's and each judged member's) and whether the engine's
verdicts are all equal to them (`perfect`) and all within one level (`near_perfect`). Written by
`benchmark_content --accuracy-csv` (`cargo run --release --features stats --bin benchmark_content
-- --samples <an empty directory> --accuracy-csv research/data/quality/content_accuracy.csv`; the
empty samples directory skips the timing half, which `make benchmark-content` writes to
`../performance/content_speed/`), on the laptop at `d577f89a`. 194 fixtures judged: 131 pictures,
23 archives, 13 cursors, 11 fonts, 12 catalogs, 4 documents; the same counts and rates the root
README prints. These are judged, not painted: the fixtures are the content ground truth, as
`src/test/data/diffs/` is the code one, and a re-run after adding or re-judging one rewrites this
file.

## Other files in this directory

- `content_accuracy.csv` - see above.
- `quality_baseline.csv` - the per-fixture accuracy gate of `make check-quality`, written by
  `make update-quality-baseline` as a projection of the `fixtures` stubs' limits.
- `painting_attribution.csv` - the painting gate of `make check-painting-attribution`, one row per
  fixture and graded case, written by `make update-painting-attribution` (the
  `painting_failure_census` test). The `preset` column is `minimal` or `full`, or `single` for a
  fixture whose lone painting names no preset: both presets are rendered against that one painting,
  and a byte counts as mismatched when either disagrees with it (`GradedCase::Single`).
- `mismatch_census.csv` - every mismatch against the human mapping, classified by operation,
  reason and node kind, written by the `mismatch_census` test.
- `convention_census.csv` - leaves whose text survived but whose painting differs between
  fixtures, written by the `cross_fixture_convention_census` test.
- `nm_candidates.csv` - every subtree omnidiff deletes or inserts whole while an identical twin on
  its side is matched: what a pass attaching leftovers to N:M groups could add, labelled with the
  human's verdict on each. Written by the `nm_candidate_census` test.
- `kind_mismatches.csv` - every ground-truth pair whose two nodes differ in kind, written by
  `analyze_human_mappings --kind-mismatches`.
- `kind_invariant_candidates.csv` - the delete+insert leaf pairs the kind invariants would force
  to match, written by `analyze_human_mappings --kind-invariant-cost`.
- `ground_truth_conflicts.md` - the open ground-truth conflicts: conventions the fixtures answer
  both ways, single fixtures that hold back a general change, and the fixtures whose 2026-10-13
  `analysis.md` flags their ground truth. Written by hand; a settled conflict moves to its
  *Resolved* section.

The four census tests live in `src/test/helper/human_mapping/tests/exploratory.rs`, are
`#[ignore]`d, and run with `cargo test --release --lib --features test-fixtures <name> --
--ignored`.
