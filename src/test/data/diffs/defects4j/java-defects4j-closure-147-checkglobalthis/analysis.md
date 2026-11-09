# Accuracy analysis: java-defects4j-closure-147-checkglobalthis

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1855 nodes, 1855 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 11956: renderer 0, matcher 0 |
| Painting, Full | 64 bytes wrong of 11956: renderer 64, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-147: `CheckGlobalThis.shouldTraverse` also accepts object literals as parents: the last
operand line `pType == Token.ASSIGN)) {` (before 108) becomes `pType == Token.ASSIGN ||` plus a
new line `pType == Token.OBJECTLIT)) {` (after 108-109).

## Painting

Mapping is exact and Minimal matches its painting. All 64 Full bytes survive with the human
mapping, so they are renderer-owned.

- Full, Move -> None, 64 bytes: the relocated `)) {` and the unchanged body `return false;` and
  `}` (before 108-110, after 109-111), painted Move on both sides. The `)) {` only moved to the
  new last line and the body only shifted by one row; the Full painting leaves all of it
  unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full displacement painting in `src/diff/text.rs` (row-shifted unchanged
  block after an edit on the line holding its `{`).
- Ground truth questionable: no.
