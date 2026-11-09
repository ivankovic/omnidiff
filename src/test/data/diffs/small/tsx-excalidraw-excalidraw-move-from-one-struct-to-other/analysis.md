# Accuracy analysis: tsx-excalidraw-excalidraw-move-from-one-struct-to-other

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 5511 nodes, 5511 graded, N:M floor 0 visible |
| Painting | no painting on file |
| Ground-truth invariant violations | none |

## The change

`MultiDimension.tsx` moves from the `Point` tuple type to the math package's `GlobalPoint`: the
import changes (lines 16, after 23), the `anchor` parameter type changes (line 107), and two point
literals `[x1, y1]` become `point(x1, y1)` calls (lines 184 and 289).

## Mapping

- Before lines 184 and 289, the `,` of `[x1, y1]`, 2 visible: the ground truth maps it to the `,`
  of `point(x1, y1)`. omnidiff deletes it (and inserts the after comma), reason
  `APTED("qualified_name")` (the method/arrow-function APTED call of `solve_qualified_name_groups`,
  src/diff/solve_syntax_aware_matching.rs). `x1` and `y1` themselves are mapped as the ground
  truth expects. Under `UnitCostModel::ren` (src/diff/apted/common.rs) a cross-kind pair such as
  `array` -> `arguments` costs more than delete + insert, so the comma's parent is necessarily
  deleted; but that does not by itself forbid mapping the comma, whose mapped ancestors (the outer
  `arguments`) stay consistent, and mapping it would be cheaper. Why the comma is left out was not
  determined from the evidence (a pre-match pruning `x1`/`y1` before APTED is one possibility,
  not verified). `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the method-body APTED call of `solve_qualified_name_groups`
  (src/diff/solve_syntax_aware_matching.rs) or a pre-match before it; the exact cause was not pinned
  down
- Ground truth questionable: no
