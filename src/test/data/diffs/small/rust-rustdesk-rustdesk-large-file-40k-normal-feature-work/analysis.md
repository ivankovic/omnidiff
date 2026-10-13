# Accuracy analysis: rust-rustdesk-rustdesk-large-file-40k-normal-feature-work

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 60 (40 visible) of 91000 nodes, 91000 graded, N:M floor 0 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

From rustdesk/rustdesk (`src/server/connection.rs`, commit 5faf0ad3), a 4,500-line file: the
same terminal-connection feature as the `io_loop` fixture, 112 lines added and 14 removed.
`description.md` says only "In progress" (the fixture has no painting yet).

## Mapping

- The `else if` chain (26 visible, 10 invisible), before lines 1254-1256 / after lines 1265-1267:
  `... else if self.view_camera { (3, ViewCamera) } else { (0, Remote) }` gains a branch `else if
  self.terminal { (4, AuthConnType::Terminal) }`. The ground truth keeps the `view_camera` branch
  identical and inserts the new one. omnidiff matches the old `if self.view_camera {..} else
  {..}` to the new `if self.terminal {..} else {..}`, the same-shape subchain one level deeper
  (reason `StructurallyIdenticalSubtrees`, descendants `StructurallyIdenticalAncestor`), so it
  reports `view_camera` -> `terminal`, `3` -> `4`, `ViewCamera` -> `Terminal` and pairs the new
  branch's nodes with the old ones. The same else-if nesting case as
  `rust-rustdesk-rustdesk-actual-logic-change-in-io-loop-medium-sized-file`. `matcher`.
- A copied statement (12 visible, 8 invisible): `self.keyboard = false;` (before line 1516) stays
  at after line 1533 and a copy appears in the new `else if self.terminal` branch (after line
  1526). The ground truth maps the original 1:1 and inserts the copy; omnidiff maps the old
  statement to both (reason `MultiMap`), reading the copy as a duplication. The multi-map pass
  accepts a 1:2 here that the ground truth does not. `matcher`.
- `portable_check`'s condition (2 visible, 2 invisible), before lines 3565-3569 / after line 3615:
  `a || b || c || d || !self.keyboard` becomes `a || !self.is_remote() || !self.keyboard`. The
  ground truth keeps the `||` in front of the unchanged `!self.keyboard` (before line 3569) and
  deletes the one on before line 3567; omnidiff, through `APTED("qualified_name")` on the
  re-nested left-associative `binary_expression`s, maps the line 3567 `||` to that position and
  deletes the line 3569 one. `matcher`.

## Painting

None: the fixture has no painting.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: structural-hash matching (`StructurallyIdenticalSubtrees`) of `else if`
  chains with an inserted branch; `solve_multi_maps` accepting a copy of a statement as a 1:2
  duplication; APTED pairing of operators in a shortened `||` chain
- Ground truth questionable: no
