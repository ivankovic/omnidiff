# Accuracy analysis: scala-com-lihaoyi-mill-add-a-function-call

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 642 nodes, 642 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `ScriptModule.moduleDynamicBuildOverrides` the `.map { case (k, v) => (..., v) }` lambda now
wraps the tuple's second element: `v` becomes `internal.LocatedValue(scriptConfig.scriptFile,
v.index, v)`, and the one-line case clause is reformatted over several lines.

## Mapping

- 2 visible, one wrong twin. The ground truth maps the tuple's `v` (before line 26, column 82) to
  the bare last argument `v` of `LocatedValue(...)` on after line 29, and treats the `v` of
  `v.index` (after line 29, column 56) as inserted. omnidiff maps the before `v` to the `v` of
  `v.index` instead (Identical, reason `APTED("qualified_name")`, the APTED run inside the matched
  `moduleDynamicBuildOverrides` definition), which leaves the bare `v` inserted. Both candidates are
  same-label leaves that change parent, so APTED's cost is the same either way and the left-most
  one wins. The ground truth's choice is the one that tells the reader what happened: the old value
  is passed through as the last argument and `v.index` is new code. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: tie-breaking among equal-cost leaf candidates in the APTED run of
  `solve_syntax_aware_matching` (`qualified_name`): prefer the candidate whose parent kind is
  closer to the original (bare argument over field-expression receiver).
- Ground truth questionable: no
