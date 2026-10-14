# Accuracy analysis: rust-zed-workspace-tasks

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 117 (81 visible) of 4075 nodes, 4075 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

zed `crates/workspace/src/tasks.rs`: in the task-spawning method, `if let Some(terminal_provider)
= self.terminal_provider.as_ref() { let task_status = ...spawn(...); let task = cx.spawn(async
|w, cx| { let res = ...; match res { ... } }); ... }` becomes `if self.terminal_provider.is_some()
{ let task = cx.spawn_in(window, async move |workspace, cx| { ...save before spawning... let
spawn_task = ...; if let Some(spawn_task) = ... { let res = ...; match res { ... } } }); ... }`.
The old match moves one level deeper, inside a new `if let`, after ~20 new lines.
`description.md` records it as a performance fixture (1.3 s, 4075 nodes).

## Mapping

All 117 mismatches are `APTED("qualified_name")` in this one block (before lines 76-98, after
lines 78-129); the arms of the `match` itself (before 82-96) are mapped right.

- **Old statements paired with new, unrelated ones, 39 visible + 14 invisible.**
  `let task_status = terminal_provider.spawn(spawn_in_terminal, window, cx);` (before line 77)
  is mapped onto `let save_current = workspace.update_in(...)` / `workspace.save_active_item(...)`
  (after lines 90-92), `self.terminal_provider.as_ref()` (before 76) onto
  `cx.background_spawn(async { ... })` (after 85), and the `let` of `let task` (before 79) onto
  the `let` of `if let Some(save_action)` (after 97). The ground truth deletes the first, keeps
  `self.terminal_provider` in `self.terminal_provider.is_some()` and keeps `let task`. `matcher`.
- **Scaffolding around the moved match deleted, 42 visible + 22 invisible.** `let task =
  cx.spawn(async |w, cx| {` (before 79), `let res = cx.background_spawn(task_status).await;`
  (before 80), `match res {` (before 81), `Some(Err(e)) => {` / `_ = w.update(` (before 89, 91)
  and the closing `}`, `};`, `});` (before 95-98): ground truth maps them to their counterparts
  in the new, deeper structure (`cx.spawn_in(...)`, after 79; `let res`, `match res`, after
  110-111; ...); omnidiff deletes them. Ordered tree edit distance cannot keep an ancestor chain
  once the old children are re-parented under a new `if let` inside new sibling code, so it keeps
  the arm contents and drops the nodes around them. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the `qualified_name` APTED call site: a block re-wrapped one level deeper
  (anchor on the byte-identical `match` arms and recover the enclosing `let` / `match` chain), and
  not pairing statements across unrelated new code
- Ground truth questionable: no
