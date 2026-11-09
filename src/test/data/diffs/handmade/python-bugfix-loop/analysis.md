# Accuracy analysis: python-bugfix-loop

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 338 nodes, 338 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 1289: renderer 4, matcher 0 |
| Painting, Full | 22 bytes wrong of 1289: renderer 22, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A handmade fixture. In `find_duplicate`, `for num in nums:` (line 5) becomes
`for i in range(len(nums)):` with a new `num = nums[i]` and a comment above the loop (after lines
5-7). A third test case is appended (after lines 22-25). Every node maps as in the ground truth.

## Painting

All runs are the same with the human mapping, so they are all the renderer's. They all sit on the
`for` header (before line 5, after line 6). The row shifts by one because of the inserted comment.

- **Move -> None, `in` on both sides, 4 bytes, Minimal, `renderer`.** `num` -> `i` shortens the
  loop variable, so `in` moves from column 12 to column 10 as well as down one row. omnidiff paints
  this identical, displaced keyword as Move. The Minimal painting leaves it unpainted, like `for`,
  which keeps its column. A reader sees an unchanged `in` flagged as moved.
- **None -> Move, `    for `, ` ` and ` ` (around `in`) on both sides, 20 bytes, Full,
  `renderer`.** The Full painting marks the unchanged parts of the loop header as Match:
  `    for ` and ` in `. omnidiff's Full rendering leaves them unpainted. The unpainted parts are
  indentation, `for`, and the gaps around `in`, which do not change column (or are gap text).
- **Move -> None, the trailing `:` on both sides, 2 bytes, Full, `renderer`.** omnidiff paints the
  displaced `:` as Move. The painting leaves it unpainted, an identical line tail (painting rule
  3).

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: displacement-based Move painting in src/diff/text.rs
  (`identical_or_move`): identical tokens pushed sideways by a narrowed update on the same line
  should stay unpainted in Minimal and for line tails. Full should paint the unchanged header
  parts the way this painting does.
- Ground truth questionable: no
