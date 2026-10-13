# Accuracy analysis: php-theseer-directoryscanner-add-two-test-cases-and-reformat-file

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 2282 nodes, 2282 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

The test class is reindented from 3- to 4-space steps throughout (159 lines in, 142 out), and two
new test methods, `testSettingInvalidFlagThrowsException` and
`testUnSettingInvalidFlagThrowsException`, each with an `@expectedException` docblock, are added
before `testSetIncludes` (after lines 62-76).

## Mapping

- 2 visible, one wrong pairing. The ground truth maps the docblock of `testSetIncludes` (before
  lines 63-66, "Set the include to be matching for *.txt via setIncludes") to the same docblock
  after the reindent (after lines 80-83), an Update because the comment's inner indentation
  changed, and treats the first new `@expectedException` docblock (after lines 62-65) as inserted.
  omnidiff pairs the old docblock with that new `@expectedException` docblock instead (Update,
  reason `APTED("qualified_name")`, the APTED run inside the matched class). Both pairings keep
  sibling order and both are leaf renames; the reindent means neither candidate is byte-identical,
  so the cost apparently could not tell the near-identical text from the unrelated one, and the
  earlier candidate won. The ground truth's pairing is plainly the right one. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED leaf rename cost within `solve_syntax_aware_matching`
  (`qualified_name`): weigh comment text similarity (or compare comments with inner whitespace
  normalised) so a reindented comment beats an unrelated one.
- Ground truth questionable: no
