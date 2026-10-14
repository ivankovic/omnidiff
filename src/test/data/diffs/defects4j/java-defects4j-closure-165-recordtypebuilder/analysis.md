# Accuracy analysis: java-defects4j-closure-165-recordtypebuilder

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 745 nodes, 745 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 7059: renderer 0, matcher 0 |
| Painting, Full | 4 bytes wrong of 7059: renderer 4, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-165: `RecordTypeBuilder` gains an `isDeclared` flag (after line 54) with a setter
`setSynthesized` (after 63-65), and passes it to the `RecordType` constructor: line 92
`registry, Collections.unmodifiableMap(properties));` becomes
`registry, Collections.unmodifiableMap(properties), isDeclared);` (after 96).

## Painting

Mapping is exact and Minimal matches its painting. The 4 Full bytes are renderer-owned (they
survive with the human mapping).

- Full, Move -> None, 2 bytes per side: the closing `);` of the constructor call, after which
  the new argument `, isDeclared` is inserted; its column (and row) shifted, so Full paints it
  Move. The Full painting leaves it unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full displacement painting in `src/diff/text.rs` (punctuation shifted only
  by an insertion in front of it on the same line).
- Ground truth questionable: no.
