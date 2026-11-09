# Accuracy analysis: cpp-optimize-algorithm

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 295 nodes, 295 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 626: renderer 0, matcher 0 |
| Painting, Full | 2 bytes wrong of 626: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A handmade fixture (no README). In `findMin`, the manual loop
`int min = vec[0]; for (...) { if (...) min = vec[i]; } return min;` becomes
`return *std::min_element(vec.begin(), vec.end());` (before lines 6-12, after line 6).

## Painting

- **Terminator painted Move (Full, 2 bytes, `Move -> None`, `renderer`).** The `return` statement is
  kept and its value is replaced. Its `;` (before line 12, after line 6) is identical, but its row
  changed because the loop above it was deleted, so omnidiff's Full rendering paints it Move. The
  Full painting leaves it unpainted. The runs are the same with the human mapping.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full rendering in `src/diff/text.rs`. An identical terminator of a kept
  statement would not be painted Move when only deletions above it shifted the line.
- Ground truth questionable: no
