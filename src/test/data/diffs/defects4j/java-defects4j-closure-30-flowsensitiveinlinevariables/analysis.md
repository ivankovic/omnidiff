# Accuracy analysis: java-defects4j-closure-30-flowsensitiveinlinevariables

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 5742 nodes, 5742 graded, N:M floor 0 visible |
| Painting, Minimal | 5 bytes wrong of 32952: renderer 5, matcher 0 |
| Painting, Full | 11 bytes wrong of 32952: renderer 11, matcher 0 |
| Ground-truth invariant violations | none |

## The change

One-line fix at line 157: `(new NodeTraversal(compiler, this)).traverse(root);` becomes
`.traverseRoots(externs, root);`, so the traversal also visits the externs.

## Painting

Both runs survive with the human mapping, so all 16 bytes are the renderer's; the mapping has no
mismatches.

- Minimal, `Update -> Insert`, line 157 after side, 5 bytes `Roots`. omnidiff narrows the `traverse`
  -> `traverseRoots` identifier update to the gained suffix, but `intra_node_update_ranges` keeps
  the middle `Update` on a name (only content nodes get `Insert`/`Delete`). The Minimal painting
  marks `Roots` as Insert. `renderer`.
- Full, `None -> Move`, line 157, 5 bytes `root)` before and 6 bytes ` root)` after. The Full
  painting marks the unchanged argument `root)` as a Move (shifted right by the inserted `externs,
  `) and the space before it as Insert. omnidiff leaves the identical, same-row tail unpainted.
  `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs intra_node_update_ranges (middle operation on names) and
  the Full handling of in-row shifted identical tails
- Ground truth questionable: no
