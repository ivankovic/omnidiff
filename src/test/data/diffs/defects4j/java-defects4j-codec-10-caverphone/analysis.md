# Accuracy analysis: java-defects4j-codec-10-caverphone

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 3056 nodes, 3056 graded, N:M floor 0 visible |
| Painting, Minimal | 6 bytes wrong of 14496: renderer 6, matcher 0 |
| Painting, Full | 6 bytes wrong of 14496: renderer 6, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Line 76: the regex `"^mb"` becomes `"mb$"`, so the rule anchors at the end of the word rather than
the start.

## Painting

One renderer group, both presets, same with the human mapping, 6 bytes on line 76: both paintings
delete `^` and insert `$` (Full also paints the shifted `mb` Move). omnidiff paints the string
content `^mb` -> `mb$` as one whole Update (`Update -> Delete` before, `Update -> None` / `Update ->
Move` after). `intra_node_update_ranges` narrows only by a common prefix and suffix with a single
changed middle; here the kept `mb` is in the middle, so there is no affix and the whole content is
one Update. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs intra_node_update_ranges: one middle only, no
  delete-at-start plus insert-at-end split
- Ground truth questionable: no
