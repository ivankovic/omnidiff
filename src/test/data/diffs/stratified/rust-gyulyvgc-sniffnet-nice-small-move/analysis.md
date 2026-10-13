# Accuracy analysis: rust-gyulyvgc-sniffnet-nice-small-move

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 20 (16 visible) of 1134 nodes, 1134 graded, N:M floor 0 visible |
| Painting, Minimal | 89 bytes wrong of 3619: renderer 58, matcher 70 |
| Painting, Full | 89 bytes wrong of 3619: renderer 58, matcher 86 |
| Ground-truth invariant violations | none |

## The change

From GyulyVGC/sniffnet (`src/gui/styles/donut.rs`, commit a4df0b28): in `struct Style` the field
`pub(crate) text_color: Color,` moves from last (before line 49) to second place (after line 52),
and two fields `filtered_out` and `dropped` are appended (after lines 55-56); the struct literal
in `DonutType` gains the matching two initializers (after lines 29-33).

## Mapping

One group, reason `APTED("qualified_name")` (16 visible, 4 invisible; the after-side `Insert (with
children)` lines print the same reason), before line 49 / after lines 52 and 55. The ground truth
moves `text_color: Color` to after line 52 and inserts `filtered_out: Color` on after line 55.
omnidiff keeps the old last field in place and pairs it with the new field at that position, so
`text_color` -> `filtered_out` becomes an `Update` of the name with `pub(crate)`, `:`, `Color` and
`,` identical, and the field at its new place (after line 52) is inserted. The tree edit distance
prefers the positional update over a move of the identical one-line field, and no move pass reclaims
it. `matcher`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Painting

Matcher-caused (Minimal 70 bytes, Full 86): the moved field on after line 52 is painted `Insert`
(painting `Move`), and the new `filtered_out` field on after line 55 is painted as an update of
the old field (painting `Insert`).

Renderer-caused (58 bytes, both presets, with the human mapping too): `None -> Move` on
`pub(crate) text_color: Color,` on before line 49 and after line 52. Even with the field mapped as
the ground truth has it, `identical_or_move` does not paint it moved: it kept its column and only
changed row, which reads as an unrelated edit above; the `crossed_backwards` check that catches a
sibling reorder applies only to multi-row nodes. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: a move pass after `APTED("qualified_name")` for an identical one-line
  field reordered among its siblings; `src/diff/text.rs` `identical_or_move` (`crossed_backwards`
  for single-row nodes that changed order)
- Ground truth questionable: no
