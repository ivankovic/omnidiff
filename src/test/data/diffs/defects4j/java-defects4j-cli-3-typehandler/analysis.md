# Accuracy analysis: java-defects4j-cli-3-typehandler

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 14 (11 visible) of 1715 nodes, 1723 graded, N:M floor 5 visible |
| Painting, Minimal | 69 bytes wrong of 13685: renderer 42, matcher 63 |
| Painting, Full | 97 bytes wrong of 13685: renderer 55, matcher 71 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-3 in `TypeHandler.createNumber`: `return NumberUtils.createNumber(str);` (before line
162) becomes a null check and a choice between `return Double.valueOf(str);` (after line 165) and
`return Long.valueOf(str);` (after line 169), and the `NumberUtils` import is dropped.

## Mapping

- **The second `return` (5 visible, 3 invisible).** The ground truth maps the old `return ...(str);`
  to both new returns as a 1:2 all-to-all group. omnidiff maps it to the `Double` one and inserts
  the `Long` one's `return`, `(`, `str`, `)`, `;` (reason `APTED("qualified_name")`). This is the
  fixture's N:M floor (5 visible). `nm-floor`.
- **The callee (6 visible).** Within the matched `return`, the ground truth deletes
  `NumberUtils.createNumber` and inserts `Double.valueOf`; omnidiff pairs them piecewise,
  `NumberUtils` -> `Double` and `createNumber` -> `valueOf` as Updates and the `.` as Identical
  (`APTED("qualified_name")`). A different class and method is a replacement, which is how both
  paintings paint it. `matcher`.

## Painting

- Matcher-caused, 38 bytes in each preset, Update -> Delete/Insert: `NumberUtils.createNumber`
  (before line 162) and `Double.valueOf` (after line 165).
- N:M-caused, Minimal 13 / Full 25 bytes: the `Long` return's `return ` and `(str);` (after line
  169), painted Insert; the Minimal painting leaves them unpainted, the Full painting paints them
  `Move`. `nm-floor`.
- Renderer-caused (also with the human mapping), Minimal 18 bytes: `return` (before 162, after
  165) painted `Move` (it moved into a nested `if`), which the Minimal painting does not paint,
  and the braces of the new blocks (after lines 162-171), which it does. Full 34 bytes: the
  indentation of before line 162 (painted `Move` by the Full painting) and after line 165
  (painted Insert by it), which omnidiff leaves unpainted. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `nm-floor`, `renderer`
- Where a fix would go: `APTED("qualified_name")`: piecewise Updates of a replaced qualified
  callee; N:M support (`solve_multi_maps`) for one statement copied into two branches
- Ground truth questionable: no
