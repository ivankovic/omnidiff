# Accuracy analysis: java-defects4j-cli-26-optionbuilder

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 6 (6 visible) of 1780 nodes, 1780 graded, N:M floor 0 visible |
| Painting, Minimal | 29 bytes wrong of 20015: renderer 0, matcher 29 |
| Painting, Full | 95 bytes wrong of 20015: renderer 76, matcher 48 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-26 in `OptionBuilder.create(String)`: the body is wrapped in `try { ... } finally {
OptionBuilder.reset(); }` so the builder is reset even when `new Option` throws. The declaration
`Option option = new Option(opt, description);` (before line 349) is split into `Option option =
null;` above the `try` (after line 348) and `option = new Option(opt, description);` inside it
(after line 351).

## Mapping

- **The split declaration (6 visible).** The ground truth (and both paintings) reads before line
  349 as becoming the assignment on after line 351: `option`, `=` and `;` kept, the type `Option`
  deleted, and `Option option = null;` inserted whole. omnidiff, reason
  `APTED("large_flat_subtree")`, keeps `Option option` with the new declaration on after line 348
  and deletes `=` and `;`, so the initializer moves alone into the assignment. omnidiff's reading
  (the declaration was hoisted, its initializer stayed behind) is not absurd, but the ground
  truth's is the cheaper account of the text: before line 349 differs from after line 351 only by
  the dropped `Option `. `matcher`.

## Painting

Matcher-caused (absent with the human mapping), Minimal 29 / Full 37 bytes: `Option` (before line
349) not painted Delete, `=` painted Delete; `Option option ` (after line 348) not painted Insert
and `option =` (after line 351) painted Insert.

Renderer-caused (also with the human mapping), Full 58 bytes in `real` (76 with the human mapping):

- Move -> None: `new Option(opt, description);` (before line 349, after line 351; the whole
  `option = ...;` with the human mapping). The statement only gained indentation because the new
  `try` wraps it. `FULL` paints it `Move`; this fixture's Full painting does not. The
  `WrapGrowth` re-tag (gated by `paint_reindent_only_moves`, which `FULL` turns on) is the path
  that decides this. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: `APTED("large_flat_subtree")` (`solve_large_flat_subtrees`) for the split
  declaration; `paint_reindent_only_moves` / `WrapGrowth` painting under `FULL`, which this
  fixture's Full painting does not paint (unlike `rust-add-if`'s)
- Ground truth questionable: no
