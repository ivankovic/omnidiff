# Accuracy analysis: java-defects4j-closure-104-uniontype

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 4915 nodes, 4915 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 28618: renderer 0, matcher 0 |
| Painting, Full | 1 bytes wrong of 28618: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-104: `UnionType.meet` tests the result for the empty type instead of for null: line 291
`if (result != null) {` becomes `if (!result.isNoType()) {`.

## Painting

Mapping is exact and Minimal matches its painting. The single Full byte is renderer-owned (it
survives with the human mapping).

- Full, before line 291, the space between `result` and the deleted `!= null`, None -> Delete,
  1 byte: the Full painting paints the separating space Delete with the operator; omnidiff's Full
  rendering leaves this gap whitespace unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full gap-whitespace painting next to a deleted span in `src/diff/text.rs`.
- Ground truth questionable: no.
