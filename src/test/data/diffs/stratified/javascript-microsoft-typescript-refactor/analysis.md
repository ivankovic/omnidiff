# Accuracy analysis: javascript-microsoft-typescript-refactor

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 10 (6 visible) of 243 nodes, 243 graded, N:M floor 0 visible |
| Painting, Minimal | 26 bytes wrong of 877: renderer 0, matcher 26 |
| Painting, Full | 34 bytes wrong of 877: renderer 16, matcher 46 |
| Ground-truth invariant violations | none |

## The change

TypeScript baseline `exportDeclareClass1.js`: the emitted JS loses its AMD wrapper. The body of
`define(["require", "exports"], function (require, exports) { ... });` (before lines 15-20) becomes
top-level code (after lines 15-18), unindented. The TypeScript half of the file is unchanged.

## Mapping

All 10 mismatches (6 visible, 4 invisible) are the small statements that moved out of the wrapper
function, reason `APTED("fast_fallback")`: `"use strict";` (before line 16: the two quotes, the
fragment and the `;` visible, the `expression_statement` and `string` invisible) and the two
empty statements `;` (before lines 18-19, the token visible, the `empty_statement` invisible).
The ground truth maps them Identical to after lines 15, 17, 18. The larger
`Object.defineProperty(...)` statement on line 17 is matched as expected; these three are not.
They moved up four levels (`call_expression` > `arguments` > `function_expression` >
`statement_block`) into the program, and the fallback deletes and re-inserts them rather than
bridging the removed nesting. `matcher`.

## Painting

Matcher-caused (26 bytes Minimal, 30 Full): `"use strict";` on before line 16 / after line 15 is
Delete / Insert in omnidiff's rendering. Minimal leaves it unpainted, Full paints it Move. In Full
the two `;` on before lines 18-19 / after lines 17-18 likewise (Delete / Insert -> Move). `matcher`.

Renderer-caused (Full, 4 bytes in omnidiff's rendering, 16 with the human mapping): the Full
painting marks the removed 4-space indentation of before lines 16-19 as Delete. omnidiff leaves
the indentation of line 17 unpainted; rendered from the human mapping it leaves line 16's
indentation unpainted and extends the Move over the indentation of lines 17-19. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: matching statements across removed nesting levels before
  `APTED("fast_fallback")` (the fixture's note calls it case 2 of `rust-algorithm-change`); Full
  rendering of removed indentation
- Ground truth questionable: no
