# Accuracy analysis: rust-add-if

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 139 nodes, 139 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 276: renderer 0, matcher 0 |
| Painting, Full | 3 bytes wrong of 276: renderer 3, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Handmade. `if number % 2 == 0 { ... } else { ... }` (line 3) gets a new first branch: it becomes
`if number == 0 { println!("Zero"); } else if number % 2 == 0 { ... }`, so the old `if` moves to
the tail of after line 5, behind `} else `.

## Painting

The mapping matches the ground truth everywhere and Minimal matches its painting exactly. Full
differs by 3 bytes, identical with the human mapping, so all of it is the renderer's:

- `None -> Delete`, before line 3, the 2 bytes of indentation in front of the old `if`, and `None
  -> Move`, after line 5, the space between `else` and `if` (1 byte). The Full painting accounts
  for the old indentation byte by byte: column 0 deleted, column 1 moved to the gap before the
  relocated `if`. omnidiff paints the moved `if` statement itself but never the whitespace in
  front of a moved node. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full painting of the indentation left behind by a node that moved inside
  its own row, in `src/diff/text.rs`; small enough to leave
- Ground truth questionable: no (the split of the old indentation into a `Delete` and a 1-byte
  `Move` is an unusual but consistent Full reading)
