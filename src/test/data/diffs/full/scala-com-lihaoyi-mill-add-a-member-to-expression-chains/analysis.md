# Accuracy analysis: scala-com-lihaoyi-mill-add-a-member-to-expression-chains

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 3 (2 visible) of 6621 nodes, 6621 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`CodeGen.scala` adapts to `HeaderData` fields becoming located values: `.value` (and
`.value.map(_.value)`) is appended to several member chains, and the reader passed to
`BufferedValue.transform` changes from `upickle.reader[HeaderData]` to
`HeaderData.headerDataReader(scriptPath)`, reformatted over four lines.

## Mapping

All 3 mismatches are on before line 109, `upickle.reader[HeaderData]` ->
`HeaderData.headerDataReader(scriptPath)` (after lines 111-112).

- The ground truth deletes the `generic_function` and inserts the `call_expression`, but maps their
  insides: `upickle.reader` -> `HeaderData.headerDataReader` (updates), the `type_arguments`
  `[HeaderData]` -> the `arguments` `(scriptPath)`, `HeaderData` -> `scriptPath`, `[` -> `(` and
  `]` -> `)`.
- omnidiff agrees on everything except the bracket list: it maps the inner `HeaderData` ->
  `scriptPath` as the ground truth does, but deletes the `type_arguments` node (invisible) and its
  `[` and `]` (2 visible), all with reason `APTED("qualified_name")` (the APTED run inside the
  matched `processDataRest` definition). `type_arguments`/`arguments` and `[`/`(` have different
  labels, so APTED pays a rename for each, and with these small subtrees delete + insert came out
  no dearer. The result maps a child while deleting its container and both brackets, which is less
  coherent than the ground truth's container-and-brackets update. `matcher`.

The visible effect is small: two bracket characters shown as deleted/inserted instead of updated.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED in `solve_syntax_aware_matching` (`qualified_name`): when a child is
  mapped, map its bracketed container (`type_arguments` <-> `arguments`) and the bracket pair with
  it rather than deleting them.
- Ground truth questionable: no
