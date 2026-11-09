# Accuracy analysis: java-defects4j-closure-137-normalize

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 6738 nodes, 6738 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 39440: renderer 0, matcher 0 |
| Painting, Full | 8 bytes wrong of 39440: renderer 8, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-137 (second file of the same commit): `Normalize.process` calls
`removeDuplicateDeclarations(root);` later: the statement moves from before line 82 to after
line 101.

## Painting

Mapping is exact and Minimal matches its painting. The 8 Full bytes are renderer-owned (they
survive with the human mapping).

- Full, the moved statement's 4-byte indentation (before line 82, after line 101), None -> Move:
  the Full painting carries the indentation with the moved line; omnidiff paints the statement
  Move but not the whitespace in front of it. `renderer`. (The same pattern as
  `java-defects4j-closure-102-normalize`, which moves this call the other way.)

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full painting of a moved line's leading indentation in `src/diff/text.rs`.
- Ground truth questionable: no.
