# Accuracy analysis: scala-com-lihaoyi-mill-change-from-one-function-to-other-and-delete-a-function

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 5 (4 visible) of 6692 nodes, 6692 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `CodeGen.scala`, `processDataRest`'s `onProperty` parameter type changes from
`ujson.Value` to `upickle.core.BufferedValue`, the nested-object case changes from
`upickle.read[HeaderData](v)` to
`upickle.core.BufferedValue.transform(v, upickle.reader[HeaderData])`
(spread over four lines), and the `writeBuildOverrides` function and its call are deleted.

## Mapping

Both groups come from the APTED run inside the matched `processDataRest` definition (reason
`APTED("qualified_name")`); the deleted function is handled as the ground truth expects.

- Line 101, `ujson.Value` -> `upickle.core.BufferedValue` (2 visible). The ground truth updates
  the type name `Value` -> `BufferedValue` and treats `upickle.core` as new (its `ujson` maps
  inside a 1:2 group). omnidiff maps the type name `Value` (before column 40) to the identifier
  `core` (after column 42) as MatchButNotIdentical, across node kinds (`type_identifier` ->
  `identifier`), which leaves `BufferedValue` unexplained. The ground truth keeps the
  last-segment type name with the last-segment type name, which is what the edit did. `matcher`.
- Line 107, `upickle.read[HeaderData](v)` -> `BufferedValue.transform(v, upickle.reader[...])`
  (2 visible, 1 invisible). The ground truth maps the outer call to the outer call, its argument
  list `(v)` to `(v, ...)` with `(` and `v` Identical and the `,` inserted, and moves
  `upickle.read[HeaderData]` into the arguments as `upickle.reader[HeaderData]`. omnidiff deletes
  the argument list `(v)`, its `(` and its `v`. The `v` is the same variable passed in the same
  first-argument position, so the ground truth's reading is the more faithful one. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED within matched definitions (`solve_syntax_aware_matching`,
  `qualified_name`): cross-kind leaf renames (`type_identifier` -> `identifier`) should lose to a
  same-kind update, and an argument list whose contents survive should be kept.
- Ground truth questionable: no
