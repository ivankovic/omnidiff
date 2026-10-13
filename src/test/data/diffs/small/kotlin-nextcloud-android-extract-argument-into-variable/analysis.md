# Accuracy analysis: kotlin-nextcloud-android-extract-argument-into-variable

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 3858 nodes, 3858 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `GalleryRowHolder.kt` line 79 the argument `dimensions[i]` of
`adjustFile(i, row.files[i], dimensions[i], row)` is extracted into a new local
`val dim = dimensions.getOrNull(i) ?: (...)` (after line 79), and the call passes `dim`.

## Mapping

- Before line 79, `dimensions` and `i` of `dimensions[i]`. The ground truth maps them into the
  new declaration: `dimensions` -> the receiver of `dimensions.getOrNull(i)`, `i` -> its argument
  (after line 79). Omnidiff deletes both (`dimensions`: `UnresolvedNode`; `i`:
  `APTED("qualified_name")`).
- Why: the identifiers move from an argument of one statement into a new statement, a
  `property_declaration` with a different shape (`index_expression` -> `call_expression` with a
  `navigation_expression`). APTED inside the function keeps `adjustFile(...)` matched to its after
  twin, and the move out of it is not detected; no later pass picks up the two leftover leaves.
  The ground truth's reading (extract variable) is the reasonable one. `matcher`: move into a new
  statement not detected.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: a moved-leaf pass after APTED (orphaned leaves whose identical twin in a
  new sibling statement is unmapped, e.g. `solve_orphaned_leaves`).
- Ground truth questionable: no
