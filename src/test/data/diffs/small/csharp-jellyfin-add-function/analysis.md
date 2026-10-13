# Accuracy analysis: csharp-jellyfin-add-function

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 55 (36 visible) of 4647 nodes, 4651 graded, N:M floor 3 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

Jellyfin's `LimitedConcurrencyLibraryScheduler` gets two new private methods,
`ShouldForceSequentialOperation()` and `CalculateScanConcurrencyLimit()` (after lines 99-118).
They are built from code that was inline: the
`_serverConfigurationManager.Configuration.LibraryScanFanoutConcurrency` read and the
`Environment.ProcessorCount` fallback (before lines 103-104), and the `== 1` check (before line
226). `var parallelism = ...` becomes
`var operationFanout = Math.Max(0, CalculateScanConcurrencyLimit() - _taskRunners.Count)`.
`description.md`: "Requires N:M (2:2) mapping."

## Mapping

All 55 mismatches (36 visible) come from `APTED("qualified_name")`. APTED runs within each
method, so content that moves into a newly added method can only be deleted and inserted.

- Config access and `Environment.ProcessorCount`, 29 visible: 5 leaves for each of the 4 copies
  of `_serverConfigurationManager.Configuration.LibraryScanFanoutConcurrency` (before 103, 226;
  after 102, 109), and 3 leaves for each of the 3 copies of `Environment.ProcessorCount` (before
  104; after 103, 113). The ground truth maps them all-to-all (2:2 and 1:2). Omnidiff deletes
  every before copy and inserts every after copy. The N:M floor is 3 visible: one-to-one, one
  after copy of `Environment.ProcessorCount` must stay unmatched. The other 26 could have been
  paired. `matcher` / `nm-floor`.
- `var fanoutConcurrency = ...;` (before line 103), 4 visible (`var`, name, `=`, `;`). The ground
  truth moves the declaration into the new `ShouldForceSequentialOperation` as
  `var fanoutSetting = ...` (after line 102, name updated). Omnidiff deletes it. Same cause:
  cross-method move. `matcher`.
- `var parallelism = ...` (before line 104), 3 visible (`var`, name, `=`). Omnidiff updates it in
  place to `var operationFanout = ...` (after line 123). The ground truth deletes it. The two uses
  of `parallelism` on the next lines are not in the mismatch list, so the ground truth maps them to
  `operationFanout` as omnidiff does. Pairing the declaration with them is at least as
  defensible. `ambiguous`; see the verdict.

## Verdict

- Primary cause: `matcher`
- Other causes: `nm-floor`, `ambiguous`
- Where a fix would go: cross-method move detection for code extracted into new methods. It would
  run after the per-function APTED at the `qualified_name` call site
  (`solve_syntax_aware_matching.rs`).
- Ground truth questionable: yes, mildly. It renames both uses of `parallelism` to
  `operationFanout` but deletes the declaration instead of pairing it with
  `var operationFanout = ...`.
