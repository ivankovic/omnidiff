# Accuracy analysis: java-defects4j-closure-144-functiontypebuilder

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 7475 nodes, 7475 graded, N:M floor 0 visible |
| Painting, Minimal | 5 bytes wrong of 42709: renderer 0, matcher 5 |
| Painting, Full | 9 bytes wrong of 42709: renderer 2, matcher 7 |
| Ground-truth invariant violations | none |

## The change

Closure-144 (second file of the same commit): `FunctionTypeBuilder` leaves the return type `null`
when there is no `@return` (line 201-202 becomes `... : null;`), guards the template check with
a new operand line `returnType != null &&` (after 203), adds `inferReturnStatements(...)` (after
214-244), and passes `returnTypeInferred` to `withReturnType` (line 496 -> 527).

## Mapping

- The `&&` operators of `templateTypeName != null && returnType.restrict...()` (before 203,
  after 203-204), 2 visible. The chain is left-nested, so the inserted middle operand makes the
  old `&&` structurally the outer one, which now ends the new line 204; textually it stays at the
  end of line 203. The ground truth keeps the line-203 `&&` and inserts the new line's `&&` with
  it. omnidiff maps the old `&&` to the new line's `&&` (op Identical,
  `APTED("large_flat_subtree")`) and inserts the line-203 one. `matcher`. (The same pattern as
  `java-defects4j-closure-131-tokenstream`, there via APTED("qualified_name").)

## Painting

- Matcher, Minimal 5 / Full 7 bytes: the line-203 `&&` painted Insert (and the before one Move
  in Full), the new line's ` &&` left unpainted where the painting has Insert. `matcher`.
- Renderer, Full 2 bytes (survive with the human mapping): the `)` of
  `.withReturnType(returnType)` (before 496, after 527) painted Move after its column shifted;
  the painting leaves it alone. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: the `large_flat_subtree` APTED pass
  (`src/diff/solve_large_flat_subtrees.rs`): operand insertion into a left-nested binary chain
  should keep identical operators at their textual position.
- Ground truth questionable: no.
