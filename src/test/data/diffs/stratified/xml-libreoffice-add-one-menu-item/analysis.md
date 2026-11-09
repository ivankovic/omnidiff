# Accuracy analysis: xml-libreoffice-add-one-menu-item

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 16448 nodes, 16448 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 72049: renderer 0, matcher 0 |
| Painting, Full | 0 bytes wrong of 72049: renderer 10, matcher 10 |
| Ground-truth invariant violations | none |

## The change

LibreOffice Calc `menubar.xml`: one `<menu:menuitem menu:id=".uno:SelectArrayFormula"/>` is
appended as the last item of a `<menu:menupopup>` (after line 119).

## Mapping

2 visible, the whitespace `CharData` around the new item, reason
`APTED("large_flat_subtree_container")`. Before, the item on line 118 is followed by
`"\n        "` (8 spaces, the indent of `</menu:menupopup>`). After, it is followed by
`"\n          "` (10 spaces, the new item's indent, `CharData:13`) and the new item by
`"\n        "` (`CharData:14`). The ground truth maps the old whitespace to `CharData:13` as an
Update and calls `CharData:14` new. omnidiff maps the old whitespace Identical to the
byte-identical `CharData:14` and treats `CharData:13` as new. Both readings are defensible:
position says the first gap is the old one, content says the second is. omnidiff's reading
renders the paintings exactly; rendered from the human mapping, Full misses the new item's
10-space indentation (10 bytes). `ambiguous`.

## Verdict

- Primary cause: `ambiguous`
- Other causes: none
- Where a fix would go: none: ambiguous
- Ground truth questionable: no
