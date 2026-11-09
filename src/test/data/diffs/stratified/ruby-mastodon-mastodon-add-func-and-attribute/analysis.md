# Accuracy analysis: ruby-mastodon-mastodon-add-func-and-attribute

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 167 nodes, 167 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 806: renderer 0, matcher 0 |
| Painting, Full | 10 bytes wrong of 806: renderer 10, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A serializer gets a new `:id` attribute in front of `:type` in
`attributes :type, :featured_object, :featured_object_type` (line 4) and a new `def id` method.

## Painting

- Full only, renderer: `:type` on line 4 (5 bytes on each side, 10 total) is painted Move
  by omnidiff and left unpainted by the Full painting. The same run is in
  `paint_full_human.txt`, so it comes from rendering, not from the mapping. `:type` is the first
  argument before and the second after, because `:id,` is inserted in front of it; the Full
  rendering paints that shift as a move, the painter does not (the Minimal rendering does not
  paint it either). `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full-preset move painting in the text renderer: a sibling shifted only by
  an insertion in front of it should not be painted as moved.
- Ground truth questionable: no
