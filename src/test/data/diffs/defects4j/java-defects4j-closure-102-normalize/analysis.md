# Accuracy analysis: java-defects4j-closure-102-normalize

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 6472 nodes, 6472 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 35796: renderer 0, matcher 0 |
| Painting, Full | 8 bytes wrong of 35796: renderer 8, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-102: `Normalize.process` calls `removeDuplicateDeclarations(root)` earlier, before the
traversal it used to follow: the statement moves from before line 94 to after line 89.

## Painting

Mapping is exact and Minimal matches its painting. The 8 Full bytes are renderer-owned (they
survive with the human mapping).

- Full, the moved statement's 4-byte indentation (before line 94, after line 89), None -> Move:
  the Full painting carries the indentation with the moved line; omnidiff paints the statement
  Move but not the whitespace in front of it. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full painting of a moved line's leading indentation in `src/diff/text.rs`.
- Ground truth questionable: no.
