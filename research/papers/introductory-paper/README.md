# Introductory paper

"OmniDiff: Content-Aware Diffing", an ACM `sigconf` LaTeX paper. Its subject is diffing *content* -
text, code, pictures, archives, fonts, documents, catalogs, cursors - and the distance between a
delta (an edit script a machine executes) and a visualisation (what a human reads). Code is one
content type among the others; it is the one with the deepest ground truth, and the paper says so
wherever a question can only be answered for code. It is a research paper with a tool
contribution: the title and the abstract's second paragraph name OmniDiff, the introduction states
five groups of research questions, each is answered in its own section, and the tool's own section
comes after them.

1. Introduction
2. Background: Deltas and Visualisations (2.1 Deltas, 2.2 Visualisations, 2.3 Definitions: delta,
   mapping, painting, projection, 2.4 Tools)
3. A Living Ground-Truth Dataset (methodology only, no RQ answered: project and corpus, files,
   sampling, ground truth, the 31 invariants, growth, cross-validation)
4. Content of Real-World Changes (RQ1.1 what content, RQ1.2 how large)
5. Delta versus Visualisation (RQ2.1 invariants broken by a painted script, RQ2.2 what reaches the
   reader, RQ2.3 expressibility, RQ2.4 cost)
6. Uniqueness (RQ3.1 mappings, RQ3.2 visualisations: Minimal against Full)
7. Exact Deltas within an Interactive Budget (RQ4)
8. State of the Art (RQ5.1 what tools show per content, RQ5.2 accuracy, RQ5.3 speed: eight
   established tools in eleven configurations)
9. OmniDiff
10. Threats to Validity
11. Related Work
12. Conclusions

The ACM conference, DOI, ISBN and rights fields are placeholders until a venue is chosen. The prose
is in British spelling ("visualisation", "generalises").

## Rules that are easy to break

* **No RQ answer names OmniDiff.** An RA box and its supporting argument describe the problem or
  the state of the art. RA2.4 and RA3.1 compare the human mapping's cost with the harness's own
  matcher; the text describes it tool-neutrally. RA1.1's census uses OmniDiff's content
  recognition, and RA2.1's "tree script" is not OmniDiff's output but the human mapping painted
  literally. OmniDiff's own results, including its own rendering held against the invariants,
  are in Section 9.
* **Content first.** Where a question can be asked of all content it is (RA1.1, RA2.2, RA5.1),
  with code as one row or one half of the answer; where the data exists only for code the
  question says "code". Do not reintroduce a "beyond code" framing.
* **RA2.3 and RA3.1 read the same multi-map groups two ways**: as an N:M site the four edit
  operations cannot express (RA2.3), and as a set of equally correct one-to-one pairings (RA3.1).
  RA3.1 adds a second reading that needs no annotation, cost ties at a different mapping
  (`CostTieDifferent*`). Change the two sections together.
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
  measure-tools-accuracy` before `make measure-tools-timing`; `make
  measure-robustness-fixtures`; `make measure-astdiff-oracle` and
  `measure-astdiff-oracle-human`. Afterwards, re-read the prose's ordering claims (best tool,
  fastest tool): macros cannot check them. `benchmark_other_report.py` warns when srcDiff and
  difftastic are no longer the two established tools with the highest Perfect share.
* **The paper is not at one corpus state.** The node-level, painting-level, invariant, preset
  and growth numbers are at the current corpus and engine (1,699 fixtures on 2026-11-10); the tool
  comparison, the cost comparison (`cost_preference` is scoped to the comparison's fixtures) and
  OmniDiff's line-level, speed and robustness figures are at 1,217 fixtures and an earlier
  engine, and the oracle comparison at 440 units. Footnote `fn:in-progress` and the threats
  section say so; when the tool comparison is re-run on the server, drop those sentences.
* **Three painting cases** (`human_mapping::graded_cases`): `Minimal` and `Full`, a single
  painting naming no preset (one rendering, both presets must produce it), or a single painting
  named for one preset. `preset_report.graded_case` mirrors the Rust rule; `PresetSingle*` is the
  second case and is not `PaintingSingle*` (`rendering_report.py`'s older count of every lone
  painting).
* **The invariants table is generated** (`figures/invariants_table.tex`,
  `analysis/invariants_report.py`): its five families (`GROUPS`) must partition the 31 rules of
  `src/test/helper/human_mapping/invariants.rs`, which a unit test checks, and the prose of
  Section 3.5 names the family sizes (three, nine, ten, four, five). A 32nd invariant means
  `INVARIANTS` in the `invariant_gap_census` test, `GROUPS`, and that paragraph.
* **Authored project facts** - stars, forks and their date (`PROJECT`), the issue counts
  (`dataset_growth.ISSUES_*`, `ISSUE_FIXTURE_PREFIXES`) and the before/after counts of the
  invariant repair (`INVARIANT_REPAIR`) - are read off GitHub or a past commit, not measured from
  a file. Refresh them together with their date.
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
* **"RQ1" in file and macro names is the paper's RQ4** (`data/rq1/`, `\RqOne*`, the `RQ1_GROUP_*`
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
  `plots/tips.png` needs the corpus database `stats.sqlite`, which is not committed: `make
  file-stats-report MODE=full` re-reads it on the measuring machine.
* **Some generated macros are unused on purpose** - `Shape*`, `Ablation*`
  (`scripts/ablation_study.sh`, an older corpus state), `OracleHumanOmniDiff*` - so a cut result
  can return with a `main.tex` edit only.

## Reviews

`REVIEW-<date>.md` records each of the author's annotated reviews of a PDF: every mark and what was
done about it.

## Building

This uses the ACM `acmart` document class, which is not vendored into this repository. To build
locally you need a LaTeX distribution that includes it and the fonts it uses (Linux Libertine,
Biolinum, Inconsolata, newtx), plus `cm-super` for full font expansion. Without those fonts acmart
silently falls back to Computer Modern, which changes the layout and the page count (21 pages with
the right fonts on 2026-11-10); check with
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
