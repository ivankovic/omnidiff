# Accuracy analysis: cpp-mongodb-mongo-add-function-and-2-call-sites

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1896 nodes, 1896 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 10746: renderer 4, matcher 0 |
| Painting, Full | 0 bytes wrong of 10746: renderer 0, matcher 0 |
| Ground-truth invariant violations | 1 |

## The change

The mock adds heartbeat counters: two call sites `++_serverHeartbeatCounts.successes;` /
`++_serverHeartbeatCounts.failures;` (after lines 48 and 63) and a new accessor
`TopologyListenerMock::serverHeartbeatCounts()` (after lines 64-67).

## Painting

- **Indentation in the Minimal painting (4 bytes, `None -> Insert`, `ground-truth`).** The Minimal
  painting's Insert range on after line 67 (`    return _serverHeartbeatCounts;`) starts at column 0
  and covers the indentation. Omnidiff's Minimal trims it, as text_painting_findings.md rule 4 says
  Minimal should. `invariants.txt` reports it ([6]: "Minimal never claims a line's indentation").
  The run is the same with the human mapping.

## Verdict

- Primary cause: `ground-truth`
- Other causes: none
- Where a fix would go: ground truth (Marko's call). This is the Minimal painting's range on after
  line 67.
- Ground truth questionable: yes. The Minimal painting paints leading whitespace (invariant [6]).
