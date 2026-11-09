# Accuracy analysis: go-fatedier-frp-delete-and-update

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 600 nodes, 600 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 1331: renderer 4, matcher 0 |
| Painting, Full | 140 bytes wrong of 1331: renderer 140, matcher 0 |
| Ground-truth invariant violations | 2 |

## The change

An RPC test helper stops wrapping the connection in a `bufio.Reader`. The `"bufio"` import (line
4), the comment and `rd := bufio.NewReader(r)` (lines 19-21) are deleted, and the two uses of `rd`
become `r` (before lines 23 and 27 -> after lines 19 and 23).

## Painting

Every run is the same with the human mapping, so none of them is the matcher's.

- Full, `Move -> None`, 140 bytes: the unchanged text after `rd` -> `r` on before lines 23-25
  and 27 and after lines 19-21 and 23. That is `, binary.BigEndian, &length); err != nil {`, the
  `return nil, err` and `}` rows of the same if, and `, buffer)`. All of it is shifted (one
  column left on the edited rows, and three rows up). Full's displaced-move rendering paints it
  grey Move. The Full painting paints only the `rd`/`r` pair. Why the unshifted-column rows
  `return nil, err` and `}` are Move too was not traced further. `renderer`.
- Minimal, `None -> Update`, 4 bytes: the `r` of `rd` (before) and the `r` (after), on both uses.
  The painting updates the whole identifier `rd` -> `r`. Omnidiff narrows the update to the `d`
  it lost. Invariant [16] flags the painting's `r` on after lines 19 and 23: a Minimal painting
  should leave the shared part of an identifier alone. `ground-truth`.

## Verdict

- Primary cause: `renderer`
- Other causes: `ground-truth`
- Where a fix would go: Full displaced-move painting in `src/diff/text.rs` (`identical_or_move`,
  `paint_displaced_moves`), for unchanged text that follows a shortened identifier. The Minimal
  bytes are Marko's call.
- Ground truth questionable: yes. The Minimal painting paints the shared `r` of `rd` -> `r` as
  Update. Invariant [16] reports both rows.
