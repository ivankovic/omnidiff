# Accuracy analysis: rust-rustdesk-rustdesk-actual-logic-change-in-io-loop-medium-sized-file

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 74 (46 visible) of 46728 nodes, 46728 graded, N:M floor 0 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

From rustdesk/rustdesk (`src/client/io_loop.rs`, commit 5faf0ad3), a 2,300-line file: adds a
terminal connection type. Among other hunks, the `conn_type` chain `if is_file_transfer() {
FILE_TRANSFER } else if is_view_camera() { VIEW_CAMERA } else { default }` (before lines
158-163) gains a branch `else if self.handler.is_terminal() { ConnType::TERMINAL }` (after lines
156-163), and the four-way `let is_conn_not_default = ... || ...;` plus `if !is_conn_not_default {`
(before lines 198-202) collapse into `if self.handler.is_default() {` (after line 198).

## Mapping

- The `else if` chain (41 visible, 23 invisible), before lines 158-162 / after lines 156-162:
  the ground truth keeps both old branches identical and inserts the new `else if is_terminal()`
  branch. omnidiff matches the whole old chain, `if A {..} else if B {..} else {..}`, to the new
  chain's tail starting one level deeper, `if B {..} else if C {..} else {..}`, which has the same
  shape (reason `StructurallyIdenticalSubtrees` on the root, `StructurallyIdenticalAncestor` on
  its descendants). The result is a chain of updates shifted by one branch: `is_file_transfer` ->
  `is_view_camera`, `FILE_TRANSFER` -> `VIEW_CAMERA`, `is_view_camera` -> `is_terminal`,
  `VIEW_CAMERA` -> `TERMINAL`, and the new branch's nodes paired with old ones. The structural
  matcher picks a same-shape nested subchain instead of the outer `if` with one inserted
  `else if` (the else-if nesting case). `matcher`.
- The `is_default()` collapse (5 visible, 5 invisible), before lines 198-202: the ground truth
  maps the four `self.handler.is_*()` calls as a 4:1 group onto `self.handler.is_default()`
  (expecting one pair) and keeps the `if` whose condition changed. omnidiff pairs none of the four
  calls with the new one, and deletes the old `if` (with its `if_expression` and statement) via
  `APTED("qualified_name")`, although only its condition changed. `matcher`.

## Painting

None: the fixture has no painting.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the structural-hash matcher (`StructurallyIdenticalSubtrees`) on `else if`
  chains: prefer the outer `if` with an inserted branch over a same-shape nested subchain;
  `APTED("qualified_name")` keeping an `if` whose condition alone changed
- Ground truth questionable: no
