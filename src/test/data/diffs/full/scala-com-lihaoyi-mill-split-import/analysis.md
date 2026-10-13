# Accuracy analysis: scala-com-lihaoyi-mill-split-import

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 11211 nodes, 11211 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `MainModuleTests.scala`, `import mill.constants.{OutFiles, OutFolderMode}` is split into
`import mill.constants.{OutFolderMode}` and `import mill.constants.OutFiles.OutFiles` (line 4 ->
lines 4-5): the code now imports the `OutFiles` member of the `OutFiles` object.

## Mapping

- Before line 4, column 24, `OutFiles`, 1 visible. The ground truth puts it in a 1:2 group with
  the two `OutFiles` of the new import line and expects exactly one pair to be matched. omnidiff
  matches none: the before `OutFiles` is deleted and both after `OutFiles` are inserted (the
  mismatch line does not print omnidiff's reason). The selector moved out of the braces into a new
  import whose text contains it twice; a move to either occurrence would satisfy the ground truth.
  `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: import handling in `solve_syntax_aware_matching` (or the moved-subtree
  pass): pair an import selector removed from a braced list with an identical identifier in a
  new import of the same prefix.
- Ground truth questionable: no
