# Accuracy analysis: java-defects4j-closure-79-normalize

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 9678 nodes, 9678 graded, N:M floor 0 visible |
| Painting, Minimal | 5 bytes wrong of 55536: renderer 5, matcher 0 |
| Painting, Full | 11 bytes wrong of 55536: renderer 11, matcher 0 |
| Ground-truth invariant violations | none |

## The change

One-line fix at line 122: `.traverse(root);` becomes `.traverseRoots(externs, root);`, so the
traversal also visits the externs (the same edit as Closure-30 in another file).

## Painting

Both runs survive with the human mapping, so all 16 bytes are the renderer's; the mapping has no
mismatches.

- Minimal, `Update -> Insert`, line 122 after side, 5 bytes `Roots`. omnidiff narrows the `traverse`
  -> `traverseRoots` identifier update to the gained suffix but keeps it `Update`:
  `intra_node_update_ranges` gives `Insert`/`Delete` middles only to content nodes, not names. The
  Minimal painting marks `Roots` Insert. `renderer`.
- Full, `None -> Move`, line 122, 5 bytes `root)` before and 6 bytes ` root)` after. The Full
  painting marks the unchanged argument `root)` Move (shifted right by the inserted `externs,`) and
  the space before it Insert; omnidiff leaves the same-row identical tail unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs intra_node_update_ranges (middle operation on names) and
  the Full handling of in-row shifted identical tails
- Ground truth questionable: no
