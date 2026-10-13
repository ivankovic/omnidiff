# Accuracy analysis: java-genymobile-scrcpy-call-slightly-different-function

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 2722 nodes, 2722 graded, N:M floor 0 visible |
| Painting, Minimal | 17 bytes wrong of 14947: renderer 17, matcher 0 |
| Painting, Full | 0 bytes wrong of 14947: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In scrcpy's `AudioCapture`, the call `startActivityAsUserWithFeature(intent)` becomes
`startActivity(intent)` (line 82).

## Painting

Minimal only, 17 bytes, renderer-owned (the same with the human mapping). The identifier rename
`startActivityAsUserWithFeature` -> `startActivity` is an Update in both mappings. Minimal narrows
it to the differing characters. The before side is left with `AsUserWithFeature`, which has no
counterpart on the after side, and paints it Update. The Minimal painting paints that suffix Delete,
because nothing replaced it. Full paints the whole identifier as an Update
(`whole_identifier_updates`) and agrees with the Full painting. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` `intra_node_update_ranges` (Minimal): a narrowed Update
  whose one side is empty could be labelled Delete/Insert
- Ground truth questionable: no
