# Accuracy analysis: tsx-keybase-client-change-from-one-import-and-call-to-another

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 3744 nodes, 3744 graded, N:M floor 0 visible |
| Painting | no painting on file |
| Ground-truth invariant violations | none |

## The change

`emoji-row.tsx` stops reading the ordinal through the raw context and uses a hook instead: the
import `{OrdinalContext}` becomes `{useOrdinal}` (line 3), and `const ordinal =
React.useContext(OrdinalContext)` becomes `const ordinal = useOrdinal()` (line 17).

## Mapping

- Before line 17 `React`, 1 visible: the ground truth updates `React`, the object of the callee
  `React.useContext`, into the new callee identifier `useOrdinal`, and deletes `.`, `useContext` and
  the argument `OrdinalContext`. omnidiff deletes `React` (and inserts `useOrdinal`) with reason
  `UnresolvedNode`, which the terminal sweep `solve_unresolved_nodes` records for a node no pass
  decided. The geometry is exactly what `solve_stray_renames` (src/diff/solve_stray_renames.rs)
  is written to undo: a single-token `Update` whose parents are not paired and differ in kind
  (`member_expression` vs `call_expression`) while the before parent lost a named sibling
  (`useContext` is deleted). That pass only unpairs, and leaves the delete and insert to the final
  sweep, which matches the reason seen here. Its doc comment cites
  `java-defects4j-chart-8-week`, where `RegularTimePeriod` of `RegularTimePeriod.DEFAULT_TIME_ZONE`
  renamed into `zone` was treated as a replaced expression. Here the ground truth keeps such a
  rename. Which leaf of the old callee "becomes" `useOrdinal` (`React` or `useContext`, or neither)
  is a judgement call. `ambiguous`.

## Verdict

- Primary cause: `ambiguous`
- Other causes: `ground-truth`
- Where a fix would go: none: ambiguous. `solve_stray_renames` deliberately unpairs this shape, the
  ground truth keeps it; which convention holds is Marko's call
- Ground truth questionable: yes, mildly: it renames a leaf of a replaced member expression into the
  new callee, the shape that `solve_stray_renames` was built to undo after
  `java-defects4j-chart-8-week`; the two fixtures' ground truths may not follow one convention
