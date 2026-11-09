# Accuracy analysis: ruby-marcandre-backports-actual-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 11 (6 visible) of 951 nodes, 951 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In the `Enumerator::Product` backport, `Enumerator::Product.new(*enums).each(&block)` (before
lines 8 and 17) is split into `product = ...new(*enums)` plus `if block_given? product.each(&block);
return nil end; product`. The guard `size == nil || size == Float::INFINITY || size == 0` (before
line 59) gets `size == 0` moved to the front and a new `size == -Float::INFINITY` at the end
(after line 75). An `initialize_copy` method and `self` returns are added.

## Mapping

- **Moved `size == 0` clause, before line 59 -> after line 75** (5 visible, 4 invisible, reason
  `APTED("qualified_name")`). The ground truth maps `size == 0` from the end of the `||` chain to
  the front, and inserts `size == -Float::INFINITY` at the end. Omnidiff pairs the last clause
  with the last clause by position: before `size == 0` becomes after `size == -Float::INFINITY`
  (its `size` and `==` kept, the `0` deleted), and the front `size == 0` is new. The `||` chain is
  a left-nested `binary` tree, so the move changes nesting depth as well as order. An ordered tree
  edit distance cannot express a reorder, so APTED takes the positional answer. `matcher`: move
  not detected.
- **Split call, before line 17 -> after lines 24-27** (1 visible, 1 invisible). The ground truth
  maps the call `Enumerator::Product.new(*enums).each(&block)` 1:2 onto the new assignment and the
  new `product.each(&block)`, including the `.` of `.each` (column 38). Omnidiff deletes the `.`
  (reason `APTED("qualified_name")`) and matches none of the 1:2 group. The same change on line 8
  is not in the mismatch list, so it is handled there. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: move detection inside a reordered `||` chain (APTED in
  `solve_qualified_name_groups` cannot reorder; a moved-subtree pass for identical
  `binary` operands would be needed); the split-call case on line 17.
- Ground truth questionable: no
