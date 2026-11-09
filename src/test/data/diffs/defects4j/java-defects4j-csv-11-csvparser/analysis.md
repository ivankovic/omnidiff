# Accuracy analysis: java-defects4j-csv-11-csvparser

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 4330 nodes, 4330 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 35800: renderer 0, matcher 0 |
| Painting, Full | 47 bytes wrong of 35800: renderer 47, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Line 384: `emptyHeader = header.trim().isEmpty();` becomes `emptyHeader = header == null ||
header.trim().isEmpty();`, guarding against a null header.

## Painting

Minimal matches exactly. Full, 47 bytes, renderer (same with the human mapping), line 384:

- `Move -> None` on `header.trim().isEmpty()`, both sides (23 + 23 bytes): omnidiff's Full rendering
  paints the unchanged call grey Move because it went one level deeper (right operand of the new
  `||`) and shifted right. The Full painting leaves it unpainted.
- `None -> Insert` on the space before the inserted `header == null ||` (1 byte), which the Full
  painting includes.

The Full painting here does not mark a re-parented, shifted operand Move, while other Full paintings
in this corpus do (`java-defects4j-closure-30-flowsensitiveinlinevariables` marks a shifted `root)`
Move). Held to this painting, Full's move painting is `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs Full move painting of a node re-parented under a new binary
  operator
- Ground truth questionable: no
