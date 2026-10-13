# Accuracy analysis: javascript-microsoft-typescript-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 76 nodes, 76 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 430: renderer 0, matcher 0 |
| Painting, Full | 12 bytes wrong of 430: renderer 12, matcher 0 |
| Ground-truth invariant violations | none |

## The change

TypeScript baseline `ArrowFunction4.js`, line 9: `var v = function (a, b) {` becomes
`var v = (a, b) => {`.

## Painting

There are no mapping mismatches and Minimal agrees exactly. Full, 12 bytes, Move -> None: the
parameter list `(a, b)` on line 9, both sides. Its column moves from 18 to 9 and its parent from
`function_expression` to `arrow_function`, and omnidiff's Full rendering paints it Move (the same
with the human mapping). The fixture has a single painting (`Only one solution`), Minimal in
style: it deletes `function ` and inserts `=>` and leaves the parameters alone. Full is held to
that painting under both presets; a Full-style painting elsewhere in this corpus marks reparented
text Move. `grading-artifact`.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: none
- Where a fix would go: none
- Ground truth questionable: no
