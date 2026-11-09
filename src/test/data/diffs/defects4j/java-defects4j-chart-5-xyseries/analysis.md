# Accuracy analysis: java-defects4j-chart-5-xyseries

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 5812 nodes, 5812 graded, N:M floor 0 visible |
| Painting, Minimal | 53 bytes wrong of 51489: renderer 53, matcher 0 |
| Painting, Full | 676 bytes wrong of 51489: renderer 676, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Chart-5 in `XYSeries.addOrUpdate`: the `!this.allowDuplicateXValues` test is taken out
of `if (index >= 0 && !this.allowDuplicateXValues)` (before line 548) and handled up front by a
new `if (this.allowDuplicateXValues) { add(x, y); return null; }` (after lines 544-547).

## Painting

No mapping mismatches. Every painting run also appears with the human mapping, so all are the
renderer's.

- Full, 668 bytes, Move -> None: the rest of the `if (index >= 0) {` statement, from `) {` to the
  closing `}` (before lines 548-557, after lines 552-561), painted `Move` on both sides. Its
  contents are unchanged and in place; the painting leaves them unpainted. The one relocated node
  is `this.allowDuplicateXValues`, hoisted from that condition to the new `if` above it, and the
  run starts right after its old position. This looks like the walk-order move attribution
  (`crossed_backwards`, settled by `reconcile_moves`, which keeps both sides' claims under
  `FULL`'s `paint_resized_moves`) blaming the statements that the hoisted node jumped over, as
  described in `move_attribution.md`. The exact path was not traced. `renderer`.
- Full, 8 bytes, None -> Insert: the indentation of the new `if` line (after line 544), which the
  Full painting includes in the insert. `renderer`.
- Minimal, 52 bytes, None -> Move: `this.allowDuplicateXValues` on before line 548 and after line
  544. Both mappings pair it, and the Minimal painting paints it `Move`. omnidiff's Minimal
  rendering paints it nothing, so the one real relocation in the file is not shown. `renderer`.
- Minimal, 1 byte, None -> Insert: the closing `}` of the new `if` block (after line 547). The
  painting marks it inserted, like the `) {` on line 544, which omnidiff does paint. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` move attribution (`crossed_backwards` / `reconcile_moves`):
  under `FULL` a hoisted expression paints the unchanged statements it passed as `Move`, and
  under `MINIMAL` it paints the hoisted expression nothing
- Ground truth questionable: no
