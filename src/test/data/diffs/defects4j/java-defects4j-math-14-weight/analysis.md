# Accuracy analysis: java-defects4j-math-14-weight

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 10 (8 visible) of 583 nodes, 583 graded, N:M floor 0 visible |
| Painting, Minimal | 31 bytes wrong of 4686: renderer 0, matcher 31 |
| Painting, Full | 31 bytes wrong of 4686: renderer 0, matcher 31 |
| Ground-truth invariant violations | none |

## The change

Defects4J Math-14. The `Weight(double[])` constructor stops building a dense matrix and filling its
diagonal in a loop. It now assigns `weightMatrix = new DiagonalMatrix(weight);`. The
`createRealMatrix(dim, dim)` call and the `for` loop are removed.

## Mapping

- **Replaced right-hand side partly kept (8 visible, 2 invisible), `matcher`.** On line 43 the
  ground truth deletes `org.apache.commons.math3.linear.MatrixUtils.createRealMatrix(dim, dim)` (a
  `method_invocation`) with all its descendants, and inserts `new DiagonalMatrix(weight)` (an
  `object_creation_expression`) whole. omnidiff maps `org` (identifier) -> `DiagonalMatrix`
  (type_identifier), the first `dim` -> `weight`, the argument lists `(dim, dim)` -> `(weight)`, and
  their `(` and `)`. All ten print `APTED("qualified_name")`: the qualified-name pre-match
  (`solve_qualified_name_groups`) pairs the enclosing `Weight` declarations by name and resolves
  them with APTED, which relabels the right-hand side's leaves instead of deleting and inserting.
  The parents are different node kinds, and the two expressions share no name. The pairing follows
  only from the position of the assignment's right-hand side.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Painting

All 31 bytes in each preset are matcher-owned: rendering the human mapping gives 0. `org` is painted
Update instead of Delete. `(dim` and `)` are left unpainted instead of Delete. On the after side, `
DiagonalMatrix(weight)` is unpainted instead of Insert.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the APTED resolution of the qualified-name pre-match
  (`APTED("qualified_name")`, `src/diff/solve_syntax_aware_matching.rs`), which maps leaves under
  a `method_invocation` to leaves under an `object_creation_expression`
- Ground truth questionable: no
