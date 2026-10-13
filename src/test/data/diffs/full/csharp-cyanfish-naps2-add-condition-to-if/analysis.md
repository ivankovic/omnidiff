# Accuracy analysis: csharp-cyanfish-naps2-add-condition-to-if

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 3 (0 visible) of 11235 nodes, 11235 graded, N:M floor 0 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

A fourth alternative is appended to an `if` condition in `SaneScanDriver.cs`:
`a || b || c` becomes `a || b || c || controller.TrySet(SaneOptionNames.DUPLEX, true)` (lines
137-140).

## Mapping

There are 3 mismatches, all invisible: `binary_expression` nodes on line 137. tree-sitter nests
`||` to the left, so before is `((a || b) || c)` and after is `(((a || b) || c) || d)`.

- Omnidiff maps before `((a || b) || c)` to the identical after `((a || b) || c)` (`IdenticalHash`)
  and `(a || b)` to `(a || b)` (`IdenticalHashOfAncestor`). The new outermost node is the insert.
- The ground truth aligns the spine from the top. Before's outermost maps to after's outermost
  (MatchButNotIdentical), before `(a || b)` maps to after `((a || b) || c)`, and after `(a || b)`
  is new.

The leaves, the operators and every visible token agree, so a reader sees the same diff from
both. Both alignments of the invisible spine are valid. Omnidiff's keeps identical subtrees
whole. `grading-artifact`, close to `ambiguous`.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: `ambiguous`
- Where a fix would go: none (invisible nodes only, both readings valid)
- Ground truth questionable: no
