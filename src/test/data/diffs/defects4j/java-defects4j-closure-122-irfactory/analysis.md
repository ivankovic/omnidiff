# Accuracy analysis: java-defects4j-closure-122-irfactory

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 7 (5 visible) of 26992 nodes, 26999 graded, N:M floor 5 visible |
| Painting, Minimal | 19 bytes wrong of 108761: renderer 1, matcher 18 |
| Painting, Full | 283 bytes wrong of 108761: renderer 265, matcher 18 |
| Ground-truth invariant violations | none |

## The change

Closure-122: `IRFactory.handleBlockComment` detects suspicious JSDoc-like comments with a regular
expression: a new `Pattern p = Pattern.compile(...)` line (after 252), and the condition
`comment.getValue().indexOf("/* @") != -1 || comment.getValue().indexOf("\n * @") != -1`
(before line 252) becomes `p.matcher(comment.getValue()).find()` (after 253).

## Mapping

- The two `comment.getValue()` calls of the old condition (before 252:9 and 252:53) collapse
  into one (after 253:19). The ground truth matches both to it (2:1, an all-to-all group).
  omnidiff, one-to-one, keeps the second and deletes the first (op Delete,
  `APTED("qualified_name")`). The 5 visible mismatches equal the N:M floor. `nm-floor`
  (5 visible, 2 invisible).

## Painting

- Matcher, both presets, Delete -> Move, 18 bytes: the first `comment.getValue()` on before line
  252, painted Delete where the 2:1 painting has Move. `nm-floor`.
- Renderer, Minimal, 1 byte: the space between the deleted `!= -1` and `||` is painted Delete by
  omnidiff (gap between two deleted spans); the Minimal painting leaves it alone. `renderer`.
- Renderer, Full, 265 bytes (survive with the human mapping): the unchanged `) {` and the whole
  `if` body (before lines 252-257, after 253-258, 132 bytes per side) painted Move. Nothing there
  moved; the rows shifted by the inserted `Pattern` line and the condition in front of `) {`
  changed. Plus one space after `||` the Full painting deletes and omnidiff leaves unpainted.
  `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `nm-floor`
- Where a fix would go: Full displacement painting in `src/diff/text.rs` (row-shifted unchanged
  text after an edit on the same line).
- Ground truth questionable: no.
