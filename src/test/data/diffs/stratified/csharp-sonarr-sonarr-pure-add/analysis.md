# Accuracy analysis: csharp-sonarr-sonarr-pure-add

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 3974 nodes, 3974 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 13164: renderer 2, matcher 0 |
| Painting, Full | 32 bytes wrong of 13164: renderer 32, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Two test-case tables gain a Georgian row: `new object[] { 52, Language.Georgian }` and `new object[]
{ Language.Georgian, 52 }` are appended (after lines 64 and 120), and the formerly last rows get a
trailing comma (after lines 63 and 119).

## Painting

One painting (`Minimal`) scores both presets. Every run is identical with the human mapping.

- Minimal, 2 bytes, None -> Insert, after lines 63 and 119: the new `,` after the old last row,
  which the painting paints and omnidiff's Minimal rendering drops as structural punctuation
  (`STRUCTURAL_PUNCTUATION`, src/diff/text/render_options.rs). `renderer`.
- Full, 32 bytes, Insert -> None, after lines 64 and 120: the 16-space indentation of each
  inserted row. Full paints the indentation of a wholly inserted line by design; the Minimal
  painting does not. `grading-artifact`.

(Under Full the two commas agree, because Full keeps punctuation.)

## Verdict

- Primary cause: `grading-artifact`
- Other causes: `renderer`
- Where a fix would go: none for the Full bytes (a Full rendering held to the only, Minimal
  painting); for Minimal, the structural-punctuation filter in src/diff/text/render_options.rs
- Ground truth questionable: no
