# Accuracy analysis: java-defects4j-mockito-38-argumentmatchingtool

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 779 nodes, 779 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 3384: renderer 0, matcher 0 |
| Painting, Full | 29 bytes wrong of 3384: renderer 29, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Mockito-38. `ArgumentMatchingTool.toStringEquals` guards against a null argument:
`.equals(arg.toString())` becomes `.equals(arg == null? "null" : arg.toString())` (line 48).

## Painting

Full only, 29 bytes, renderer-owned (the same with the human mapping).

- **`Move -> None`, 14 bytes per side.** `arg.toString()` on line 48 is matched Identical and only
  pushed right by the inserted `arg == null? "null" : `. Full paints it Move, and both paintings
  leave it unpainted. Full's exemption for a node slid sideways by an edit on its own row
  (`shifted_by_an_edit_beside_it`) fails in `node_untouched_on_its_row`. The inserted text begins
  with `arg`, so the rows' common prefix runs through `equals(arg`, and the clamped suffix
  (`.toString());`) no longer covers the node. Minimal has a same-row fallback and is unaffected.
  `java-defects4j-mockito-29-same` has the same mechanism. `renderer`.
- **`None -> Insert`, 1 byte.** The Full painting inserts the space after `:`. The Full rendering
  does not. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` `node_untouched_on_its_row`: the prefix/suffix
  computation when the inserted text shares leading characters with the node after it
- Ground truth questionable: no
