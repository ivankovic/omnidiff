# Accuracy analysis: rust-real-logic-change-in-a-huge-75k-node-file

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 44 (31 visible) of 153129 nodes, 5519 graded, N:M floor 0 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

Handmade on a 9,100-line Rust file (the `human_solver` TUI): adds a sample triage status
(`SampleTriageStatus`, reject action, modal and tests), 593 lines added and 153 removed over ~45
hunks. Among them the `HELP_TEXT` string constant grows (before lines 198-251, after 205-), and
`list_samples_with_status` changes its doc comment and its return type `Vec<(String, bool)>` ->
`Vec<(String, SampleTriageStatus)>` (before lines 508-513, after 540-545).

## Mapping

The ground truth grades only 5,519 of 153,129 nodes, so most of the file is ungraded and the
result elsewhere is not checked. All 44 mismatches are in two places.

- `HELP_TEXT` (15 visible, 3 invisible), before lines 198-251: the ground truth matches the
  `const_item` and its `string_literal` (`MatchButNotIdentical`), keeping `const`, the name, the
  type, the quotes, escapes and `;` identical and updating the string contents. omnidiff deletes
  the whole constant (reason `APTED("fast_fallback")`, the Myers-LCS alignment of the residual
  forest used when whole-tree APTED is too costly for a file this size), so the unchanged name
  and type are reported as removed. `matcher`.
- `list_samples_with_status` doc comment (15 visible, 10 invisible), before lines 508-512: the
  ground truth matches each of the five `///` lines to the five rewritten ones (after lines
  540-544), keeping `//` and `/` identical and updating the text. omnidiff deletes them, also via
  `APTED("fast_fallback")`. The first line keeps its opening words; the others are rewritten
  enough that delete+insert is defensible, but the pairing is the reader's natural reading.
  `matcher`.
- `bool` -> `SampleTriageStatus` (1 visible), before line 513: the ground truth updates the
  `primitive_type` into a `type_identifier`; omnidiff deletes it (reason `APTED("qualified_name")`),
  since the update crosses node kinds. `matcher`.

## Painting

None: the fixture has no painting.

## Verdict

- Primary cause: `matcher`
- Other causes: `grading-artifact`
- Where a fix would go: `apted::for_roots_fallback` / `resolve_residual_forest_via_myers_lcs`
  (pair residual items whose head is identical: a `const_item` with the same name and type, doc
  comments above the same function); APTED cost of `primitive_type` -> `type_identifier`
- Ground truth questionable: no (but it grades under 4% of the nodes, so 0 mismatches elsewhere
  means little)
