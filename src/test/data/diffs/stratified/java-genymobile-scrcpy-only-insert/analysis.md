# Accuracy analysis: java-genymobile-scrcpy-only-insert

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 106 nodes, 106 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 594: renderer 0, matcher 0 |
| Painting, Full | 4 bytes wrong of 594: renderer 4, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In scrcpy's `AsyncProcessor` interface, a nested `TerminationListener` interface is added, and `void
start()` becomes `void start(TerminationListener listener)`.

## Painting

Full only, 4 bytes, renderer-owned. The inserted parameter pushes `);` right, and the inserted
nested interface pushes the row down (line 4 -> 13). Full paints such a displaced span Move by
design (`RenderOptions::paint_displaced_moves`). This fixture's own Full painting leaves it
unpainted, so the Full convention and the Full painting disagree. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs`: `RenderOptions::paint_displaced_moves` under Full,
  against this fixture's Full painting
- Ground truth questionable: no
