# Accuracy analysis: java-defects4j-chart-20-valuemarker

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 744 nodes, 744 graded, N:M floor 0 visible |
| Painting, Minimal | 32 bytes wrong of 9968: renderer 32, matcher 0 |
| Painting, Full | 2 bytes wrong of 9968: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Chart-20: the `ValueMarker` constructor (line 95) passed `paint, stroke` twice to
`super(...)`; the fix passes `outlinePaint, outlineStroke` as the third and fourth arguments.

## Painting

No mapping mismatches. Every painting run also appears with the human mapping, so all are the
renderer's. All are on line 95.

- Minimal, 32 bytes, how an identifier that gained a prefix is painted. The Minimal painting
  paints `outline` as Insert and pairs the whole `paint` with `Paint` (and `stroke` with
  `Stroke`) as an Update. omnidiff narrows the Update to the differing characters: before `p`
  and after `outlineP` (likewise `s` / `outlineS`), all labelled Update. So `aint`/`troke` are
  unpainted on both sides where the painting has Update, and `outline` is Update where the
  painting has Insert. `renderer`.
- Full, 2 bytes, None -> Move: the space after the `,` between the third and fourth arguments, on
  both sides. The Full painting pairs `, ` with `, ` as a Move (byte-identical, shifted); omnidiff
  paints the comma `Move` but not the space after it. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` Update narrowing under `MINIMAL` (a prefix added to an
  identifier read as Insert + Update rather than one narrowed Update); whitespace after a moved
  separator under `FULL`
- Ground truth questionable: no
