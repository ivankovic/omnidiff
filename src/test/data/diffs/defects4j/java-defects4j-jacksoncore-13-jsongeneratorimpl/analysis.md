# Accuracy analysis: java-defects4j-jacksoncore-13-jsongeneratorimpl

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1422 nodes, 1422 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 12050: renderer 0, matcher 0 |
| Painting, Full | 56 bytes wrong of 12050: renderer 56, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Adds an `@Override public JsonGenerator disable(Feature f)` method (lines 127-134 after) that also
resets `_cfgUnqNames` when `QUOTE_FIELD_NAMES` is disabled.

## Painting

Minimal matches the single `Minimal` painting exactly. Full, `Insert -> None`, 56 bytes: the
indentation of each of the eight inserted lines (lines 127-134 after). Full paints indentation; the
Minimal painting it is held to does not. Same with the human mapping. `grading-artifact`.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: none
- Where a fix would go: none: Full held to a single Minimal painting
- Ground truth questionable: no
