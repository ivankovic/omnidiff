# Accuracy analysis: css-wordpress-wordpress-go-to-one-line

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 49 nodes, 49 graded, N:M floor 0 visible |
| Painting, Minimal | 37 bytes wrong of 120: renderer 37, matcher 0 |
| Painting, Full | 4 bytes wrong of 120: renderer 4, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A four-line rule is joined onto one line (minified): `.wp-block-archives
.wp-block-archives{border:0;margin:0}`.
The last declaration loses its `;`.

## Painting

All runs are the same with the human mapping, so they are the renderer's. Both omnidiff and both
paintings mark the removed `;` Delete.

- Minimal, 37 bytes, Move -> None: `border:0;` (before line 2), `  margin:0;` (before line 3,
  indentation included) and `border:0;margin:0` (after line 1) are painted Move. They changed row
  and column, but kept their order inside the same block. The Minimal painting does not see a reflow
  as a change and leaves them unpainted. omnidiff paints Move for a column change unless one of the
  "shifted on its own row" exemptions in `identical_or_move` (src/diff/text.rs) applies, and none
  fits a join of several lines. `renderer`.
- Full, 4 bytes: the two-space indentation removed from before lines 2 and 3. The Full painting
  deletes both. omnidiff leaves line 2's unpainted and paints line 3's Move (the gap between the two
  moved declarations). The declarations themselves are Move in both. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `identical_or_move` in src/diff/text.rs. Under Minimal, a same-order reflow
  should not paint Move. Under Full, removed indentation should be painted Delete instead of being
  absorbed into a Move gap.
- Ground truth questionable: no
