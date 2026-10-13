# Accuracy analysis: java-defects4j-math-6-powelloptimizer

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 4 (4 visible) of 4558 nodes, 4558 graded, N:M floor 0 visible |
| Painting, Minimal | 48 bytes wrong of 29612: renderer 0, matcher 48 |
| Painting, Full | 96 bytes wrong of 29612: renderer 52, matcher 44 |
| Ground-truth invariant violations | none |

## The change

Defects4J Math-6. `PowellOptimizer.doOptimize` drops its local iteration counter. `int iter = 0;` is
removed, `++iter;` becomes `incrementIterationCount();` (line 192 after), and
`checker.converged(iter, ...)` becomes `checker.converged(getIterations(), ...)` (line 226 after).
One painting is on file (`Minimal`), so it scores both presets.

## Mapping

- **Removed variable updated into method names (4 visible), `matcher`.** Reason
  `APTED("qualified_name")`. The `iter` of `++iter;` (line 193) is mapped Update to
  `incrementIterationCount`, the name of a `method_invocation` (line 192). The `iter` argument of
  `converged(iter, previous, current)` (line 227) is mapped Update to `getIterations`, the name of a
  nested call (line 226). The ground truth deletes both `iter`s, together with the declaration `int
  iter = 0;`, which omnidiff also deletes, and inserts the calls. A local counter replaced by
  inherited accessor calls is a replacement, not a rename. The `++` and the declaration are gone, so
  `iter` has no counterpart. The pairing only keeps an identifier slot filled.

## Painting

- **Matcher, 48 bytes in Minimal and 44 in Full.** `iter` on lines 193 and 227 is unpainted or
  Update where the painting has Delete. `incrementIterationCount()` and `getIterations()` on lines
  192 and 226 are partly unpainted or Update where the painting has Insert. `matcher`.
- **Grading artifact, Full, 44 bytes (`Move -> None`).** Rendering the human mapping, Full paints
  the `;` after `incrementIterationCount()` and the tail `, previous, current);` of the `converged`
  call as Move on both sides. These identical tails were pushed sideways by the edit before them,
  and their rows moved up one line, because `int iter = 0;` was deleted. Full paints such displaced
  spans Move by design (`RenderOptions::paint_displaced_moves`; the same-row exemption does not
  apply when the row index changed). The only painting on file is Minimal-style, so it leaves them
  unpainted. `grading-artifact`.
- **Grading artifact, Full, 8 bytes (`Delete -> None`).** Full paints the indentation of the deleted
  `int iter = 0;` (line 191). The only painting on file is Minimal-style, so it leaves the
  indentation out. `grading-artifact`.

## Verdict

- Primary cause: `matcher`
- Other causes: `grading-artifact`
- Where a fix would go: the `APTED("qualified_name")` leaf pairing (an identifier pairs with a
  method name in a different construct); the Full bytes need no fix (a Full painting would settle
  them)
- Ground truth questionable: no
