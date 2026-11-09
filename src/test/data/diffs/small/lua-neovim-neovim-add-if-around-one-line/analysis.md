# Accuracy analysis: lua-neovim-neovim-add-if-around-one-line

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (0 visible) of 15684 nodes, 15684 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `test/functional/testnvim.lua`, `function M.stop()` had a one-statement body,
`assert(session):stop()`. The commit wraps that statement in `if loop_running then ... end` (after
lines 323-325).

## Mapping

- **The function's body block paired with the new `if` block (0 visible, 2 invisible),
  `matcher`.** Before, the function's `block` holds only `assert(session):stop()`, so it is
  text-identical to the new `if`'s inner `block`. The ground truth keeps the function body block
  as the function body block, and the `if` block is new. omnidiff re-tags the before block as an
  identical match for the `if`'s block (op Identical, reason `WrapGrowth`, `solve_wrap_growth`).
  It leaves the outer body block unmatched. The wrapped statement and all leaves map as in the
  ground truth, so the reader sees the same diff. The only cost is a body block that changes
  parent level.

## Verdict

- Primary cause: `matcher` (invisible only)
- Other causes: none
- Where a fix would go: `solve_wrap_growth` (`WrapGrowth`): when the wrapped node is a container
  whose parent is matched, keep it on its parent's partner and re-tag only its statements
- Ground truth questionable: no
