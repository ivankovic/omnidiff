# Accuracy analysis: java-genymobile-scrcpy-refactor-for-loop-in-a-function

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 46 (32 visible) of 3043 nodes, 3043 graded, N:M floor 0 visible |
| Painting | none on file |
| Ground-truth invariant violations | none |

## The change

In scrcpy's `LogUtils`, the inline "collect the upper bounds of the FPS ranges into a `TreeSet`"
loop (lines 106-109) is extracted into a new `private static SortedSet<Integer>
getUniqueSet(Range<Integer>[] ranges)` (lines 138-144), and called as `getUniqueSet(lowFpsRanges)`.
The commit also lists high-speed capture sizes, using the same helper. No painting on file.

## Mapping

All 46 mismatches (32 visible) have reason `APTED("qualified_name")`, the whole-method APTED call
that `solve_syntax_aware_matching` runs per qualified-name group.

- **Extracted code not followed into the helper (26 visible, 12 invisible), `matcher`.** The ground
  truth maps `new TreeSet<>()` (line 106) to the helper's `new TreeSet<>()` (line 139), and the `for
  (Range<Integer> range : lowFpsRanges) { uniqueLowFps.add(range.getUpper()); }` loop (lines
  107-109) to the helper's loop (lines 140-142), with `uniqueLowFps` -> `set` as an Update. omnidiff
  deletes all of it and inserts the helper body. The helper `getUniqueSet` is a new method, so it
  has no qualified-name partner. The APTED call on the enclosing method cannot see it, and nothing
  later recovers statements that moved into a different method.
- **Cross-kind leftovers in the old method (6 visible, 2 invisible), `matcher`.** Inside the old
  method, the same APTED call pairs the old `TreeSet` (type_identifier, line 106) with the new call
  name `getUniqueSet` (identifier, line 107). It pairs the empty `()` of the constructor call with
  `(lowFpsRanges)`, and its parens with the call's parens. The ground truth inserts the
  `getUniqueSet(...)` call and maps the loop's iterable `lowFpsRanges` (line 107) to the call's
  argument. omnidiff deletes that identifier.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: cross-method move detection for code extracted into a new method; the
  `APTED("qualified_name")` scope is one same-named method, and no later pass looks for subtrees
  moving into a newly inserted method
- Ground truth questionable: no
