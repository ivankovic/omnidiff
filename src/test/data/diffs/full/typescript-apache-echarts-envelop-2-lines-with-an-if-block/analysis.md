# Accuracy analysis: typescript-apache-echarts-envelop-2-lines-with-an-if-block

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 6 (4 visible) of 10064 nodes, 10064 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `TimeScale`, the two statements of the `each(ticks, tick => { ... })` callback (before lines
209-210) are wrapped in a new `if (tick.time) { ... }` and indented one level (after lines
209-212).

## Mapping

All 6 mismatches are the braces of the two blocks (4 visible: `{` and `}` on each side; 2
invisible: the `statement_block` nodes).

- The ground truth keeps the arrow function's body block (`{` on line 208, column 29; `}` on line
  211, column 9 before / 213 after) as itself, and treats the `if`'s block `{ ... }` (after line
  209, column 28 to line 212, column 13) as new, with the two statements moved into it.
- omnidiff maps the old arrow-function body to the new `if` block instead (reason
  `IdenticalHashOfAncestor` for the braces, the block re-tagged `WrapGrowth`), and leaves the new
  arrow-function body as inserted. The old block and the new `if` block hold the same two
  statements and hash identically (the indentation is not part of the hash), so the identical-hash
  pass took them as a pair before structure could decide. The result shows the unchanged
  `tick => {` brace as inserted and the new `if` brace as old. The ground truth's reading matches
  what happened. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: identical-hash matching (`IdenticalHash` pass) / `solve_wrap_growth`: when
  a block's identical twin is the body of a new wrapper inside the block's own matched parent,
  keep the block with its parent and move the contents.
- Ground truth questionable: no
