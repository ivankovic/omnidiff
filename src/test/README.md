# Testing

How OmniDiff is tested: the corpus of real changes it is measured on, the three goals it is held
to and where it stands against each, and how to run the tests and the measurements. The
[project README](../../README.md) has the overview, and
[CONTRIBUTING.md](../../CONTRIBUTING.md) the Makefile targets and what CI runs.

## What is here

```
src/test/
  helper.rs, helper/     shared test helpers: handmade test data, and the readers and checkers
                         of the human ground truth (mappings, paintings, verdicts, invariants)
  fixtures.rs, fixtures/ one test per fixture: a real diff held to its ground truth
  data/diffs/            code changes, a directory per fixture, in five datasets: small, full
                         and stratified (sampled from the corpus), defects4j (an external
                         oracle's cases) and handmade (regression tests written for OmniDiff)
  data/pictures/, archives/, fonts/, cursors/, catalogs/, documents/
                         changes to other content, judged by hand
  data/changesets/       changes that span several files
```

A fixture directory holds the two versions (`before.<ext>.test`, `after.<ext>.test`), a
`README.md` with its provenance and license, and the ground truth a human authored for it: a node
mapping (`human_mapping.json`), with a painting of every byte for many, or a verdict for other
content. `make human-solver` opens the tool they are authored in. The corpus is real changes
sampled from open-source repositories; `list_of_repositories.csv` at the repository root names
them, and [research/](../../research/README.md) has the sampling pipeline.

## Goals and measurements

OmniDiff is held to three goals: fast, robust and accurate. Each is a number measured over this
corpus, and the figures below are regenerated, not remembered.

### Fast

OmniDiff's goal is:

* **A median diff in 100ms or less.**
* **A 99th-percentile diff in 1000ms or less.**

Both are met. Over the 2,302 fixtures in `src/test/data/diffs/`, measured 2026-11-07 on a laptop,
an Intel Core i7-1370P (6 performance and 8 efficiency cores, 20 threads) with 32 GB RAM: **p50
4.4ms, p90 35.7ms, p99 181ms**, slowest 507ms. **100ms is the 97.5th percentile** — 57 fixtures
take longer than that, and none takes longer than a second.

Times belong to the machine that measured them, so each machine keeps its own (see
`research/data/machines.csv`): the Intel Xeon E3-1275 v5 server that measured the research
figures takes about 2.5 times as long per fixture. Runtime is reported, never gated: `make
check-quality` prints this distribution against the machine's own committed baseline on every
push, and warns when the runtime is more than twice the baseline's.

Other content is timed over pairs sampled from the same corpus, decoding included - unpacking an
archive, drawing a font's glyphs or a PDF's pages - measured 2026-11-07 on the same laptop:

| Content | Pairs | p50 | p90 | p99 | Under 100ms |
|---|---|---|---|---|---|
| Pictures | 230 | 2.1ms | 43ms | 589ms | 96.1% |
| Cursors | 22 | 0.6ms | 1.3ms | 18ms | 100% |
| Fonts | 100 | 8.4ms | 60ms | 1,109ms | 93% |
| Archives | 100 | 3.3ms | 160ms | 1,878ms | 87% |
| Message catalogs | 81 | 0.4ms | 2.1ms | 5.1ms | 100% |
| PDFs | 100 | 89ms | 473ms | 1,330ms | 55% |

Each figure is the median of three runs. Archives, fonts and PDFs miss the 99th-percentile goal:
the slowest archives hold thousands of changed members, and a font or a PDF draws every changed
glyph or page. Only 55% of PDFs take under 100ms. `make
benchmark-content` measures this and writes `research/data/performance/content_speed/<machine>.csv`.

### Robust

OmniDiff's goal is to process 100% of all commits.

The full test dataset holds the git commit history of about 7,500 open-source git repositories,
as available on the main branch. This list of repositories comes from the Gentoo Linux
distribution. Find it in `list_of_repositories.csv`.

Measured over every modified code file in the most recent 50 commits of each of those
repositories - 442,530 readable before/after pairs in the 25 languages OmniDiff had a grammar for at
the time (SQL, the 26th, came later), diffed with no size cap under
a 120-second budget and a 6 GB memory cap per process - **442,322 (99.95%) completed
successfully.** 96 pairs ran past the budget and 112 past the memory cap; the latter are twenty
files, nineteen of them generated or embedded - tree-sitter parser tables, codegen, minified
bundles, a PNG as a C array - and one commit of a 40,000-line single-header C++ library. Given
24 GB and 300 s, 11 of the 28 files that first hit the cap complete. The run, its harness and every
pair that did not complete are documented in `research/data/performance/PROVENANCE.md`.

Beyond code, over every file changed in the same window - 2,018,010 changes in 7,352
repositories, pictures, fonts and archives included - **98.8% are diffed as text or by their
content** (98.2% counting each repository at most 1,000 times, so that a few giant ones do not
decide it; 6,481 repositories entirely). What is left is mostly text in legacy encodings, compiled
objects, audio, and EOT fonts compressed with MicroType Express. The census is documented in
`research/data/corpus_stats/PROVENANCE.md`, and
[what real-world changes look like](../../research/data/corpus_stats/README.md) has the rest of
what that corpus shows.

### Accurate

OmniDiff must match a human's own reading of a change, measured against the hand-authored
ground-truth mappings in `src/test/data/diffs/`:

* **90% of test cases with zero mismatched bytes.**
* **99% of test cases with at most 1% of bytes mismatched.**

Neither is met yet. 1,479 of the 2,302 fixtures carry a hand-painted ground truth - every byte of
both files labelled with what a human says happened to it - and OmniDiff's own highlighting is
compared against it byte by byte, under each of its two highlighting presets (`--full`, which keeps
brackets, separators and leading whitespace, and `--minimal`, which drops them):

| preset | zero mismatched bytes | at most 1% mismatched | mismatched bytes, whole corpus |
|---|---|---|---|
| `--full` | **873 (59.0%)** | **1,291 (87.3%)** | 112,321 of 75.9M (0.15%) |
| `--minimal` | **1,021 (69.0%)** | **1,354 (91.5%)** | 55,930 of 75.9M (0.07%) |

The whole-corpus rate is far below 1% because most of each file is unchanged and nobody gets that
wrong; the per-test-case numbers are the ones that count, since a reader meets the mistakes one
diff at a time. Most of what is left is not in the matching: rendering the human's own node
mapping still disagrees with the painting on 92% of the `--full` bytes and 92% of the `--minimal`
ones, so the highlighting rules own the gap more than the matcher does.

`make update-painting-attribution` measures this and writes one row per fixture and preset to
`research/data/quality/painting_attribution.csv`, which the table is counted from;
`make check-painting-attribution` fails if any fixture gets worse.

Other content is judged rather than painted: a human names how much each change is (invisible,
imperceptible, artifacts, edited, redrawn or replaced, see
[supported content](../../docs/README.md#supported-content))
and OmniDiff's verdict is compared with theirs. A test case is perfect when every verdict matches,
and near-perfect when every one is at most one level off:

| Content | Judged | Perfect | Near-perfect |
|---|---|---|---|
| Pictures | 131 | **91 (69%)** | **119 (91%)** |
| Cursors | 13 | **11 (85%)** | **13 (100%)** |
| Fonts | 11 | **9 (82%)** | **11 (100%)** |
| Archives | 23 | **20 (87%)** | **20 (87%)** |
| Message catalogs | 12 | **10 (83%)** | **10 (83%)** |
| PDFs | 4 | **4 (100%)** | **4 (100%)** |

`make benchmark-content` counts these too.

## Running the tests

Run automated tests frequently during coding (`make test`, or `make test-quick` in the edit
loop; [CONTRIBUTING.md](../../CONTRIBUTING.md#makefile-targets) describes both).

Diff quality and diff speed are measured separately (see "Quality" and "Speed" below). Check both
on demand, and always before a release. `make deploy` already gates on quality automatically (see
`make check-quality` in the Makefile).

### Automated tests

Each file in `src/` ends with its own test module, as is typical in Rust. These tests must cover
both the happy path and corner cases. Use `src/test/helper.rs` to get handmade, high-quality test
data.

**Per-file unit tests must run in under 1 second.**

The Python under `research/analysis/` and `scripts/` has its own unit tests in `research/tests/`,
run by `make test-python` (part of `make test` and of CI's python job). They cover the pure
functions the report scripts are built from; the scripts' shared helpers live in
`research/analysis/_common.py`.

`src/test/` also holds slower, fixture-driven tests, for example `src/test/fixtures/`.
These tests check real diffs against a human-verified ground truth. **These tests must run in
under 5 seconds.**

### Coverage

`make coverage` reports which of this repository's lines the suite executes. `cargo-llvm-cov`
drives `cargo nextest` directly under `--all-features`, so it measures exactly the suite `make
test` runs. It writes a browsable report to `target/llvm-cov/html/index.html` and prints a
per-area table:

| area | lines | |
| --- | --- | --- |
| `src/showcase/` - showcase data | 374/384 | 97.4% |
| `src/diff/` - the engine | 12514/12861 | 97.3% |
| `src/test/` - fixture helpers | 18600/19231 | 96.7% |
| `src/code/` - parsing, metadata | 1625/1719 | 94.5% |
| `src/tui/` - viewer, headless | 7789/8892 | 87.6% |
| `src/stats/` - sampling, git | 566/662 | 85.5% |
| `src/` - entry points, integrations | 985/1372 | 71.8% |
| `src/bin/` - dev tools | 10801/18135 | 59.6% |
| **product (everything but `src/bin/`)** | **42453/45121** | **94.1%** |
| everything | 53254/63256 | 84.2% |

Measured 2026-09-26 over 4777 tests. The engine and the dev tools are deliberately held to
different standards: `src/bin/` is samplers, benchmark harnesses and `human_solver`, several of
which exist to be run once and read.

**Not a CI gate.** It costs about ten minutes and 6GB peak, since it rebuilds the workspace with
instrumentation - and a threshold mostly teaches people to write tests that touch lines. At 97%
the engine would never be what tripped a floor; only the dev tools would.

One test knows it is being measured: `rust_completely_unrelated_main_files_resolves_fast` asserts
a five-second wall-clock bound, which instrumentation blows through, so it skips that half of
itself when `LLVM_PROFILE_FILE` or `CARGO_LLVM_COV` is set. It is the only wall-clock assertion in
the library suite; add the same guard if another appears.

The project README's badge reads `research/data/coverage/badge.json`, which `make coverage` rewrites.
It is therefore only as current as the last run somebody committed - re-run and commit it when
the number has moved enough to matter.

### Test dependencies

**No mocks.** Mocks block testing through the interface, and mocks are brittle.

Use the real implementation where possible.

Where the real implementation is not possible, for example for filesystem or database access, use
a fake in-memory implementation.

Install `jj` (`cargo install jj-cli`, or see jj's own install docs) if you touch
`src/jj_configure.rs`. That module's claims about how jj invokes
a diff tool - directory trees by default, file pairs with extensions preserved under
`diff-invocation-mode = "file-by-file"` - were verified empirically against jj 0.44.0, and should
be re-verified the same way rather than assumed.

### Quality

Run `make benchmark-quality`. This command diffs every fixture in `src/test/data/diffs/` that has a
human-verified ground truth mapping. It reports how many nodes each fixture gets wrong. Use this
output to see whether a change made diffs better or worse.

[Accurate](#accurate) above states the painting targets, compared byte by byte under both `--full`
and `--minimal`.

`make update-painting-attribution` measures them, one row per fixture and preset in
`research/data/quality/painting_attribution.csv`, and `make check-painting-attribution` fails CI if
any fixture gets worse. Each row carries two numbers that answer different questions:
`real_bytes` renders omnidiff's own mapping, which is what a reader sees, and `renderer_bytes`
renders the *human* tree mapping, so it holds no matcher error at all - what is left there only a
change to `diff::text` can fix. Steer painting work by the second. `painting_disagreement_detail`
(`src/test/helper/human_mapping/tests/exploratory.rs`, `MAPPING=human` for the second column)
prints the disagreeing runs of one fixture.

The tree mapping behind the painting has targets of its own:

* **90% of test cases with zero mismatched visible nodes.**
* **99% of test cases with at most 1% of visible nodes mismatched.**

Both are stated in *visible* nodes - the ones carrying text of their own, per
`omnidiff::diff::nodes::is_structurally_visible` - not all AST nodes. A wrongly-matched `block` or
`argument_list`, whose every readable byte belongs to a child, is not the same defect as a
wrongly-matched identifier. About 68% of nodes are visible corpus-wide, so the two counts differ.
The benchmark prints both (`Mismatches` / `Vis Mism`), and every clamped `fixtures` test
pins both (`assert_matches_human_mapping_within_limit(name, total, visible)`, which fails if
*either* limit is exceeded).

**Visibility is a property of the tree and the source, never of a diff.** Deriving it from the
renderer - does `diff::text::ranges` emit a span for this node - makes both the numerator and the
denominator move with the algorithm, so a diff that renders coarsely has almost nothing it can get
visibly wrong. `is_structurally_visible`'s doc comment has the reasoning in full.

### Speed

[Fast](#fast) above states the goals (p50 <= 100ms, p99 <= 1000ms). Both are met with
room to spare, so a change may cost some speed but not an order of magnitude. `make
benchmark-quality` prints the total runtime and the time per fixture; `make check-quality` prints
the p50/p90/p99 distribution against the committed baseline, and warns, never fails, on a runtime
above 2x the baseline.

Times are only comparable on one machine, so each machine keeps its own baseline. A machine is
registered in `research/data/machines.csv` by its CPU's model name, and `scripts/machine.py` tells
the Makefile which one it is on; `MACHINE=<name>` overrides. On a machine new to the project, add
its row, then run `make update-timing-baseline` (and `make benchmark-content` for the content
figures). A machine without a baseline, such as a CI runner, skips the runtime comparison.
