# Accuracy analysis: java-defects4j-closure-31-compiler

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 30122 nodes, 30122 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 147302: renderer 5, matcher 5 |
| Painting, Full | 0 bytes wrong of 147302: renderer 7, matcher 7 |
| Ground-truth invariant violations | none |

## The change

Removes the middle conjunct `!options.skipAllPasses &&` (line 1285) from the condition
`needsManagement() && !options.skipAllPasses && options.closurePass` (lines 1284-1286), so
dependency management runs even when all passes are skipped.

## Mapping

The before condition parses as `(needsManagement() && !skipAllPasses) && closurePass`; the after one
as `needsManagement() && closurePass`. One of the two `&&` survives.

- Ground truth: the outer `&&` (line 1285, after `skipAllPasses`) maps to the after `&&`, and the
  inner `&&` (line 1284, after `needsManagement()`) is deleted. This keeps the operator of the outer
  `binary_expression`, which is the node that maps to the after `binary_expression`.
- omnidiff (`APTED("large_flat_subtree")`): maps the line-1284 `&&` Identical to the after `&&` and
  deletes the line-1285 one. This keeps the operator that stays on the same line and column.

Both readings are defensible: the ground truth follows the tree, omnidiff follows the text. 2
visible mismatches, `ambiguous`.

## Painting

omnidiff's own rendering matches both paintings exactly (0 bytes). The disagreement exists only when
the HUMAN mapping is rendered: the Minimal and Full paintings both delete `!options.skipAllPasses
&&` as one span, that is the line-1285 `&&`, which is the operator the human mapping keeps. Rendered
from the human mapping, the line-1284 `&&` is painted Delete (2 bytes) and ` &&` on line 1285 is
left unpainted (3 bytes); Full also paints the after `&&` Move (2 bytes). These 5/7 renderer bytes
are not seen by a reader of omnidiff's output.

## Verdict

- Primary cause: `ambiguous`
- Other causes: `ground-truth`
- Where a fix would go: none: ambiguous; the mapping/painting contradiction in the ground truth is
  Marko's call
- Ground truth questionable: yes: The human mapping keeps the line-1285 && but both paintings delete
  it with !options.skipAllPasses; the invariant check does not flag it.
