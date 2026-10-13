# Accuracy analysis: java-defects4j-jxpath-7-coreoperationgreaterthan

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 36 (22 visible) of 316 nodes, 316 graded, N:M floor 0 visible |
| Painting, Minimal | 100 bytes wrong of 2944: renderer 0, matcher 100 |
| Painting, Full | 132 bytes wrong of 2944: renderer 0, matcher 132 |
| Ground-truth invariant violations | none |

## The change

Defects4J JxPath-7. `CoreOperationGreaterThan` loses its own `computeValue(EvalContext)` (which
compared two doubles with `>`) and the two imports it needed. It gets a `protected boolean
evaluateCompare(int compare)` hook (`return compare > 0;`) for the shared logic in
`CoreOperationRelationalExpression`. The same commit makes the same change in
`java-defects4j-jxpath-7-coreoperationgreaterthanorequal`,
`java-defects4j-jxpath-7-coreoperationlessthan`,
`java-defects4j-jxpath-7-coreoperationlessthanorequal`.

## Mapping

All 36 mismatches (22 visible) have one cause, `matcher`. The ground truth deletes the method
`public Object computeValue(EvalContext context) { ... }` (lines 33-37) with all its descendants,
and inserts `protected boolean evaluateCompare(int compare) { ... }` (lines 31-33) as new. omnidiff
maps the two `method_declaration`s, their bodies, parameter lists, `return` statements and binary
expressions to each other. At the leaves it pairs `public` -> `protected`, `Object`
(type_identifier) -> `evaluateCompare` (identifier), `EvalContext` (type_identifier) -> `compare`
(identifier), `l` -> `compare`, and the `(`, `)`, `{`, `}`, `return`, `>` and `;` tokens. The
pass is `APTED("qualified_name")` for all 36: the APTED call that `solve_qualified_name_groups`
(`solve_syntax_aware_matching.rs`) runs on a pair matched by kind and scope-qualified name. The two
methods have different names, so that pair is an enclosing named node (the class
`CoreOperationGreaterThan`), and APTED inside it aligned the two methods by their slot.

The two methods share no name, return type, parameter or statement. Only the slot in the class body
and the `return a > b;` shape are common. Two of the pairings also cross node kinds (a type to an
identifier), which shows that the alignment is driven by position, not content. The ground truth's
reading (the comparison moved to the superclass, and a new hook was added) is the one a reader would
give.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Painting

All runs are matcher-caused. Rendering the human mapping gives 0 wrong bytes in both presets. The
painting marks both methods whole, as Delete and Insert. omnidiff paints `public Object ` /
`protected ` as Update, and leaves the matched punctuation, `return`, `>`, `;` and the closing
braces unpainted. Under Full some of these are painted Move. This is 100 bytes in Minimal and 132 in
Full, on lines 33, 36, 37 (before) and 31-33 (after). `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the `APTED("qualified_name")` call on the matched class pair
  (`solve_qualified_name_groups`), which pairs the two method declarations; it would need to reject
  a method pair that shares only the position and punctuation
- Ground truth questionable: no
