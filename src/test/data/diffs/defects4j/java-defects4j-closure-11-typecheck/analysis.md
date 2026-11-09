# Accuracy analysis: java-defects4j-closure-11-typecheck

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 25077 nodes, 25077 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 134513: renderer 10, matcher 10 |
| Painting, Full | 0 bytes wrong of 134513: renderer 10, matcher 10 |
| Ground-truth invariant violations | none |

## The change

Closure-11: `TypeCheck.visitGetProp` drops the early exit for assignments: the branch
`} else if (n.getJSType() != null && parent.isAssign()) { return;` (before lines 1314-1315) is
removed from the `if` / `else if` chain.

## Mapping

- The two `else` keywords of the chain (before lines 1314 and 1316; after line 1314 keeps one).
  The ground truth's tree mapping deletes the first `else` (line 1314) and keeps the second
  (1316 -> 1314). omnidiff keeps the first and deletes the second (`APTED("large_flat_subtree")`,
  Identical / Delete). The ground truth's own paintings (Minimal and Full) do what omnidiff does:
  they delete `if (...) {` on line 1314, `return;`, and `} else` on line 1316, leaving line
  1314's `else` unpainted. Rendering the human mapping paints `else ` on line 1314 Delete and
  leaves ` else` on line 1316 alone (10 renderer bytes per preset); omnidiff's mapping matches
  both paintings exactly. Which `else` survives is a free choice, but the tree mapping and the
  painting of this fixture make opposite choices. `ground-truth` (2 visible).

## Painting

No disagreement with omnidiff's mapping under either preset (0 real bytes). The 10 renderer bytes
are the human tree mapping rendered against its own painting, described above.

## Verdict

- Primary cause: `ground-truth`
- Other causes: none
- Where a fix would go: ground truth (Marko's call); nothing in the engine.
- Ground truth questionable: yes: the tree mapping keeps the line-1316 `else`, both paintings
  keep the line-1314 `else`.
