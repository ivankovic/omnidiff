# Accuracy analysis: css-wordpress-reformat

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 30 (22 visible) of 192 nodes, 192 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

The commit expands a minified one-line WordPress stylesheet into 18 formatted lines. It also
swaps the order of `margin-bottom` and `margin-top` in three rules. The last declaration of each
rule gains a `;`.

## Mapping

- 22 visible and 8 invisible, all `APTED("fast_fallback")`. In `:where(.wp-block-post-excerpt)` both
  margins are `var(--wp--style--block-gap)`; in `.wp-block-post-excerpt__excerpt` both are `0`. The
  ground truth pairs the declarations by property name (`margin-bottom` at before column 54 to after
  line 4, `margin-top` at column 96 to after line 3), which is a reorder. omnidiff pairs them by
  position: before `margin-bottom` goes to after line 3 `margin-top` (op Update on the
  `property_name`), and every value leaf goes with it. The `;` leaves are shuffled the same way
  (before column 95 to after line 3; before column 182 and after line 8). Tree edit distance cannot
  express a swap. Renaming two property names costs less than deleting and re-inserting one
  declaration, so the result is APTED's optimum. In `__more-text` the two values differ and omnidiff
  pairs by name, as the ground truth does. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: a sibling-reorder/move pass that runs before the phase 6 APTED fallback and
  pairs declarations by `property_name` when values tie.
- Ground truth questionable: no
