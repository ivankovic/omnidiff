# Accuracy analysis: css-wordpress-wordpress-one-line-to-multiline

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 143 nodes, 143 graded, N:M floor 0 visible |
| Painting, Minimal | 335 bytes wrong of 468: renderer 335, matcher 0 |
| Painting, Full | 16 bytes wrong of 468: renderer 16, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A minified one-line stylesheet with four rules is expanded into 15 formatted lines, and a comment
`/* Center alignment for classic themes. */` is added (after line 8). The rules themselves do not
change.

## Painting

All runs are the same with the human mapping, so they are the renderer's.

- Minimal, 335 bytes, Move -> None. On the before side, everything on line 1 from the first `{`
  (column 23) to the end is painted Move. On the after side, lines 3-14 are painted Move, except the
  first rule's selector, `{` and declaration. The Minimal painting leaves the reflow unpainted and
  marks only the inserted comment, which omnidiff also inserts. Each node changed row and column,
  but the order is the same, and no exemption in `identical_or_move` (src/diff/text.rs) covers a
  line being split into many. The same cause gives 37 bytes in
  `css-wordpress-wordpress-go-to-one-line`. `renderer`.
- Full, 16 bytes. The Full painting marks the content Move, as omnidiff does, and inserts the new
  whitespace: two-space indentation, the space before `{`, the space after `:`. omnidiff paints that
  whitespace Move, because it lies in the gaps of the moved ranges (14 bytes). It leaves the first
  rule's `{` (after line 1, painting Move) and line 2's indentation (painting Insert) unpainted (3
  bytes). `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `identical_or_move` in src/diff/text.rs: same-order reflow under Minimal.
  Under Full, whitespace that exists only on the after side should not be absorbed into a Move
  range.
- Ground truth questionable: no
