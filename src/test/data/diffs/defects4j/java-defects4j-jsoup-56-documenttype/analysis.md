# Accuracy analysis: java-defects4j-jsoup-56-documenttype

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1203 nodes, 1203 graded, N:M floor 0 visible |
| Painting, Minimal | 1 bytes wrong of 4948: renderer 1, matcher 0 |
| Painting, Full | 83 bytes wrong of 4948: renderer 83, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`DocumentType` gains a `PUB_SYS_KEY` attribute: a constant (line 15 after), a new five-argument
constructor (lines 45-54 after), code that sets and prints the key (lines 32-34, 71-72 after), and
the hard-coded `" PUBLIC \""` becomes `" \""` (line 59 -> 74).

## Painting

All renderer-owned (identical with the human mapping); the mapping has no mismatches.

- Both presets, `None -> Delete`, line 59, 1 byte: both paintings delete the space before `PUBLIC`
  in `" PUBLIC \""` as its own entry, next to the deleted `PUBLIC`. omnidiff narrows the string edit
  to `PUBLIC` plus one space and trims whitespace at the ends of the range, so that space stays
  unpainted. `renderer`.
- Full only, `Move -> None`, line 59 / 74, 41 bytes each side: the unchanged tail
  `\"").append(attr(PUBLIC_ID)).append('"');` is painted grey Move. Minimal exempts it as the row's
  identical suffix; Full's `shifted_by_an_edit_beside_it` requires `s.start_row == d.start_row`, and
  15 lines inserted above move the row from 59 to 74. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs identical_or_move: Full gating of
  shifted_by_an_edit_beside_it on s.start_row == d.start_row
- Ground truth questionable: no
