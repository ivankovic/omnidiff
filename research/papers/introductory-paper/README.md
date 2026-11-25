# Introductory paper

"OmniDiff: Content-Aware Diffing", an ACM `sigconf` LaTeX paper. Its subject is diffing *content* -
text, code, pictures, archives, fonts, documents, catalogs, cursors - and the distance between a
delta (an edit script a machine executes) and a visualisation (what a human reads). Code is one
content type among the others; it is the one with the deepest ground truth. It is a research paper with a tool
contribution: the title and the abstract's second paragraph name OmniDiff, the introduction states
three groups of research questions, each is answered in its own section, and the tool's own section
comes after them.

1. Introduction
2. Background: Deltas and Visualisations (2.1 Deltas, 2.2 Visualisations, 2.3 Definitions: delta,
   mapping, painting, projection, 2.4 Tools)
3. A Living Ground-Truth Dataset (methodology only, no RQ answered: corpus, files, sampling,
   ground truth, the invariants, growth, cross-validation)
4. Content of Real-World Changes (RQ1.1 what content, RQ1.2 how large: code in lines, other
   content in pixels and members; RQ1.3 can a change be visualised within one second, answered
   for tree edit distance on code)
5. Cost (RQ2.1 what reaches the reader, RQ2.2 expressibility, RQ2.3 cost, RQ2.4 mapping
   uniqueness, RQ2.5 visualisation uniqueness: Minimal against Maximal)
6. State of the Art (RQ3.1 what tools show per content, RQ3.2 accuracy against the hand painting,
   RQ3.3 speed: eight established tools in eleven configurations)
7. OmniDiff
8. Threats to Validity
9. Related Work
10. Conclusions

The groups were five until the review of 2026-11-24 merged "Delta versus visualisation" and
"Uniqueness" into Cost and the one-second question into Content. Sections 5 and 6 of older
reviews are today's Section 5, Section 7 is the end of Section 4, Section 8 is Section 6.

The ACM conference, DOI, ISBN and rights fields are placeholders until a venue is chosen. The prose
is in British spelling ("visualisation", "generalises").

## Rules that are easy to break

* **No RQ answer names OmniDiff.** An RA box and its supporting argument describe the problem or
  the state of the art. RA2.3 and RA2.4 compare the human mapping's cost with the harness's own
  matcher; the text describes it tool-neutrally. Two answers use OmniDiff as an instrument:
  RA1.1's census uses its content recognition, and RA1.2's sizes of changes to other content are
  its count of changed pixels and members. OmniDiff's own results are in Section 7, and so is
  the comparison of its renderings and the hand paintings against the invariants. The line
  script and tree script comparison was RQ2.1 until the review of 2026-11-14 cut the question,
  and the review of 2026-11-24 cut it from Section 7 too; do not reintroduce it.
* **Content first.** Where a question can be asked of all content it is (RA1.1, RA1.2, RA2.1,
  RA3.1), with code as one row or one half of the answer. Do not reintroduce a "beyond code"
  framing.
* **The research questions say "delta", not "mapping"** (review of 2026-11-14), and Section 2.3
  defines a mapping as a set of pairs of multisets, so that an N:M correspondence is a mapping.
  The body of Section 5 still speaks of mappings and multi-map groups, which is the
  ground truth's own vocabulary.
* **RA2.2 and RA2.4 read the same multi-map groups two ways**: as an N:M site the four edit
  operations cannot express (RA2.2), and as a set of equally correct one-to-one pairings (RA2.4).
  RA2.4 adds a second reading that needs no annotation, cost ties at a different mapping
  (`CostTieDifferent*`). Change the two together.
* **Every measured number is a macro.** `main.tex` has no bare numeric literal from a
  measurement. Macros live in `figures/variables.tex`, written by
  `research/analysis/paper_variables.py`, which documents each block's source. Generated blocks
  are read back from measurement files; authored values (`CORPUS`, `TARGETS`, `MACHINE`, ...) are
  edited in `paper_variables.py`, never in `variables.tex`, which every build overwrites. A macro
  with no value prints a bold `??`: `-interaction=nonstopmode` does not fail on an undefined macro.
  If `plots/variables_empirical.tex` is missing, the assembler carries the previous empirical
  values forward rather than printing `??`, because the slow `measure-file-stats` run is not a
  prerequisite of the paper targets.
* **`figures/` holds symlinks into `research/plots/`**, not copies.
* **Refresh corpus-dependent blocks together** - ground-truth totals, the ambiguity, rendering and
  shape reports, the tool comparison, the fixture robustness run and the AST-diff oracle runs. The
  order: `benchmark_optimal_solutions --csv`, then `analyze_human_mappings --csv` (see
  `data/quality/PROVENANCE.md`) and the `CORPUS` block; `make ambiguity-report`,
  `rendering-report` and `shapes-report`; `make measure-presets`, `measure-invariants` and
  `measure-dataset-growth` (or their `*-report` halves when only the reports changed); `make
  measure-tools-accuracy` before `make measure-tools-timing` (both chunked and resumable, see
  `measure/benchmark_other_chunked.py`; run them as a capped systemd unit, as
  `data/comparison/PROVENANCE.md` describes); `make
  measure-robustness-fixtures`; `make measure-astdiff-oracle` and
  `measure-astdiff-oracle-human`. Afterwards, re-read the prose's ordering claims (best tool,
  fastest tool): macros cannot check them. `benchmark_other_report.py` warns when srcDiff and
  difftastic are no longer the two established tools with the highest Perfect share.
* **The paper is at one corpus state** since 2026-11-25: every fixture-based block is at 1,705
  fixtures and one engine (`cost_preference` is scoped to the comparison's fixtures, so a
  partial refresh splits it again). The fixture timings - the tool comparison and OmniDiff's
  robustness run - are the laptop's (`TIMING_MACHINE`), the corpus measurements, RQ1.3 and the
  content timings the server's (`MACHINE`); Section 3.1 names both. Re-timing on a different
  machine means changing `TIMING_MACHINE` and that paragraph.
* **Table 7 (`tab:painting-line-buckets`) scores tools against the hand painting**, at line
  level, on the painted fixtures; a tool agrees with a change painted twice when it agrees with
  either painting. RA3.2 quotes it (`*PaintingLinePerfectPct`). The byte-level table
  (`tab:painting-bytes`) and the mapping-based table and figure (`tab:agreement-buckets`,
  `*PerfectPct`) stand beside it; quote the painting macros wherever the paper says "as the hand
  painting does".
* **Three painting cases in the data, two in the text** (`human_mapping::graded_cases`):
  `Minimal` and `Maximal`, a single painting naming no preset (one rendering, both presets must
  produce it), or a single painting named for one preset. Section 3.4 describes the first two
  only, since the review of 2026-11-14; the measurements still grade all three.
  `preset_report.graded_case` mirrors the Rust rule; `PresetSingle*` is the second case and is not
  `PaintingSingle*` (`rendering_report.py`'s older count of every lone painting).
* **The invariants table is generated** (`figures/invariants_table.tex`,
  `analysis/invariants_report.py`) and shows the hand paintings only; the scripts' figures are
  macros, used in Section 7. Its five families (`GROUPS`) must partition the 32 rules of
  `src/test/helper/human_mapping/invariants.rs`, which a unit test checks, and the prose of
  Section 3.5 names the family sizes (three, ten, ten, four, five). A 33rd invariant means
  `INVARIANTS` in the `invariant_gap_census` test, `GROUPS`, and that paragraph.
* **Authored project facts** - stars, forks and their date (`PROJECT`), the issue counts
  (`dataset_growth.ISSUES_*`, `ISSUE_FIXTURE_PREFIXES`) and the before/after counts of the
  invariant repair (`INVARIANT_REPAIR`) - are read off GitHub or a past commit, not measured from
  a file. Refresh them together with their date. The stars are in the abstract and at the opening
  of Section 7; the issue counts are no longer in the text (`GrowthIssues*` are unused).
* **Four datasets**, `_common.PAPER_DATASETS`: Curated (`small`), Full, Stratified, Defects4J.
  `handmade` fixtures are regression tests, not a sample, and stay out of the paper. Only fixtures
  with a `human_mapping.json` are scored, and which Defects4J units are solved is how far
  annotation has reached, not a draw. The painted fixtures are not a random sample either (see
  `analysis/rendering_report.py`). The text is written as if every dataset were finished;
  unfinished passes are marked only in footnote `fn:in-progress` and the asterisked rows of the
  expressibility table. `REVIEWED_LISTS` in `ambiguity_report.summarize` names the datasets whose
  ambiguity pass is complete.
* **`Defects4J` is not a valid macro name**; macros spell it `DefectsFourJ`
  (`benchmark_other_report.macro_stem`, `ambiguity_report.PER_LIST_DATASETS`).
* **"RQ1" in file and macro names is the paper's RQ1.3** (RQ4 until 2026-11-24) (`data/rq1/`, `\RqOne*`, the `RQ1_GROUP_*`
  lists in `research/Makefile`); see `analysis/apted_only_report.py`'s docstring.
* **Lists that must stay in step**: `paper_variables.COMMON_SUBSET_TOOLS` with
  `benchmark_other_report.PAPER_MACRO_STEMS`, and the dataset list in
  `src/bin/benchmark_diff_pairs.rs` with `_common.PAPER_DATASETS`.
* **Generated tables use `main.tex`'s series names** (`LATEX_NAMES`; `DISPLAY_NAMES` is for
  matplotlib only, and `_escape_tex` is not applied to `LATEX_NAMES`). The tool count appears in
  the prose, the table captions and the captions `benchmark_other_report.py` generates; change them
  together.
* **Figures are vector and greyscale-safe**: each plot script writes a `.pdf` beside its `.png`,
  `main.tex` includes figures without an extension, and bars carry hatching as well as hue.
  `plots/tips` is drawn from `data/corpus_stats/file_type_counts.csv`: `make replot-file-types`
  redraws it, and `make file-stats-report MODE=full` rewrites the counts from the corpus database
  `stats.sqlite`, which is not committed.
* **Some generated macros are unused on purpose** - `Shape*`, `Ablation*`
  (`scripts/ablation_study.sh`, an older corpus state), `OracleHumanOmniDiff*`,
  `InvariantLineScript*` and `InvariantTreeScript*` (the cut RQ2.1) - so a cut result
  can return with a `main.tex` edit only.

## Reviews

`REVIEW-<date>.md` records each of the author's annotated reviews of a PDF: every mark and what was
done about it.

## Building

This uses the ACM `acmart` document class, which is not vendored into this repository. To build
locally you need a LaTeX distribution that includes it and the fonts it uses (Linux Libertine,
Biolinum, Inconsolata, newtx), plus `cm-super` for full font expansion. Without those fonts acmart
silently falls back to Computer Modern, which changes the layout and the page count (20 pages with
the right fonts on 2026-11-14); check with
`pdffonts main.pdf` that the fonts are LinLibertine, LinBiolinum and Inconsolata.

* Debian/Ubuntu: `sudo apt-get install texlive-publishers texlive-latex-extra texlive-fonts-extra cm-super`, then
  `latexmk -pdf -g main` (or `pdflatex main && bibtex main && pdflatex main && pdflatex main`). The
  `-g` matters: without it, latexmk can decide `main.pdf` is already current from `main.tex`'s own
  timestamp and skip rebuilding even though an `\input`-ed generated table changed underneath it.
* Or use a `texlive/texlive` Docker image.

Note that `figures/` holds symlinks into `research/plots/`, so building requires a real checkout -
copying this directory alone, without dereferencing them, leaves every figure dangling.

Regenerating from `research/` (not the repository root - these targets live in
`research/Makefile`):

* `make introductory-paper` - fast (seconds). Regenerates `plots/variables.tex` from whatever is
  on disk and rebuilds the PDF. It does not re-render the tool-comparison charts and tables; `make
  timing-report` does that, from `research/data/comparison/`.
* `make introductory-paper-empirical MODE=<tiny|small|full>` - re-renders Table 1 and friends from
  whatever `MODE`'s `stats.sqlite` already has, and rebuilds the PDF. Seconds on `tiny`, about half
  an hour on `full`. Does *not*
  run `file_stats` itself - that's `make measure-file-stats MODE=<mode>`, and it's the slow one
  (hours on the full corpus; see `data/corpus_stats/PROVENANCE.md`).
* `make paper-variables` - fast (instant). Just regenerates `plots/variables.tex` from whatever
  is already on disk, without rebuilding the PDF.
