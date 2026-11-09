# Accuracy analysis: c-protocolbuffers-protobuf-add-to-preproc-define

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1444 nodes, 1444 graded, N:M floor 0 visible |
| Painting, Minimal | 79 bytes wrong of 15691: renderer 79, matcher 0 |
| Painting, Full | 80 bytes wrong of 15691: renderer 80, matcher 0 |
| Ground-truth invariant violations | none |

## The change

protobuf `upb/wire/decode_fast/combinations.h`: inside a multi-line `#define`, the condition
`type == kUpb_DecodeFast_Bool)))` (line 206) is extended to
`type == kUpb_DecodeFast_Bool || type == kUpb_DecodeFast_Bytes || \` plus a new continuation line
`type == kUpb_DecodeFast_String)))`. A pure insertion inside the macro body.

## Painting

The mapping is perfect and every run is identical with the human mapping: all renderer.

- Both presets, `Update -> Insert`, after lines 206-207, 79 bytes Minimal, 80 bytes Full: the macro
  body is one `preproc_arg` token, so the edit is an Update of that token. omnidiff narrows it to
  the inserted text (` || type == kUpb_DecodeFast_Bytes ||        \` and the new line), whose
  before side is empty, and keeps the Update label. Both paintings label the inserted tokens
  Insert. In Minimal the painting also leaves the spaces between tokens and the new line's
  indentation unpainted, which omnidiff's single narrowed range covers. Those bytes are in the same
  run (labels show the first byte only). `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `intra_node_update_ranges` in `src/diff/text.rs`: label a narrowed update
  part whose other side is empty as Insert (and, for Minimal, split it at inner whitespace)
- Ground truth questionable: no
