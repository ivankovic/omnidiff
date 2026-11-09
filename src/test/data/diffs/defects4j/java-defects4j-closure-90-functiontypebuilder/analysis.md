# Accuracy analysis: java-defects4j-closure-90-functiontypebuilder

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 9368 nodes, 9368 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 51689: renderer 2, matcher 0 |
| Painting, Full | 2 bytes wrong of 51689: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Line 184: `if (!type.isSubtype(` becomes `if (!type.restrictByNotNullOrUndefined().isSubtype(`, so a
nullable `this` type is restricted before the subtype check.

## Painting

One renderer group, the same in both presets and with the human mapping, 2 bytes: `Move -> None` on
the `.` before `isSubtype` (line 184, both sides). The single painting inserts
`.restrictByNotNullOrUndefined()` and leaves the outer call's `.` unpainted. The tree mapping keeps
that `.` with the outer `method_invocation`, which is right, but its object `type` went one level
deeper into the new inner call, and omnidiff paints the shifted `.` grey Move although `isSubtype`
beside it, shifted the same way, stays unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs move painting of a punctuation leaf whose sibling was
  re-parented
- Ground truth questionable: no
