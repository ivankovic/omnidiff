# Accuracy analysis: csharp-sonarr-sonarr-add-list-item

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 85 nodes, 85 graded, N:M floor 0 visible |
| Painting, Minimal | 1 bytes wrong of 554: renderer 1, matcher 0 |
| Painting, Full | 0 bytes wrong of 554: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The commit appends an enum member `UpgradesNotAllowed` after `MinCustomFormatScore`, which gains a
trailing comma (after lines 11-12).

## Painting

Minimal only, 1 byte, identical with the human mapping, so renderer-owned:

- None -> Insert, after line 11: the new `,` after `MinCustomFormatScore`. The Minimal painting
  paints it as an Insert; omnidiff's Minimal rendering drops a range that is only structural
  punctuation (`,` is in `STRUCTURAL_PUNCTUATION`, src/diff/text/render_options.rs). `renderer`.

Full agrees exactly.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Minimal structural-punctuation filter in
  src/diff/text/render_options.rs, for a lone inserted separator `,`
- Ground truth questionable: no
