# Accuracy analysis: csharp-lidarr-condition-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1347 nodes, 1347 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 7028: renderer 0, matcher 0 |
| Painting, Full | 1 bytes wrong of 7028: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A health-check condition drops its Windows case. `if ((OsInfo.IsWindows ||
_configFileProvider.UpdateAutomatically) &&`
becomes `if (_configFileProvider.UpdateAutomatically &&` (line 42).

## Painting

Minimal matches exactly. Full has one byte, and it is the same with the human mapping.

- Full, `None -> Delete`, before line 42, 1 byte: the space between `OsInfo.IsWindows ||` and
  `_configFileProvider.UpdateAutomatically`. The Full painting deletes it, so that the kept
  operand, which it moves, starts cleanly. Omnidiff paints the deleted tokens and the moved
  operand, but not the space between them. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full rendering of inter-token whitespace between a deleted range and a
  moved neighbour.
- Ground truth questionable: no
