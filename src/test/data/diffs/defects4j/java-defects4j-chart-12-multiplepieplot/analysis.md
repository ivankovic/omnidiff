# Accuracy analysis: java-defects4j-chart-12-multiplepieplot

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 6541 nodes, 6541 graded, N:M floor 0 visible |
| Painting, Minimal | 15 bytes wrong of 42839: renderer 15, matcher 0 |
| Painting, Full | 1 bytes wrong of 42839: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Chart-12: in the `MultiplePiePlot(CategoryDataset)` constructor (line 145) the field
assignment `this.dataset = dataset;` becomes the setter call `setDataset(dataset);`, so the plot
registers itself as a dataset listener.

## Painting

No mapping mismatches. Every painting run also appears with the human mapping, so all are the
renderer's. All are on line 145.

- Minimal, 14 bytes, Move -> None: the identifier `dataset` (the assignment's right-hand side in
  before, the call argument in after), on both sides. Both mappings pair the two `dataset`
  tokens, whose parent changed from an assignment to an argument list, and the renderer paints
  them `Move`. The Minimal painting leaves the kept argument unpainted; the Full painting agrees
  with the `Move` (no Full run there). `renderer`.
- Minimal, 1 byte, Delete -> None: the space between `this.dataset` and `=` in before. omnidiff
  paints the deleted `this.dataset =` as one range including the inner space; the Minimal
  painting leaves the space out. `renderer`.
- Full, 1 byte, None -> Delete: the space between `=` and `dataset` in before. The Full painting
  includes it in the deleted run; omnidiff stops at `=`. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` move painting under `MINIMAL` for a leaf that changed parent
  but kept its text and its place on the row; gap whitespace between deleted tokens
- Ground truth questionable: no
