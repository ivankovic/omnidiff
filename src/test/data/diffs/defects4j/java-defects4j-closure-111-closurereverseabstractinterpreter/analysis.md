# Accuracy analysis: java-defects4j-closure-111-closurereverseabstractinterpreter

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 4042 nodes, 4042 graded, N:M floor 0 visible |
| Painting, Minimal | 15 bytes wrong of 17992: renderer 0, matcher 15 |
| Painting, Full | 26 bytes wrong of 17992: renderer 5, matcher 22 |
| Ground-truth invariant violations | none |

## The change

Closure-111: for `goog.isArray`, `ClosureReverseAbstractInterpreter` no longer returns the top
type unchanged when it is the all type: line 54 `return topType;` becomes
`return topType.isAllType() ? getNativeType(ARRAY_TYPE) : topType;` (after lines 54-55).

## Mapping

- The returned `topType` (before 54:18), 2 visible. After the change, `topType` occurs twice: as
  the receiver of `isAllType()` at the same position (54:18) and as the ternary's fallback
  (55:43). The ground truth reads the change as wrapping the old return value in a ternary and
  matches it to the fallback; omnidiff matches it to the receiver at the same position (op
  Identical, `APTED("qualified_name")`) and inserts the fallback. Both are identical identifiers;
  one reading is "the old value is still what is returned otherwise", the other "the line keeps
  its start and gains a tail". `ambiguous`.

## Painting

- Matcher, Minimal 15 bytes / Full 22: the consequence of the twin choice. omnidiff leaves the
  receiver `topType` on after line 54 unpainted (the painting: Insert) and paints the fallback
  ` topType` on line 55 Insert (the painting: Move). In Full the before ` topType;` is also
  unpainted where the painting has Move. `ambiguous`.
- Renderer, Full 5 bytes (survive with the human mapping): the spaces in front of `topType` and
  the `;` of the return, which the Full painting assigns differently (Move/Insert for the spaces,
  None for `;`). `renderer`.

## Verdict

- Primary cause: `ambiguous`
- Other causes: `renderer`
- Where a fix would go: none: ambiguous (a preference for the "wrapped value" reading would go in
  the APTED("qualified_name") resolution, `src/diff/solve_syntax_aware_matching.rs`).
- Ground truth questionable: no.
