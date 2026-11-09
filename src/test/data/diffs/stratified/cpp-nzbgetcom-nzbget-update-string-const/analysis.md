# Accuracy analysis: cpp-nzbgetcom-nzbget-update-string-const

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 5816 nodes, 5816 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 18122: renderer 0, matcher 0 |
| Painting, Full | 1 byte wrong of 18122: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The word `the` is inserted into an error message string:
`"... Make sure that the path to 7-Zip is valid."` (line 118).

## Painting

- **Space after the inserted word (Full, 1 byte, `None -> Insert`, `renderer`).** The Full painting
  inserts `the ` with its trailing space. Omnidiff's Full rendering inserts `the` only. Minimal
  agrees. The run is the same with the human mapping.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full rendering in `src/diff/text.rs`. It would include the separating space
  of a word inserted inside a string.
- Ground truth questionable: no
