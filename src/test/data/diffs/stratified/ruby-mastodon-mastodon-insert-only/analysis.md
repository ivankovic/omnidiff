# Accuracy analysis: ruby-mastodon-mastodon-insert-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 363 nodes, 363 graded, N:M floor 0 visible |
| Painting, Minimal | 1 byte wrong of 1493: renderer 1, matcher 0 |
| Painting, Full | 0 bytes wrong of 1493: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A migration gets `Setting.reset_column_information` and a new keyword argument
`unique_by: ...` after the `.map do ... end` argument, so `end` (after line 17) gains a `,`.

## Painting

- Minimal only, renderer: the new `,` after `end` on after line 17 (1 byte). The Minimal painting
  paints it Insert; omnidiff's Minimal rendering leaves it unpainted. The run is the same with the
  human mapping, so it is the renderer's. Minimal rendering leaves punctuation unpainted by
  design; here the painter treated the comma, which exists only because a new argument follows,
  as part of the insertion. The Full rendering paints it and agrees. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Minimal-preset punctuation suppression in the text renderer: a separator
  comma that is itself new (no before counterpart) could be painted with the inserted sibling.
- Ground truth questionable: no
