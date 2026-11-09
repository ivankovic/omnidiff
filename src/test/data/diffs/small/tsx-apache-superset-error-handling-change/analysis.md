# Accuracy analysis: tsx-apache-superset-error-handling-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 7 (5 visible) of 5292 nodes, 5293 graded, N:M floor 1 visible |
| Painting | no painting on file |
| Ground-truth invariant violations | none |

## The change

`DatasourceModal` imports `isFeatureEnabled` and `FeatureFlag` (after lines 26-27), adds dataset
folders to the save payload behind a feature flag (after lines 178-181), and makes the Save button's
tooltip show the validation errors: the `: ''` branch (before line 359) becomes `: errors.length > 0
? errors.join('\n') : ''` (after lines 365-367). Marko's description.md: "Requires an N:M mapping. A
rare case of 1:2." The ground truth maps the before `:` to both after `:`s.

## Mapping

- After line 367 `:`, 1 visible: the second half of the 1:2 group for the before `:` of line 359.
  A one-to-one mapping can give the before `:` to only one of the two after colons. `nm-floor`.
- Before line 359 `''`, 4 visible + 2 invisible: the ground truth keeps the empty string as the
  final `''` of the new nested ternary (after line 367). omnidiff maps it, with its two quotes, to
  the `'\n'` argument of `errors.join('\n')` on after line 366 (MatchButNotIdentical / Identical,
  reason `APTED("qualified_name")`, the method-body APTED call of `solve_qualified_name_groups`).
  Both after strings sit inside the inserted inner ternary, so neither choice breaks ancestry, and
  under unit costs the two mappings cost the same: keeping `''` -> `''` and inserting `'\n'` (4
  nodes) against `''` -> `'\n'` (insert the escape sequence) plus inserting the trailing `''` (3
  nodes). APTED's tie-break takes the earlier twin, which leaves the identical `''` reported as
  new. `matcher` (wrong twin in a cost tie).

## Verdict

- Primary cause: `matcher`
- Other causes: `nm-floor`
- Where a fix would go: tie-breaking of the method-body APTED call (`solve_qualified_name_groups`,
  src/diff/solve_syntax_aware_matching.rs; `apted::for_nodes`): among equal-cost mappings prefer the
  byte-identical twin
- Ground truth questionable: no
