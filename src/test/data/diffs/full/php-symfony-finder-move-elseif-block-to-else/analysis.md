# Accuracy analysis: php-symfony-finder-move-elseif-block-to-else

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (0 visible) of 8482 nodes, 8482 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`Finder::append()` drops its final `else { throw new \InvalidArgumentException(...); }` branch:
the `elseif (is_iterable($iterator))` branch becomes the `else` (its body is unchanged), and the
`@throws` docblock lines go.

## Mapping

- Before line 718, the `else_clause` container node, 1 invisible, 0 visible. The ground truth
  deletes the `elseif` keyword and condition, moves the elseif body's statements into the
  surviving `else_clause`, keeps the `else` keyword Identical and deletes the `throw`; the old
  `else_clause` maps to the new one as MatchButNotIdentical. omnidiff agrees on every node a reader
  sees, including the `else` keyword, but leaves the old `else_clause` container itself unmapped
  (op Delete, reason `APTED("large_flat_subtree")`). The mismatch is real in the tree but has no
  visible effect. `grading-artifact` (invisible-only), with a small `matcher` inconsistency
  underneath: a matched `else` keyword whose parent clause is deleted.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: `matcher`
- Where a fix would go: none needed for a reader; for tree consistency, the APTED run of
  `solve_large_flat_subtrees` (or a later parent-propagation pass) could map the `else_clause`
  whose keyword it already matched.
- Ground truth questionable: no
