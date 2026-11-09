# Accuracy analysis: c-postgres-real-logic-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 124 (82 visible) of 15327 nodes, 15025 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

postgres `slotsync.c`: `update_local_synced_slot` loses its two out-parameters. The
"WAL not flushed" check (`remote_slot->confirmed_lsn > latestFlushPtr`), which existed twice in
`synchronize_one_slot`, moves into `update_local_synced_slot`. The
`if (!found_consistent_snapshot) { ereport(...) }` block moves from
`update_and_persist_local_synced_slot` into the `else` branch of `update_local_synced_slot`.
description.md: "Requires N:M multi-map for complete solution". The ground truth leaves the N:M
part ungraded: the new latestFlushPtr block (after lines 213-230) and its two before copies (lines
768-794 and 872-898) have no entries. They are among the 302 ungraded nodes.

## Mapping

All 124 mismatches are on the before side. They fall in three groups.

### Skeletons of the three code-exchanging functions deleted (114: 75 visible, 39 invisible)

The three changed functions are `update_local_synced_slot` (before lines 205-360),
`update_and_persist_local_synced_slot` (lines 617-690) and `synchronize_one_slot` (lines 696-904).
In these functions the nodes that earlier passes did not map are all set to Delete with reason
`APTED("qualified_name")`. The ground truth has them Identical or MatchButNotIdentical to their
after counterparts. The affected nodes are: the `function_definition`, `function_declarator` and
`parameter_list` nodes; `bool`, the function name and its parentheses; `if`, `else`, `{` and `}` of
the changed if-statements; the unchanged comments in them (for example
`/* Slot not ready yet, let's attempt to make it sync-ready now. */` at line 796); and the
call-site tokens `update_local_synced_slot ( remote_slot , remote_dbid` at lines 626 and 822. The
shell of the moved `if (!found_consistent_snapshot)` block (lines 657, 658 and 669: `if`, `{`, `}`)
is in this group too. Its body is mapped into the after file's `update_local_synced_slot`, which
agrees with the ground truth. By function: 28 visible in the first, 18 in the second, 29 in the
third. No other function in the file has a mismatch.

All three are exactly the functions whose bodies have descendants mapped to another function (before
`update_and_persist...` to after `update_local...`: the ereport block and `return false;`; before
`synchronize_one_slot` to after `update_local...`: the `latestFlushPtr` declaration, which the
ground truth also moves). `solve_syntax_aware_matching` pairs functions by qualified name and runs
`apted::for_nodes(..., "qualified_name")` on each pair. In `apted/common/resolve.rs`,
`ContainmentCtx` vetoes a pairing whose pruned (already mapped) descendants landed outside the other
side's subtree. `qualified_name` is in `PREMATCH_SIBLING_ORDER_SOURCES`, so this call also gets the
sibling-order check. Cross-function moves leave each of these function pairs with descendants mapped
outside their partner. The pattern fits this veto: the root and everything under it that is not yet
mapped is deleted, and that includes leaves such as `bool` with no mapped descendants. I did not
trace which check rejects the leaves. The `skip_reason` assignment below is mapped inside the
`update_local_synced_slot` pair with reason `APTED("qualified_name")`, so not every unmapped node of
that pair is left deleted. `matcher`.

### `return false;` at line 668 (3 visible, 1 invisible)

The ground truth deletes the `return false;` of the removed `if (!found_consistent_snapshot)` block
in `update_and_persist_local_synced_slot` (`delete_with_children`). omnidiff maps it to `return
false;` at after line 229, which ends the new latestFlushPtr block in `update_local_synced_slot`.
That block is ungraded. Its text comes from the two before copies in `synchronize_one_slot`, and the
second copy (line 897) ends in the same `return false;`. So omnidiff took the twin from the wrong
block. The pairing is `MovedSubtree` (op Identical): the move pass matched the deleted `return
false;` across functions to a byte-identical statement. Because the destination is ungraded, this is
one-sided. `matcher`.

### `skip_reason = SS_SKIP_NO_CONSISTENT_SNAPSHOT;` at line 330 (4 visible, 2 invisible)

The ground truth deletes before's `if (found_consistent_snapshot && !(*found_consistent_snapshot))
skip_reason = SS_SKIP_NO_CONSISTENT_SNAPSHOT;` (lines 329-330) with all its children. omnidiff maps
the assignment statement to the identical `skip_reason = SS_SKIP_NO_CONSISTENT_SNAPSHOT;` at after
line 344 (`APTED("qualified_name")`, op Identical). Both lines are in `update_local_synced_slot`
(before lines 205-360, after lines 199-416), so this is APTED on that function pair carrying the
statement from the old `if` into the new one, not a move pass. After line 344 is inside the new `if
(!found_consistent_snapshot) { ... }`. The ground truth has entries for the `Assert` (insert) and
the ereport (moved from `update_and_persist...`) in that block, but none for this assignment. The
before line 330 is the only identical source for it. omnidiff's move is the natural reading. The
ground truth deletes the source and leaves the destination ungraded. `ground-truth`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Verdict

- Primary cause: `matcher`
- Other causes: `ground-truth`
- Where a fix would go: `apted::for_nodes` from `solve_syntax_aware_matching` (call site
  `"qualified_name"`), in `ContainmentCtx` (containment / sibling-order veto in
  `apted/common/resolve.rs`). When a function pair has descendants moved to another function, its
  unmapped skeleton should still pair. For `return false;`, the twin choice of `MovedSubtree`.
- Ground truth questionable: yes: the after `skip_reason = SS_SKIP_NO_CONSISTENT_SNAPSHOT;` at line
  344 is ungraded. Its only identical source (before line 330) is graded as deleted, so omnidiff's
  move cannot score.
