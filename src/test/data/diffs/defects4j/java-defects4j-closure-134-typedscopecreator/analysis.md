# Accuracy analysis: java-defects4j-closure-134-typedscopecreator

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (0 visible) of 18596 nodes, 18596 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 101454: renderer 0, matcher 0 |
| Painting, Full | 0 bytes wrong of 101454: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-134 (second file of the same commit): `TypedScopeCreator.findOverriddenFunction` also
looks in implemented interfaces: the `if (propType instanceof FunctionType) { ... }` on before
line 558 gains an `else { for (ObjectType iface : ...) { ... } }` branch (after lines 560-570).

## Mapping

- 1 invisible mismatch, no visible ones: the `if_statement` node itself (before 558). The ground
  truth matches it to the extended `if_statement` (MatchButNotIdentical); omnidiff leaves the
  node unmapped (op Delete, `APTED("qualified_name")`) while mapping all of its children as the
  ground truth does. Nothing is painted differently under either preset. `grading-artifact`.

## Painting

No disagreement under either preset.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: none
- Where a fix would go: none needed for readers; the unmapped parent comes from the
  APTED("qualified_name") call in `src/diff/solve_syntax_aware_matching.rs`.
- Ground truth questionable: no.
