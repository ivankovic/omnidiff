# Accuracy analysis: xml-gap-packages-toric-remove-two-attributes

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 34 (22 visible) of 12430 nodes, 12430 graded, N:M floor 0 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

GAP package toric, `doc/Toric.xml`: the `Label="InsideCone"` and `Label="InDualCone"` attributes
are removed from two `<ManSection>` start tags (lines 383 and 426). Nothing else changes; line
numbers are the same on both sides.

## Mapping

All 34 mismatches (22 visible, 12 invisible) are far from the edit, on lines 566-568, where three
byte-identical `<A>k</A>` elements sit in one paragraph (line 566 col 36, line 567 col 36, line
568 col 44; the file has nine `<A>k</A>` in all). The ground truth maps each to itself. omnidiff
rotates them: before 566 -> after 567, before 567 -> after 568, before 568 -> after 566. The three
element roots are paired by `IdenticalHash`, their tags and text follow by
`IdenticalHashOfAncestor` (21 visible tokens, 12 invisible containers). With the order broken, the
text node between the second and third (` subsets of the\nrays describing the cone. Certain of
these `, line 567 col 44) can no longer be aligned and is deleted, reason
`APTED("large_flat_subtree_container")` (1 visible). The twin selection picks a wrong candidate
although the in-place one (same row and column, same siblings) was available. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: twin selection in `IdenticalHash` for byte-identical sibling elements
  (prefer the order-preserving assignment)
- Ground truth questionable: no
