# Accuracy analysis: tsx-kong-insomnia-rewrite-if-using-ternary-twice

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 14 (8 visible) of 8414 nodes, 8415 graded, N:M floor 0 visible |
| Painting | no painting on file |
| Ground-truth invariant violations | none |

## The change

Two `if (!previousEnv) { sourceEnv.metaSortKey = …; } else { sourceEnv.metaSortKey = …; }` blocks
(before lines 218-222 and 227-231, the second with `nextEnv`) become single assignments of a
ternary, `sourceEnv.metaSortKey = !previousEnv ? … : …;` (after lines 218-220 and 225-227). Marko's
description.md: "Contains a 2:1 mapping not currently expressible with 1-1 maps". The ground truth
expresses it as multi-map groups: the two branch assignments (statement, `assignment_expression`,
`=`, `;`) against the one new assignment, with exactly one pair expected to match. So it is
reachable one-to-one, and the N:M floor is 0.

## Mapping

Both rewrites fail the same way, all with reason `APTED("qualified_name")` (the function-body APTED
call of `solve_qualified_name_groups`, src/diff/solve_syntax_aware_matching.rs):

- The merged assignment, 4 visible (`=` and `;` of before lines 219 and 228) + 4 invisible (the two
  `expression_statement`s and `assignment_expression`s): omnidiff maps neither branch's assignment
  to the new one, so the multi-map groups get 0 pairs instead of 1. This much follows from the tree
  edit distance's ancestry rule: the new assignment holds both old right-hand sides, and each old
  assignment holds only one, so pairing the new assignment with either old one would forbid
  mapping the other branch's right-hand side. Leaving the assignment unmapped is cheaper. `matcher`
  (one-to-one TED cannot express the merge cheaply).
- The condition `!previousEnv` / `!nextEnv`, 4 visible (`!` and the identifier, before lines 218
  and 227) + 2 invisible (the `unary_expression`s): the ground truth maps it identically into the
  ternary's condition; omnidiff deletes it. The ancestry rule does not explain this: with the
  `if_statement` and the new assignment both unmapped, the condition could map at no cost. Why
  APTED's solution (or a pre-match pruning before it) drops it was not determined from the
  evidence. `matcher`.

The single `if/else` -> ternary rewrite of `tsx-kong-insomnia-if-to-ternary-operator` keeps the
condition, so the difference is in this shape (a statement in both branches with the same left-hand
side), not in if-to-ternary as such.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the function-body APTED call of `solve_qualified_name_groups`
  (src/diff/solve_syntax_aware_matching.rs), or a dedicated post-pass for if/else-to-ternary merges
  that pairs the new assignment with one branch and keeps the condition
- Ground truth questionable: no
