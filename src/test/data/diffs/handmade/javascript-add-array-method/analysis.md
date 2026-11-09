# Accuracy analysis: javascript-add-array-method

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 153 nodes, 153 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 385: renderer 0, matcher 0 |
| Painting, Full | 2 bytes wrong of 385: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A hand-made fixture: a linear-search `for` loop and the trailing `return -1;` (before lines 2-7)
are replaced by `return array.indexOf(target);` (after line 2). The `return` statement is kept;
its `-1` becomes `array.indexOf(target)`.

## Painting

- Full, 2 bytes, the `;` of the kept `return` statement (before line 7 col 13, after line 2 col
  32), Move -> None (`renderer`, also with the human mapping). The statement moves from line 7 to
  line 2; `return` keeps its column and is left unpainted, but the `;` is pushed right by the
  longer expression and Full paints it Move. The Full painting leaves it unpainted. Minimal
  agrees exactly.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full displacement painting in src/diff/text.rs (a trailing `;` displaced
  only by its updated sibling)
- Ground truth questionable: no
