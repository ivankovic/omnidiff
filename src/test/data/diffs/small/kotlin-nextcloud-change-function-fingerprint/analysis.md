# Accuracy analysis: kotlin-nextcloud-change-function-fingerprint

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 4 (4 visible) of 2521 nodes, 2521 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

The Compose function `TaskView(task, capability, showTaskActions, showTranslateScreen)` (before
line 52) becomes `TaskView(task, viewModel, capability, showTaskActions)` (after line 55): a
`viewModel` parameter is inserted second and the `showTranslateScreen` callback is removed. Its
call `showTranslateScreen(task)` (before line 63) becomes
`viewModel.updateScreenState(AssistantScreenState.Translation(task))`.

## Mapping

All four have reason `APTED("fast_fallback")`.

- **Commas of the parameter list, line 52 -> 55** (3 visible). The ground truth attaches each
  comma to the parameter in front of it: the comma after `capability: OCCapability` (before
  column 50) maps to the comma after it in after (column 81), the comma after `showTaskActions`
  (column 79) is deleted with the removed last parameter, and the comma after the new `viewModel`
  (after column 55) is inserted. Omnidiff pairs the commas by position (2nd with 2nd, 3rd with
  3rd). Identical punctuation tokens give the LCS no reason to prefer one pairing, and both are
  readable. `ambiguous`.
- **`showTranslateScreen` on line 63** (1 visible). The ground truth updates it to `Translation`
  in `AssistantScreenState.Translation(task)`, the inner call that keeps the `(task)` argument.
  Omnidiff deletes it. Mapping an identifier to a differently named one inside a new wrapper is a
  judgment; deleting the old callback name is also a fair reading. `ambiguous`.

## Verdict

- Primary cause: `ambiguous`
- Other causes: none
- Where a fix would go: none: ambiguous (a convention that a separator comma follows the element
  in front of it would match the ground truth's comma choice, in the LCS fallback,
  `resolve_residual_forest_via_myers_lcs`).
- Ground truth questionable: no
