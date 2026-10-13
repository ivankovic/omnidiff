# Accuracy analysis: go-cloudflare-cfssl-change-zero-value-to-is-zero

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 5 (5 visible) of 2736 nodes, 2736 graded, N:M floor 0 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

A vendored Prometheus client file: line 237 `if ts == (time.Time{}) {` becomes
`if ts.IsZero() {`. Nothing else changes.

## Mapping

All five mismatches are on line 237, all from `APTED("qualified_name")` (the per-function APTED
call in `solve_syntax_aware_matching.rs`).

- `Time` (type_identifier inside the composite literal) -> `IsZero` (field_identifier of the new
  selector). The ground truth deletes `Time` and inserts `IsZero`; omnidiff relabels one to the
  other as an Update (2 visible: one per side). The names share nothing and are different node
  kinds; APTED's leaf rename cost made the relabel cheaper than delete+insert. `matcher`.
- `(`, `.`, `)` of `(time.Time{})`: the ground truth maps them to the `.` of `ts.IsZero` and the
  `(` `)` of the argument list, so only `time`, `Time{}` and `==` read as deleted. Omnidiff
  deletes them, because their parents (parenthesized_expression, qualified_type) have no partner
  in the after tree (3 visible). The ground truth's choice keeps the characters a reader sees
  unchanged; omnidiff's is the strict structural reading. Counted as `matcher`, close to
  `ambiguous`.

## Verdict

- Primary cause: `matcher`
- Other causes: `ambiguous` (the punctuation pairing across unrelated parents)
- Where a fix would go: APTED leaf relabel cost for unrelated identifiers of different kinds
  (`apted::for_nodes`, `qualified_name` call site)
- Ground truth questionable: no
