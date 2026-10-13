# Accuracy analysis: java-defects4j-mockito-29-same

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 545 nodes, 545 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 2244: renderer 0, matcher 0 |
| Painting, Full | 34 bytes wrong of 2244: renderer 34, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Mockito-29. `Same.describeTo` guards against a null `wanted`. It changes
`description.appendText(wanted.toString());` to `description.appendText(wanted == null ? "null" :
wanted.toString());` (line 29). One painting is on file (`Only one solution`), so it scores both
presets.

## Painting

Full only, 34 bytes, renderer-owned (the same with the human mapping). On line 29,
`wanted.toString()` is matched Identical and only pushed right by the inserted `wanted == null ?
"null" : `. Full paints it Move on both sides (17 bytes each). The painting, like Minimal, leaves it
unpainted.

This is not only the single painting being held to Full. Full normally exempts a single-row node
slid sideways by an edit beside it on the same row (`shifted_by_an_edit_beside_it` in
`identical_or_move`). The exemption fails here because of `node_untouched_on_its_row`: the inserted
text begins with `wanted`, so the common prefix of the two rows runs through `appendText(wanted`.
The common suffix is clamped to what is left (`.toString());`), and the node then lies in neither.
Minimal is saved by its same-row fallback, and Full has none.
`java-defects4j-mockito-19-typebasedcandidatefilter` has the same mechanism. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` `node_untouched_on_its_row`: the prefix/suffix
  computation when the inserted text shares leading characters with the node after it
- Ground truth questionable: no
