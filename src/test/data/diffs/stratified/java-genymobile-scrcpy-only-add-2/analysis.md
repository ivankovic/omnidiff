# Accuracy analysis: java-genymobile-scrcpy-only-add-2

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 565 nodes, 565 graded, N:M floor 0 visible |
| Painting, Minimal | 1 bytes wrong of 2137: renderer 1, matcher 0 |
| Painting, Full | 0 bytes wrong of 2137: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In scrcpy's `AudioCodec` enum, a `RAW(...)` constant is added after `AAC(...)`, whose `;` becomes
`,` (lines 6-7).

## Painting

Minimal only, 1 byte, renderer-owned. Both paintings insert the `,` that now ends the `AAC(...)`
constant on line 7. Minimal leaves it unpainted. This is consistent with
`RenderOptions::structural_punctuation` being off in `MINIMAL`; the code path was not traced.
`java-genymobile-scrcpy-add-func-2` has the same 1-byte case. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` Minimal punctuation suppression: an inserted separator
  that is the only trace of a list extension on its row
- Ground truth questionable: no
