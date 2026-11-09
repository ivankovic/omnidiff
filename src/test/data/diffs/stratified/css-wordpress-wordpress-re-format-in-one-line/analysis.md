# Accuracy analysis: css-wordpress-wordpress-re-format-in-one-line

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 87 nodes, 87 graded, N:M floor 0 visible |
| Painting, Minimal | 207 bytes wrong of 246: renderer 207, matcher 0 |
| Painting, Full | 10 bytes wrong of 246: renderer 10, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A five-declaration rule is joined onto one line (minified): `.wp-block-code{border:1px solid
#ccc;...;padding:.8em 1em}`.
The last declaration loses its `;`.

## Painting

All runs are the same with the human mapping. This is the same reformat as
`css-wordpress-wordpress-go-to-one-line`, with more declarations.

- Minimal, 207 bytes, Move -> None: the four declarations (before lines 2-5, the indentation of
  lines 3-5 included; after line 1 columns 15-115) are painted Move. The Minimal painting leaves the
  reflow unpainted and paints only the removed `;`, which omnidiff also deletes. No exemption in
  `identical_or_move` (src/diff/text.rs) covers several lines being joined into one. `renderer`.
- Full, 8 bytes: the removed two-space indentation of before lines 2-5. The Full painting deletes
  it. omnidiff leaves line 2's unpainted and paints lines 3-5 Move, as gaps between the moved
  declarations. `renderer`.
- Full, 2 bytes, None -> Move: the `{` at line 1 column 14. The Full painting has a Match entry for
  it from (0,14) to (0,14), so it is painted Move on both sides. omnidiff leaves it unpainted,
  because it is at the same position and its text is unchanged. `ground-truth` (minor).

## Verdict

- Primary cause: `renderer`
- Other causes: `ground-truth`
- Where a fix would go: `identical_or_move` in src/diff/text.rs: same-order reflow under Minimal.
  Under Full, removed indentation should be painted Delete instead of being absorbed into a Move
  gap.
- Ground truth questionable: yes, minor: the Full painting marks the rule's `{` Move although it is
  at line 1 column 14 on both sides.
