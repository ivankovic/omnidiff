# Accuracy analysis: cpp-libreoffice-remove-two-wrapping-functions

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 20224 nodes, 20224 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 82352: renderer 2, matcher 4 |
| Painting, Full | 5 bytes wrong of 82352: renderer 2, matcher 4 |
| Ground-truth invariant violations | none |

## The change

`SetSnapGridWidth(Fraction(aGridSize.Width(), 1), Fraction(aGridSize.Height(), 1))` becomes
`SetSnapGridWidth(aGridSize.Width(), aGridSize.Height())` (line 231). The two `Fraction(..., 1)`
wrappers are removed.

## Mapping

- **Wrong comma (2 visible, `matcher`, `APTED("large_flat_subtree")`).** The ground truth deletes
  the comma inside the first wrapper (`Width(), 1`, column 60). It maps the separator between the
  two arguments (column 64) to the after separator. Omnidiff does the reverse. It keeps the inner
  comma, which the unwrapped `aGridSize.Width()` used to be next to, and deletes the real separator.
  Both are bare `,` tokens. The parent of the ground truth's choice is the outer argument list on
  both sides, which omnidiff's choice breaks.

## Painting

- **Matcher (4 bytes in each preset).** The two `, ` runs on line 231 get each other's paint. The
  inner one is left unpainted (or painted Move in Full), and the separator is painted Delete.
- **Renderer (2 bytes in each preset).** These runs stay with the human mapping. Minimal paints the
  space between a deleted `,` and the deleted `1` (`, 1` in both wrappers). The Minimal painting
  leaves it out. Full leaves two separating spaces unpainted (one per side) that the Full painting
  marks Move.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: comma assignment in the `large_flat_subtree` APTED call
  (`src/diff/solve_large_flat_subtrees.rs`) when a wrapping call is removed.
- Ground truth questionable: no
