# Accuracy analysis: python-refactoring

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 267 nodes, 267 graded, N:M floor 0 visible |
| Painting, Minimal | 1 bytes wrong of 1034: renderer 1, matcher 0 |
| Painting, Full | 0 bytes wrong of 1034: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A handmade fixture. A statistics function replaces its manual accumulation loops with
`sum`/`len`/`max`/`min`, and the returned dict gains a `'total': total` entry. This adds a `,` after
`'count': count` (after line 17).

## Painting

- **None -> Insert, after line 17, 1 byte (the `,` added after `'count': count`), Minimal,
  `renderer`.** The Minimal painting paints the new separator comma as an Insert. omnidiff's
  Minimal rendering drops it as structural punctuation. The run is the same with the human mapping.
  Full paints it and agrees. The comma is the only change on that line, so it carries meaning. The
  Minimal punctuation filter removes it anyway.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Minimal structural-punctuation filter in src/diff/text.rs: keep an
  inserted or deleted separator that is the only change on its line (or whose neighbour across it
  is inserted)
- Ground truth questionable: no
