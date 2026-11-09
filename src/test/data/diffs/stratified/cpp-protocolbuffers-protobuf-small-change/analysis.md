# Accuracy analysis: cpp-protocolbuffers-protobuf-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1152 nodes, 1152 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 4136: renderer 0, matcher 0 |
| Painting, Full | 18 bytes wrong of 4136: renderer 18, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`m->ParseFromArray(input.ptr, static_cast<int>(input.len))` becomes
`m->ParseFromString(absl::string_view(input.ptr, static_cast<int>(input.len)))`, wrapped onto two
lines (before line 22, after lines 22-23). The same change is made to `ParsePartialFromArray`
(before line 30, after lines 31-32).

## Painting

All runs are Full-only and are the same with the human mapping.

- **`;` painted Move (4 bytes, `Move -> None`).** The `;` of each rewritten `return` now sits on the
  continuation line, so omnidiff paints it Move. The Full painting leaves it unpainted.
- **Continuation-line indentation (12 bytes, `None -> Insert`).** The six spaces that open the new
  `absl::string_view(...)` lines (after lines 23 and 32) are inserted in the Full painting. Omnidiff
  leaves them unpainted.
- **`(` after the method name (2 bytes, `None -> Move`).** On line 22 the Full painting marks the
  call's `(` Move. Omnidiff leaves it unpainted.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full rendering in `src/diff/text.rs`, for the terminators of re-wrapped
  statements and the indentation of inserted continuation lines.
- Ground truth questionable: no
