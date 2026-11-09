# Accuracy analysis: java-genymobile-scrcpy-add

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 1274 nodes, 1274 graded, N:M floor 0 visible |
| Painting, Minimal | 11 bytes wrong of 5458: renderer 0, matcher 11 |
| Painting, Full | 11 bytes wrong of 5458: renderer 0, matcher 11 |
| Ground-truth invariant violations | none |

## The change

In scrcpy's `ScreenCapture`, `setMaxSize(int)` now returns `boolean` instead of `void`, and returns
`true` (lines 51-53).

## Mapping

- **Return type not renamed (1 visible), `matcher`.** The ground truth maps `void` (a `void_type`)
  to `boolean` (a `boolean_type`) on line 51 as MatchButNotIdentical: the method's return type
  changed. omnidiff (`APTED("qualified_name")`) deletes `void` and inserts `boolean`. The two type
  nodes have different kinds, which is the likely reason; the cost code was not checked. Both
  mappings keep the method declaration, so the return-type slot is the same.

## Painting

All 11 bytes in each preset are matcher-owned (the human mapping renders 0). `void` and `boolean` on
line 51 are painted Delete/Insert where the paintings have an Update.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the `APTED("qualified_name")` rename cost for return-type kinds (`void_type`
  <-> `boolean_type`, likewise `integral_type` and others)
- Ground truth questionable: no
