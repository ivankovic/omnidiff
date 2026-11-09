# Accuracy analysis: java-defects4j-closure-143-removeconstantexpressions

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1296 nodes, 1296 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 7988: renderer 0, matcher 0 |
| Painting, Full | 184 bytes wrong of 7988: renderer 184, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-143 (second file of the same commit): `RemoveConstantExpressions` keeps `new` and call
expressions: the condition's closing line `) {` (before 91) becomes two new operand lines
`|| exprBody.getType() == Token.NEW` / `|| exprBody.getType() == Token.CALL) {` (after 91-92).

## Painting

Mapping is exact and Minimal matches its painting. All 184 Full bytes survive with the human
mapping, so they are renderer-owned.

- Full, Move -> None, 166 bytes: the unchanged one-statement body and its `}` (before 92-93,
  after 93-94) painted Move. Nothing moved; the rows shifted by the added line. `renderer`.
- Full, 18 bytes: the Full painting matches the old `) {` line's indentation to the indentation
  of the new last operand line (Move) and leaves `) {` unpainted; omnidiff paints that
  indentation Insert and the relocated `) {` Move. `renderer`. (Same pattern as
  `java-defects4j-closure-143-abstractcommandlinerunner`.)

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full displacement painting in `src/diff/text.rs` (row-shifted unchanged
  block after an edit on the line holding its `{`).
- Ground truth questionable: no.
