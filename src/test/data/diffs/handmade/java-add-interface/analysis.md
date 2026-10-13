# Accuracy analysis: java-add-interface

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 185 nodes, 185 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 602: renderer 0, matcher 0 |
| Painting, Full | 2 bytes wrong of 602: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Handmade fixture: a new `interface Shape { double area(); }` is added above `Rectangle`, the class
gains `implements Shape`, and `area()` gets an `@Override` annotation.

## Painting

No mapping mismatches. Minimal agrees with its painting. Full differs by 2 bytes, and the same two
runs appear with the human mapping, so they are the renderer's:

- Full, 2 bytes, Move -> None: the class body's `{` on before line 1 and after line 5. The brace
  keeps its text and its place after the class header but is pushed right by the inserted
  ` implements Shape`. `FULL` turns on `paint_displaced_moves`, which paints such a displaced
  node `Move`; this fixture's Full painting leaves the brace unpainted. The class body itself is
  not Identical (it gains `@Override`), so the `HeritageClauseGrowth` re-tag, which is never
  painted as a move, does not cover it. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` displaced-move painting (`paint_displaced_moves`,
  `shifted_by_an_edit_beside_it`) for a brace displaced by a heritage clause when its body is not
  Identical
- Ground truth questionable: no
