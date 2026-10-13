# Accuracy analysis: swift-apple-swift-argument-parser-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 18 (6 visible) of 1907 nodes, 1907 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `Tests/ArgumentParserPackageManagerTests/Tests.swift`, `import ArgumentParser` (line 12) is
removed and `@testable import ArgumentParser` is added after the other imports (after line 15). As
in the sibling fixture `swift-apple-swift-argument-parser-remove-trimming-lines-function-call`
(same commit), `#if ... / unsetenv("COLUMNS") / #endif` (before lines 18-20) becomes
`Platform.Environment[.columns] = nil` (after line 19).

## Mapping

omnidiff's pairs below are from Insert/Delete-with-children expectations; the import pair comes
from `APTED("fast_fallback")`, the `unsetenv` pair from `APTED("qualified_name")`.

- **`import` -> `@testable import` (4 visible, 4 invisible), `ambiguous`.** The ground truth deletes
  `import ArgumentParser` and inserts `@testable import ArgumentParser`. omnidiff pairs them: the
  `import` keyword and `ArgumentParser` match (`APTED("fast_fallback")`), so the declaration shows
  as moved down three lines and gaining `@testable`. Importing the same module, now with a test
  attribute, can be read as either an edit or a replacement.
- **`unsetenv("COLUMNS")` -> `Platform.Environment[.columns]` (2 visible, 8 invisible),
  `ground-truth`.** Here the ground truth deletes the whole call and inserts the whole assignment.
  omnidiff maps the call to the subscript expression (`APTED("qualified_name")`): `unsetenv` ->
  `Platform` (Update), the argument list and `"COLUMNS"` -> `.columns`. The sibling fixture's
  ground truth, from the same commit with the same before and after text, maps exactly this:
  - `match_but_not_identical` call_expression -> call_expression
  - `update` `unsetenv` -> `Platform`
  - `(` -> `[`, `)` -> `]`

  The two hand mappings disagree on the same edit. omnidiff agrees with the sibling's.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Verdict

- Primary cause: `ambiguous`
- Other causes: `ground-truth`
- Where a fix would go: none for the import. For `unsetenv`, ground truth (Marko's call): the two
  fixtures from commit 6924db87 treat the identical edit differently
- Ground truth questionable: yes. `unsetenv("COLUMNS")` -> `Platform.Environment[.columns]` is
  delete+insert here but an update in
  `swift-apple-swift-argument-parser-remove-trimming-lines-function-call`
