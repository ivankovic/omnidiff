# Accuracy analysis: kotlin-nextcloud-a-few-small-removals

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 6892 nodes, 6892 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`FileUploadWorker.kt` drops the worker-state broadcast: two imports, three message constants,
three getter functions, `setIdleWorkerState()` and its calls are removed, including
`if (result == Result.success()) { setIdleWorkerState() }` (before lines 146-148) just before
the trailing expression `result` (line 149). One `sendAdded(context)` call is added.

## Mapping

- Before lines 146 and 149, two `result` identifiers in `doWork`'s `try` block. The ground truth
  deletes the `if` with the `result` in its condition (line 146) and keeps the trailing `result`
  (line 149 -> after line 134). Omnidiff maps the `result` inside the deleted condition to after
  line 134 and deletes the trailing one (both entries `APTED("qualified_name")`: the line 146
  pairing is op Identical, the line 149 deletion op Delete). A reader sees the kept `result` line
  as deleted and the `result` inside the removed `if` as kept.
- Why: for APTED both choices cost the same (one identical leaf kept, the other deleted with or
  without its parent), and the tie goes to the earlier twin. The ground truth is the reasonable
  one: the trailing expression is the same statement in the same place. `matcher`: wrong twin.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED call in `solve_qualified_name_groups`
  (src/diff/solve_syntax_aware_matching.rs): tie-break between identical leaf twins in favour of
  the one at the same depth/statement level (a deleted subtree should not keep one leaf).
- Ground truth questionable: no
