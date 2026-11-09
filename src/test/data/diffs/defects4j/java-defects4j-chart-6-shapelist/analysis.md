# Accuracy analysis: java-defects4j-chart-6-shapelist

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 17 (12 visible) of 1081 nodes, 1081 graded, N:M floor 0 visible |
| Painting, Minimal | 35 bytes wrong of 9466: renderer 0, matcher 35 |
| Painting, Full | 112 bytes wrong of 9466: renderer 71, matcher 43 |
| Ground-truth invariant violations | none |

## The change

Defects4J Chart-6: `ShapeList.equals` stops delegating to `super.equals(obj)` (before line 111) and
compares the shapes element by element in a new `for` loop, ending in `return true;` (after lines
111-118). The fixture has one painting, `Minimal`, so both presets are scored against it.

## Mapping

- **`return super.equals(obj);` (12 visible, 5 invisible).** The ground truth keeps the `return`
  statement (before line 111 -> after line 118, `return` and `;` Identical) and deletes the call
  `super.equals(obj)`. omnidiff, all through `APTED("qualified_name")`, does the opposite: it
  deletes `return` and `;`, and pairs the call's `.equals(obj)` with `.get(i)` of `that.get(i)`
  deep inside the new loop (after line 114): `.` and the parentheses Identical, `equals` -> `get`
  and `obj` -> `i` as Updates. The new `return true;` is then inserted. The pairing has nothing
  in common but punctuation, and the ground truth's reading (the method still ends by returning,
  now `true`) is the natural one. `matcher`.

## Painting

Matcher-caused (Minimal 35 bytes, Full 43 bytes; absent with the human mapping): before `return `
painted Delete and `.equals(obj)` painted `Move` where the painting deletes it; after `.get(i)`
painted `Move` where the painting inserts it; after `return ` and `;` (line 118) painted Insert
where the painting keeps them. In Minimal the `}` of the inner `if` (after line 116) also loses
its Insert paint under omnidiff's mapping.

Full rendering held to the Minimal painting (69 + 2 bytes, also with the human mapping):

- Insert -> None, 69 bytes: the indentation of the inserted lines (after lines 111-117). `FULL`
  paints leading whitespace; the Minimal painting does not. `grading-artifact`.
- Move -> None, 2 bytes: the kept `;` of the `return` (before 111, after 118), displaced by the
  changed expression before it and painted `Move` under `paint_displaced_moves`.
  `grading-artifact`.

## Verdict

- Primary cause: `matcher`
- Other causes: `grading-artifact`
- Where a fix would go: `APTED("qualified_name")`: the call site should not pair a method
  invocation with an unrelated one deep in an inserted loop when that costs the enclosing
  `return` its match
- Ground truth questionable: no
