# Accuracy analysis: rust-gyulyvgc-sniffnet-rename-one-identifier

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 36068 nodes, 36068 graded, N:M floor 0 visible |
| Painting, Minimal | 10 bytes wrong of 136765: renderer 10, matcher 0 |
| Painting, Full | 0 bytes wrong of 136765: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

From GyulyVGC/sniffnet (`src/networking/manage_packets.rs`, commit 241c5abc): one field access
changes, `info_traffic_msg.last_packet_timestamp` ->
`info_traffic_msg.last_filtered_packet_timestamp` (line 295).

## Painting

The mapping matches the ground truth on every node, and Full matches its painting exactly. Minimal
differs by 10 bytes on after line 295, identical with the human mapping:

- The Minimal painting marks `_filtered` (columns 41-50) as an `Insert`. omnidiff narrows the
  `field_identifier` update by common prefix `last_` and suffix `packet_timestamp`, painting
  `filtered_` (columns 42-51), and labels it `Update`, because `intra_node_update_ranges` keeps a
  name's one-sided middle an `Update` (only content nodes get `Insert`/`Delete`). The run covers
  `_filtered_`: the label differs on the 8 shared bytes, and the underscores on either end are
  painted by one side only. Where the inserted `_` goes is a free choice (`ambiguous`); the label
  is the renderer's (`renderer`).

## Verdict

- Primary cause: `renderer`
- Other causes: `ambiguous`
- Where a fix would go: `src/diff/text.rs` `intra_node_update_ranges` (`Insert` for a one-sided
  middle of a name under Minimal, and the affix split at an underscore)
- Ground truth questionable: no
