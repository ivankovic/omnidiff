# Accuracy analysis: java-genymobile-scrcpy-add-func-2

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 288 nodes, 288 graded, N:M floor 0 visible |
| Painting, Minimal | 1 bytes wrong of 1012: renderer 1, matcher 0 |
| Painting, Full | 0 bytes wrong of 1012: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In scrcpy's `AudioSource` enum, a `PLAYBACK("playback")` constant is added after `MIC("mic")` (whose
`;` becomes `,`), and so is an `isDirect()` method.

## Painting

Minimal only, 1 byte, renderer-owned. On line 5 the `,` that now ends `MIC("mic")` is inserted in
both paintings, and the Full painting matches the old `;` to the new `;` after
`PLAYBACK("playback")`. Minimal leaves the comma unpainted. This is consistent with
`RenderOptions::structural_punctuation` being off in `MINIMAL`; the code path was not traced. The
Minimal painting paints it anyway, because here the punctuation carries the edit: the enum constant
list was extended. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` Minimal punctuation suppression: an inserted separator
  that is the only trace of a list extension on its row
- Ground truth questionable: no
