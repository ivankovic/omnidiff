# Accuracy analysis: cpp-paddlepaddle-paddleocr-add-namespace-closing-comment

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 507 nodes, 507 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 1258: renderer 0, matcher 0 |
| Painting, Full | 1 byte wrong of 1258: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The namespace's closing `}` gains a trailing comment: `} // namespace ppredictor` (line 26).

## Painting

- **Space before the comment (Full, 1 byte, `None -> Insert`, `renderer`).** The Full painting
  inserts ` // namespace ppredictor` with its leading space. Omnidiff's Full rendering inserts the
  comment only. The run is the same with the human mapping.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full rendering in `src/diff/text.rs`. It would include the separating space
  before an inserted trailing comment.
- Ground truth questionable: no
