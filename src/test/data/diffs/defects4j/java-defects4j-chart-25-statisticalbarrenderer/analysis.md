# Accuracy analysis: java-defects4j-chart-25-statisticalbarrenderer

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 32 (20 visible) of 6332 nodes, 6332 graded, N:M floor 0 visible |
| Painting, Minimal | 146 bytes wrong of 42436: renderer 0, matcher 146 |
| Painting, Full | 192 bytes wrong of 42436: renderer 80, matcher 172 |
| Ground-truth invariant violations | none |

## The change

Defects4J Chart-25 in `StatisticalBarRenderer`, made twice (horizontal and vertical bars): a null
check after `getMeanValue`, and the standard-deviation block guarded against a null std-dev.
`double valueDelta = dataset.getStdDevValue(row, column).doubleValue();` (before lines 315 and
459) becomes `Number n = dataset.getStdDevValue(row, column);` + `if (n != null) {` + `double
valueDelta = n.doubleValue();` (after lines 318-320 and 468-470).

## Mapping

- **The `valueDelta` declaration, twice (20 visible, 12 invisible; 10 + 6 per copy).** The ground
  truth keeps `double valueDelta = ... .doubleValue();` mapped to the `double valueDelta =
  n.doubleValue();` inside the new `if`, with the `dataset.getStdDevValue(row, column)` call moved
  out into the new `Number n = ...` line. omnidiff, reason `APTED("qualified_name")`, maps the
  before declaration to the new `Number n = ...` line instead: the variable name `valueDelta` is
  paired with the type `Number` (an `identifier` with a `type_identifier`, MatchButNotIdentical)
  and the `;` with that line's `;`. The remaining tokens of the before declaration (`double`, `=`,
  `.doubleValue`, `()`, `;`) are left deleted, and the after `double valueDelta = n.doubleValue();`
  is inserted. The edit distance favours the line that shares the long call, at the cost of
  splitting the declaration and pairing a name with a type. `matcher`.

## Painting

Matcher-caused (Minimal 146 bytes, Full about 172 bytes; absent with the human mapping), the same
in both methods: the before `double`, `=` and `.doubleValue()` painted Delete and `valueDelta`
Update; after `double valueDelta = ` and `.doubleValue();` painted Insert; `Number ` painted
Update where the painting has Insert.

Renderer-caused (Full 80 bytes, also with the human mapping), the same in both methods:

- Move -> None, 60 bytes: `.doubleValue();` on before lines 315/459 and after lines 320/470.
  It keeps its text but is displaced because the receiver before it changed; `FULL`'s
  `paint_displaced_moves` paints it `Move`, the Full painting does not. `renderer`.
- None -> Insert, 18 bytes: the indentation of the new `Number n = ...` lines (after 318/468)
  and the space after `n`. The Full painting paints them; omnidiff does not. `renderer`.
- Insert -> None, 2 bytes: the `;` ending the new `Number n = ...` lines. omnidiff paints it
  Insert; both paintings leave it unpainted. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: `APTED("qualified_name")`: the call site should not pair an `identifier`
  with a `type_identifier`, nor split a declaration whose name and type survive in another line
- Ground truth questionable: no
