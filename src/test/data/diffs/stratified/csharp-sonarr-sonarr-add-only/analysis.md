# Accuracy analysis: csharp-sonarr-sonarr-add-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 2094 nodes, 2094 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 10004: renderer 2, matcher 0 |
| Painting, Full | 0 bytes wrong of 10004: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The commit adds three properties `MinSize`, `MaxSize`, `PreferredSize` to a resource class (after
lines 28-30) and copies them in both mapping methods' object initialisers, so the formerly last
initialiser `Allowed = …` gains a trailing comma (after lines 81 and 132).

## Painting

Minimal only, 2 bytes, identical with the human mapping, so renderer-owned:

- None -> Insert, after lines 81 and 132: the new `,` after `Allowed = model.Allowed` and
  `Allowed = resource.Allowed`. The Minimal painting paints each as an Insert; omnidiff's Minimal
  rendering drops a range that is only structural punctuation (`STRUCTURAL_PUNCTUATION`,
  src/diff/text/render_options.rs). `renderer`.

Full agrees exactly.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Minimal structural-punctuation filter in
  src/diff/text/render_options.rs, for a lone inserted separator `,`
- Ground truth questionable: no
