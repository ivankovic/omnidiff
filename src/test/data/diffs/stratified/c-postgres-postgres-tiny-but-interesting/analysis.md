# Accuracy analysis: c-postgres-postgres-tiny-but-interesting

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 18225 nodes, 18225 graded, N:M floor 0 visible |
| Painting, Minimal | 7 bytes wrong of 108217: renderer 7, matcher 0 |
| Painting, Full | 10 bytes wrong of 108217: renderer 10, matcher 0 |
| Ground-truth invariant violations | none |

## The change

postgres `pqcomm.c`, line 181: `port = palloc0(sizeof(Port));` becomes
`port = palloc0_object(Port);`. The function name grows a suffix and the `sizeof(...)` wrapper
around the argument goes.

## Painting

The mapping is perfect and every run is identical with the human mapping: all renderer.

- Minimal, `Update -> Insert`, 7 bytes, `_object` on line 181 after: `palloc0` ->
  `palloc0_object` is an identifier Update, which omnidiff narrows to the appended `_object`. The
  before side of that narrowed part is empty, so it is a pure insertion. The painting labels it
  Insert; omnidiff keeps the Update label. `renderer`.
- Full, `None -> Move`, 10 bytes, `Port` and the outer `)` on line 181 (both sides): the Full
  painting marks `Port` Move because it left `sizeof(...)` for the call's argument list, and the
  outer `)` Move because it shifted. omnidiff paints neither. `Port` is at the same row and column
  on both sides, and `identical_or_move` (`src/diff/text.rs`) decides by position, not by parent.
  The `)` only slid sideways beside an edit on its row. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `intra_node_update_ranges` in `src/diff/text.rs` (label a narrowed update
  part with an empty side as Insert/Delete); Full handling of reparented nodes that keep their
  position in `identical_or_move`
- Ground truth questionable: no
