# Accuracy analysis: java-defects4j-closure-112-typeinference

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (0 visible) of 22330 nodes, 22330 graded, N:M floor 0 visible |
| Painting, Minimal | 87 bytes wrong of 114002: renderer 87, matcher 0 |
| Painting, Full | 104 bytes wrong of 114002: renderer 104, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-112: `TypeInference.inferTemplatedTypesForCall` keeps only the template keys of the
function: the initialiser `inferTemplateTypesFromParameters(fnType, n)` (before line 1193) is
wrapped in `Maps.filterKeys(..., new Predicate<TemplateType>() { ... })` (after lines 1192-1200).
The wrapped call stays on line 1193 at the same column.

## Mapping

- 2 invisible mismatches, no visible ones. omnidiff maps the before `method_invocation`
  `inferTemplateTypesFromParameters(fnType, n)` to the new outer call `Maps.filterKeys(...)`
  (MatchButNotIdentical, `APTED("large_flat_subtree")`) instead of to its identical copy inside
  it; the ground truth inserts the outer call. All leaves map as the ground truth says, so a
  reader sees no difference. `grading-artifact` (counted only because parent nodes are graded).

## Painting

- None -> Move, Minimal 86 bytes / Full 102: the wrapped call on line 1193. Both paintings carry
  an explicit `Match` of `inferTemplateTypesFromParameters(fnType, n)` (and, in Full, its
  indentation) to the identical text at the same row and column. Identical sides grade as Move;
  omnidiff, with either mapping, leaves text that did not change or move unpainted.
  `grading-artifact`.
- Renderer, 1-2 bytes (survive with the human mapping): Minimal leaves the inserted `,` after the
  wrapped call (after line 1193) unpainted where the painting has Insert; Full paints the
  statement's `;` Move (it now follows the closing `)` on after line 1200), which the Full
  painting leaves unpainted. `renderer`.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: `renderer`
- Where a fix would go: none in the engine for the bulk (grader: an in-place identical Match
  graded as Move); the outer-call parent mapping is in the `large_flat_subtree` APTED pass if it
  matters.
- Ground truth questionable: no.
