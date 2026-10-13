# Accuracy analysis: xml-mozilla-firefox-firefox-update-value

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 68 nodes, 68 graded, N:M floor 0 visible |
| Painting, Minimal | 38 bytes wrong of 1467: renderer 38, matcher 0 |
| Painting, Full | 38 bytes wrong of 1467: renderer 38, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Fenix `res/raw/keep.xml`, line 6: `@drawable/ic_synced_tabs,` is removed from the long
comma-separated `tools:keep` attribute value.

## Painting

There are no mapping mismatches; the run is identical with the human mapping, so it is the
renderer's. One run of 38 bytes on before line 6 (cols 110-147), under both presets (single
painting, `Only one solution`). The painting deletes `@drawable/ic_synced_tabs,` (cols 110-134).
omnidiff narrows the updated `AttValue` by common prefix and suffix, which places the removed 25
bytes 13 bytes later, at `synced_tabs,@drawable/ic_` (cols 123-147): the shared `@drawable/ic_`
is absorbed into the prefix. Both placements describe the same edit; the painting's follows the
list items. The run is unbroken across the overlap (cols 123-134), so omnidiff's label there is
not Delete (it paints the before side of the updated leaf, presumably Update, as in
`xml-godotengine-godot-update-element-value`). `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` narrowing of an updated text leaf (align a pure deletion to
  token / separator boundaries, and paint it Delete)
- Ground truth questionable: no
