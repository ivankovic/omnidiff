# Accuracy analysis: scala-com-lihaoyi-mill-split-import-2

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 8 (7 visible) of 660 nodes, 660 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`import mill.constants.{EnvVars, OutFiles}` (line 3) is split into `import mill.constants.EnvVars`
and `import mill.constants.OutFiles.OutFiles` (lines 3-4). Marko's `description.md`: "The best
solution would require a many-to-many map".

## Mapping

- Before line 3, 7 visible, 1 invisible. The ground truth maps the old import declaration to one
  of the new ones (MatchButNotIdentical) with `import`, `mill`, `.`, `constants`, `.` and
  `EnvVars` Identical, and puts `OutFiles` in a 1:2 any-one-to-one group with the two `OutFiles`
  of the second new import. omnidiff deletes the whole old declaration and inserts both new ones:
  every node has op Delete with reason `APTED("fast_fallback")`, the end-of-pipeline APTED
  fallback over still-unmatched roots, so no earlier pass (imports, qualified names, moved
  subtrees) paired the declaration. Since the two new lines share six of the old line's leaves,
  the ground truth's reading is clearly the better one. `matcher`.

The ground truth does not need an N:M group beyond the 1:2 `OutFiles` group (N:M floor 0), so the
whole difference is recoverable by a one-to-one mapping.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: import matching in `solve_syntax_aware_matching`: pair a braced import
  with the split imports that share its prefix, instead of leaving it to
  `apted::for_roots_fallback`.
- Ground truth questionable: no
