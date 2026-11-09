# Accuracy analysis: rust-rustdesk-rustdesk-move-list-item-with-insert

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 19342 nodes, 19342 graded, N:M floor 0 visible |
| Painting, Minimal | 76 bytes wrong of 108072: renderer 76, matcher 0 |
| Painting, Full | 80 bytes wrong of 108072: renderer 80, matcher 0 |
| Ground-truth invariant violations | none |

## The change

From rustdesk/rustdesk (`src/lang/ar.rs`, commit 626a091f), a 740-entry translation table: the
entry `("Continue with", "متابعة مع"),` is removed from line 387 and re-added at the end as
`("Continue with {}", "متابعة مع {}"),` (after line 741), i.e. moved and updated.

## Painting

The mapping matches the ground truth on every node; the 76 Minimal and 80 Full bytes are
identical with the human mapping, so all of it is the renderer's.

- `None -> Move` on the moved entry, before line 387 and after line 741: `("Continue with` and the
  Arabic text with its closing `"),` (Minimal; Full includes the indentation and stops before
  `"),`). The painting marks the entry moved, with the two `{}` as inserts. omnidiff paints only
  the inserted `{}`: the entry is one row long and kept its column, so `identical_or_move` reads
  its identical children (`(`, quotes, `,`, `)`) as shifted by an unrelated edit above (the
  `crossed_backwards` reorder check applies only to multi-row nodes), and the updated strings'
  common prefixes are painted `Identical` by `intra_node_update_ranges`, not `Move`, although the
  node moved 354 rows. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs`: `identical_or_move` (`crossed_backwards` for
  single-row nodes) and `intra_node_update_ranges` (the kept affixes of an updated node that
  relocated should paint `Move`)
- Ground truth questionable: no
