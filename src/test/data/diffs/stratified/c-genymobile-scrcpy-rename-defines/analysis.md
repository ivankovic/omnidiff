# Accuracy analysis: c-genymobile-scrcpy-rename-defines

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 42 nodes, 42 graded, N:M floor 0 visible |
| Painting, Minimal | 15 bytes wrong of 595: renderer 15, matcher 0 |
| Painting, Full | 0 bytes wrong of 595: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

scrcpy events header: the five `EVENT_*` defines are renamed `SC_EVENT_*` (lines 1-5); the
value column keeps its position, so the padding after each name shrinks by three spaces.

## Painting

- **Minimal, Update -> Insert, 15 bytes:** the added `SC_` prefix on each of the five after
  lines. omnidiff renders each rename as an Update narrowed to the added prefix; the Minimal
  painting inserts the prefix. The same with the human mapping. `renderer`.

Full agrees with the painting; the mapping has no mismatches.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` Minimal Update narrowing: label a purely added prefix
  Insert
- Ground truth questionable: no
