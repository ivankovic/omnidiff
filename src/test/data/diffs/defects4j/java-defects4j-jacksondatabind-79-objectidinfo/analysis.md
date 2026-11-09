# Accuracy analysis: java-defects4j-jacksondatabind-79-objectidinfo

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1603 nodes, 1603 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 6128: renderer 0, matcher 0 |
| Painting, Full | 20 bytes wrong of 6128: renderer 20, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Adds a shared `EMPTY` constant (line 25 after) and a `public static ObjectIdInfo empty()` method
returning it (lines 64-66 after).

## Painting

Minimal matches the single `Minimal` painting exactly. Full, `Insert -> None`, 20 bytes: the
indentation of the four inserted lines (lines 25 and 64-66 after). Full paints indentation; the
Minimal painting it is held to does not. Same with the human mapping. `grading-artifact`.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: none
- Where a fix would go: none: Full held to a single Minimal painting
- Ground truth questionable: no
