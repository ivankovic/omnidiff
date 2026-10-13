# Accuracy analysis: cpp-godotengine-godot-add-function

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 169669 nodes, 169669 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 581670: renderer 0, matcher 0 |
| Painting, Full | 17 bytes wrong of 581670: renderer 17, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The commit adds a new method `TileSet::clear_terrains` (after lines 850-865) and registers it with
one `ClassDB::bind_method` line (after line 4333). It is a pure insertion.

## Painting

- **Indentation of inserted lines (Full, 17 bytes, `Insert -> None`, `grading-artifact`).** The
  leading tabs of the body lines of `clear_terrains` (after lines 851-863) and of the new
  `bind_method` line (after line 4333) are painted `Insert` by Full. The fixture has only a
  `Minimal` painting, which leaves indentation unpainted. The same runs are present when the human
  mapping is rendered. Full is wrong only against a painting of the other style.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: none
- Where a fix would go: none. Full is scored against a Minimal-only painting.
- Ground truth questionable: no
