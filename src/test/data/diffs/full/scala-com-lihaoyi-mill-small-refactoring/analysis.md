# Accuracy analysis: scala-com-lihaoyi-mill-small-refactoring

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 1147 nodes, 1147 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `ExpandBracesTests`, the destructuring `val Result.Failure(error) = ...` becomes
`val (failure: Result.Failure) = ...`, and the following `assert(error.contains(...))` becomes
`assert(failure.error.contains(...))` (lines 54-55).

## Mapping

- Before line 54, 2 visible, reason `APTED("qualified_name")`. The ground truth deletes the
  `case_class_pattern`, inserts the `tuple_pattern` and `typed_pattern`, keeps `Result.Failure`
  Identical, keeps both brackets `(` `)` Identical and updates the bound name `error` -> `failure`.
  omnidiff deletes the `(` (column 29) and the `error` (column 30). It does not report a mismatch
  for the `)`, so it keeps the closing bracket while deleting the opening one, splitting the pair.
  The binder was renamed and the brackets survive around it, so the ground truth's reading is the
  coherent one. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED in `solve_syntax_aware_matching` (`qualified_name`): map a pattern's
  binder and both brackets across `case_class_pattern` -> `tuple_pattern`/`typed_pattern`, and do
  not split a bracket pair.
- Ground truth questionable: no
