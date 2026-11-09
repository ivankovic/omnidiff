# Accuracy analysis: rust-sniffnet-protocol

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 895 nodes, 895 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 2861: renderer 0, matcher 0 |
| Painting, Full | 4 bytes wrong of 2861: renderer 4, matcher 0 |
| Ground-truth invariant violations | none |

## The change

From GyulyVGC/sniffnet (`src/networking/types/protocol.rs`, commit 2d9c9a06), kept as a
performance fixture: a variant `ARP` (with doc comment) is added to `enum Protocol`, and to
`Protocol::ALL` (`[Protocol; 3] = [..., Protocol::ICMP]` -> `[Protocol; 4] = [...,
Protocol::ICMP, Protocol::ARP]`, line 22 / 24) and the two tests.

## Painting

The mapping matches the ground truth on every node and Minimal matches its painting exactly.
Full differs by 4 bytes, identical with the human mapping: `Move -> None` on the `];` closing
`Protocol::ALL` (line 22 / after line 24). It was pushed right by the inserted `, Protocol::ARP`
and also moved down two rows by the inserted variant; Full's exemption for a node displaced by
an edit beside it (`shifted_by_an_edit_beside_it` in `identical_or_move`) requires an unchanged
row index under `paint_displaced_moves`, so it paints `Move`. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` `identical_or_move`: Full's row-index condition for
  nodes displaced on their row
- Ground truth questionable: no
