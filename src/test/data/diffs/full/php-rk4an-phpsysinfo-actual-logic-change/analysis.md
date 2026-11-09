# Accuracy analysis: php-rk4an-phpsysinfo-actual-logic-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (1 visible) of 20685 nodes, 20685 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `BSDCommon`'s CPU-load parsing, the early `if ($params[$nparams - 1] > 100) break;` (line 339)
becomes `if ($n == 2) $fix32 = $params[$nparams - 1] > 255;`, a `$fix32 = false;` initialisation
is added, and the `$val` computation is wrapped in an `if ($fix32) { ... } else { ... }` with a
new 32-bit correction branch.

## Mapping

- Before line 339, 1 visible, 1 invisible. The ground truth maps the if condition
  `($params[$nparams - 1] > 100)` to the new condition `($n == 2)` (MatchButNotIdentical) with
  both brackets Identical; the comparison inside moves into the new assignment. omnidiff deletes
  the `parenthesized_expression` and its `)` (column 60), reason `APTED("large_flat_subtree")`,
  while it keeps the `(` as the ground truth does, so the bracket pair is split: the if keeps its
  opening bracket and gets a new closing one. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the APTED run of `solve_large_flat_subtrees` over this method: map an
  if's condition container (and both brackets) when the if itself is matched and its contents
  move out.
- Ground truth questionable: no
