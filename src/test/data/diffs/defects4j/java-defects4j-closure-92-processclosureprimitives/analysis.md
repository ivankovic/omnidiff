# Accuracy analysis: java-defects4j-closure-92-processclosureprimitives

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 12498 nodes, 12498 graded, N:M floor 0 visible |
| Painting, Minimal | 12 bytes wrong of 64934: renderer 12, matcher 0 |
| Painting, Full | 0 bytes wrong of 64934: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Line 789: `namespace.indexOf('.')` becomes `namespace.lastIndexOf('.')`.

## Painting

Full matches its painting exactly (whole-identifier Update). Minimal, 12 bytes, all renderer (same
with the human mapping), line 789:

- The Minimal painting splits the rename at the word boundary: Insert `last`, Update `index` ->
  `Index`.
- omnidiff's affix narrowing in `intra_node_update_ranges` finds no common prefix (`i` vs `l`) and
  the common suffix `ndexOf`, so it paints `i` -> `lastI` as one Update and leaves `ndex` unpainted
  on both sides. Runs: `Update -> Insert` on `last` (4 bytes), `None -> Update` on `ndex` before and
  after (8 bytes). `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs intra_node_update_ranges: affix narrowing at word
  (camelCase) boundaries
- Ground truth questionable: no
