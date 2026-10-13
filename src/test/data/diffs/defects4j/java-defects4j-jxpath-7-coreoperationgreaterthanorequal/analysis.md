# Accuracy analysis: java-defects4j-jxpath-7-coreoperationgreaterthanorequal

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 36 (22 visible) of 316 nodes, 316 graded, N:M floor 0 visible |
| Painting, Minimal | 102 bytes wrong of 2994: renderer 0, matcher 102 |
| Painting, Full | 134 bytes wrong of 2994: renderer 0, matcher 134 |
| Ground-truth invariant violations | none |

## The change

Defects4J JxPath-7. `CoreOperationGreaterThanOrEqual` loses its own `computeValue(EvalContext)`
(which compared two doubles with `>=`) and the two imports it needed. It gets a `protected boolean
evaluateCompare(int compare)` hook (`return compare >= 0;`) for the shared logic in
`CoreOperationRelationalExpression`. The same commit makes the same change in
`java-defects4j-jxpath-7-coreoperationgreaterthan`, `java-defects4j-jxpath-7-coreoperationlessthan`,
`java-defects4j-jxpath-7-coreoperationlessthanorequal`.

## Mapping

All 36 mismatches (22 visible) have one cause, `matcher`. The ground truth deletes the method
`public Object computeValue(EvalContext context) { ... }` (lines 34-38) with all its descendants,
and inserts `protected boolean evaluateCompare(int compare) { ... }` (lines 32-34) as new. omnidiff
maps the two `method_declaration`s, their bodies, parameter lists, `return` statements and binary
expressions to each other. At the leaves it pairs `public` -> `protected`, `Object`
(type_identifier) -> `evaluateCompare` (identifier), `EvalContext` (type_identifier) -> `compare`
(identifier), `l` -> `compare`, and the `(`, `)`, `{`, `}`, `return`, `>=` and `;` tokens. All 36
have reason `APTED("qualified_name")`. The methods' names differ, so the pairing is not by their
names. The class `CoreOperationGreaterThanOrEqual` keeps its name, and in the class body the tree
edit distance aligns the middle method of before (`computeValue`, between the constructor and
`getSymbol`) with the middle method of after (`evaluateCompare`). I did not trace which pair the
APTED call was made on.

The two methods share no name, return type, parameter or statement. Only the slot in the class body
and the `return a >= b;` shape are common. Two of the pairings also cross node kinds (a type to an
identifier), which shows that the alignment is driven by position, not content. The ground truth's
reading (the comparison moved to the superclass, and a new hook was added) is the one a reader would
give.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Painting

All runs are matcher-caused. Rendering the human mapping gives 0 wrong bytes in both presets. The
painting marks both methods whole, as Delete and Insert. omnidiff paints `public Object ` /
`protected ` as Update, and leaves the matched punctuation, `return`, `>=`, `;` and the closing
braces unpainted. Under Full some of these are painted Move. This is 102 bytes in Minimal and 134 in
Full, on lines 34, 37, 38 (before) and 32-34 (after). `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: `APTED("qualified_name")` (`solve_syntax_aware_matching`), which aligns the
  two method declarations in the class body; it would need to reject a method pair that shares only
  the position and punctuation
- Ground truth questionable: no
