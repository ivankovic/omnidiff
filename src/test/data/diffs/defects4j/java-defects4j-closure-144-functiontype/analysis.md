# Accuracy analysis: java-defects4j-closure-144-functiontype

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 10476 nodes, 10476 graded, N:M floor 0 visible |
| Painting, Minimal | 10 bytes wrong of 55056: renderer 0, matcher 10 |
| Painting, Full | 10 bytes wrong of 55056: renderer 0, matcher 10 |
| Ground-truth invariant violations | none |

## The change

Closure-144: `FunctionType.cloneWithNewReturnType` passes the function's `source` node to the
new `FunctionType` instead of `null`: line 517 `registry, null, null,` becomes
`registry, null, source,`.

## Mapping

- The third argument on line 517, 1 visible: the ground truth (tree and its single painting)
  treats `null` -> `source` as an in-place update of the argument. omnidiff deletes the
  `null_literal` and inserts the `identifier` (op Delete, `APTED("large_flat_subtree")`): the
  node kinds differ, so APTED's rename cost exceeds delete + insert. The argument slot is the
  same and the ground truth is consistent with itself. `matcher`.

## Painting

- Matcher, both presets, 10 bytes: `null` painted Delete and `source` Insert where the painting
  has Update on both. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the `large_flat_subtree` APTED pass
  (`src/diff/solve_large_flat_subtrees.rs`) and its rename cost: a single leaf replaced in the
  same argument position, with matched neighbours on both sides, should be an update even across
  node kinds. (Note the opposite convention in `java-defects4j-cli-8-helpformatter`, where the
  painting deletes and inserts an identifier -> literal replacement.)
- Ground truth questionable: no.
