# Accuracy analysis: go-ollama-ollama-delete-a-param

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 9724 nodes, 9724 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 26470: renderer 0, matcher 0 |
| Painting, Full | 62 bytes wrong of 26470: renderer 62, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In two test stubs (lines 43 and 237) the `_ *ggml.GGML` parameter is removed from
`loadFn: func(req *LlmRequest, _ *ggml.GGML, _ ml.SystemInfo, _ []ml.DeviceInfo, _ bool) bool`.

## Painting

Minimal matches exactly. All Full runs are the same with the human mapping, so they are the
renderer's.

- Full, `Move -> None`, 15 bytes x 4 = 60 bytes: `_ ml.SystemInfo`, the parameter right after
  the deleted one, on both sides of both lines. The deletion shifted it left on the same row, and
  Full paints it grey Move. The parameters after it (`_ []ml.DeviceInfo`, `_ bool`) shifted by the
  same amount and are not painted. So the rule reacts only to the deleted neighbour, not to the
  shift itself. The Full painting leaves it alone. `renderer`.
- Full, `None -> Delete`, 2 bytes: the space before `_ *ggml.GGML,`. The Full painting deletes it
  with the parameter. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full displaced-move painting in `src/diff/text.rs` (`identical_or_move`,
  `shifted_by_an_edit_beside_it`), for a sibling shifted by a deleted neighbour on the same line.
- Ground truth questionable: no
