# Accuracy analysis: lua-corsixth-corsixth-refactor-if-expressions

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 12 (12 visible) of 4610 nodes, 4610 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `CorsixTH/Lua/rooms/training.lua`, two conditions change from a field-string compare to a
class check:

- line 184: `if humanoid.humanoid_class ~= "Doctor" then` becomes
  `if not class.is(humanoid, Doctor) then`
- line 239: the same change for `"Handyman"`

description.md: "Another case of code partially moving in and out of strings" (`"Doctor"` becomes
the identifier `Doctor`).

## Mapping

- **Dotted name renamed in place instead of keeping `humanoid` (12 visible, 6 per line),
  `matcher`.** The ground truth maps the `humanoid` of `humanoid.humanoid_class` to the argument
  `humanoid` of `class.is(humanoid, ...)`. It deletes `.humanoid_class` and inserts `class.is`.
  omnidiff maps the before `dot_index_expression` onto the after `class.is` dot-index, child by
  child: `humanoid` -> `class` (Update), `.` -> `.`, `humanoid_class` -> `is` (Update). It then
  leaves the argument `humanoid` inserted.

  This comes from the APTED call inside the qualified-name pass (`APTED("qualified_name")` in
  src/diff/solve_syntax_aware_matching.rs), run on the matched `function_declaration`s. With unit
  costs, two renames plus one insert are cheaper than three deletes plus four inserts, so tree
  edit distance prefers the positional pairing. A reader sees `humanoid.humanoid_class` turn into
  `class.is`, and a new `humanoid` argument. The ground truth shows the unchanged identifier kept,
  which is the more natural reading. The string-to-identifier part (`"Doctor"` -> `Doctor`) does
  not appear among the mismatches.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: `APTED("qualified_name")` (src/diff/solve_syntax_aware_matching.rs, APTED
  cost model): prefer an identical leaf pairing over renaming both identifiers of a dotted name
- Ground truth questionable: no
