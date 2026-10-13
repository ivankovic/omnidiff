# Accuracy analysis: xml-libreoffice-non-latin-string-data-update

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1278 nodes, 1278 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 6600: renderer 4, matcher 0 |
| Painting, Full | 4 bytes wrong of 6600: renderer 4, matcher 0 |
| Ground-truth invariant violations | none |

## The change

LibreOffice zh-TW autotext `BlockList.xml`, line 7: the `block-list:name` attribute value of one
`<block-list:block>` changes from `尊敬的女士們和先生們，` to `親愛的女士、先生們` (both CJK).

## Painting

There are no mapping mismatches. The single painting (`Only one solution`, scored under both
presets) updates the value between the quotes. omnidiff paints the two `"` of the attribute value
Update as well, on both sides, under both presets and with either mapping (4 bytes, Update ->
None). The quotes are unchanged; omnidiff paints the whole `AttValue` token instead of narrowing
the Update to the changed text inside it (it does narrow an ASCII value in
`xml-jellyfin-jellyfin-update-attribute-values`). `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` Update narrowing of a quoted attribute value whose content
  shares no characters (keep the unchanged quotes out)
- Ground truth questionable: no
