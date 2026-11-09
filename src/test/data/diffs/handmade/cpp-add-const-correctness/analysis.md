# Accuracy analysis: cpp-add-const-correctness

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 238 nodes, 238 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 561: renderer 0, matcher 0 |
| Painting, Full | 1 byte wrong of 561: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Handmade C++ fixture (no provenance README): `printVector(std::vector<int> vec)` becomes
`printVector(const std::vector<int>& vec)` (line 4) and `std::vector<int> numbers` in `main`
becomes `const std::vector<int> numbers` (line 12).

## Painting

- Full, `None -> Insert`, 1 byte, the space between the inserted `const` and `std::vector` on
  line 4 after: the Full painting paints it as part of the insertion; omnidiff paints `const` and
  leaves the space unpainted. Identical with the human mapping. `renderer`.

Minimal agrees exactly.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full painting of the gap whitespace after an inserted token
  (`src/diff/text.rs`)
- Ground truth questionable: no
