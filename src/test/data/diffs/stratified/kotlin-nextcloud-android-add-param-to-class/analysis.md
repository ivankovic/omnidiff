# Accuracy analysis: kotlin-nextcloud-android-add-param-to-class

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 155 nodes, 155 graded, N:M floor 0 visible |
| Painting, Minimal | 1 byte wrong of 903: renderer 1, matcher 0 |
| Painting, Full | 0 bytes wrong of 903: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A data class gets a new last constructor parameter `val isTarget: Boolean`, so the previous last
parameter `val isExpanded: Boolean = false` (line 15) gains a trailing `,`.

## Painting

- Minimal only, renderer, 1 byte: the new `,` at the end of after line 15. The Minimal painting
  paints it Insert; the Minimal rendering leaves punctuation unpainted. Same with the human
  mapping. The comma exists only because a new parameter follows, and the painter counts it as
  part of the insertion; the Full rendering paints it and agrees. Same pattern as
  ruby-mastodon-mastodon-insert-only. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Minimal punctuation suppression in the text renderer: a separator comma
  with no before counterpart could be painted with the inserted sibling.
- Ground truth questionable: no
