# Accuracy analysis: scala-com-lihaoyi-mill-real-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 2121 nodes, 2121 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`Result.Failure` gains `path`, `index` and `next` fields (its parameter list is spread over six
lines), a companion `object Failure` with `combine` is added, and the two
`case Failure(error) => boundary.break(Failure(error))` clauses become
`case f: Failure => boundary.break(f)`, so the failure is passed on unchanged.

## Mapping

- Before lines 66 and 84, column 22, 2 visible, same cause twice. In `case Failure(error)` ->
  `case f: Failure`, the ground truth deletes the `case_class_pattern` with its brackets, inserts
  the `typed_pattern`, keeps `Failure` Identical and updates the bound name `error` -> `f`. omnidiff
  agrees on everything else in the clause (including the `error` -> `f` update inside
  `boundary.break(...)`) but deletes the pattern's `error`
  (so the new binder `f` is not its counterpart), reason
  `APTED("qualified_name")`. The two identifiers sit under parents of different kinds
  (`case_class_pattern` vs `typed_pattern`) and have different text, so APTED found delete +
  insert no dearer than a re-parented rename. The binder was renamed, so the ground truth's update
  is the more informative reading. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED in `solve_syntax_aware_matching` (`qualified_name`): pair the binder
  of a pattern rewritten from `C(x)` to `x: C` as an update.
- Ground truth questionable: no
