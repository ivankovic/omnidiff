# Accuracy analysis: rust-protocolbuffers-protobuf-add-two-attributes

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 860 nodes, 860 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 4204: renderer 2, matcher 2 |
| Painting, Full | 2 bytes wrong of 4204: renderer 1, matcher 1 |
| Ground-truth invariant violations | none |

## The change

From protocolbuffers/protobuf (`rust/upb/sys/wire/wire.rs`, commit 665c1a34): adds the line
`#[allow(unused)] // C struct values used in FFI.` above two enums (after lines 20 and 33).

## Painting

The mapping matches the ground truth on every node. The single painting (`Only one solution`)
inserts each new line whole, columns 0-48.

- After line 20, `None -> Insert`, 2 bytes Minimal (1 Full), with the human mapping too: the space
  between `]` and `//` (column 16) and, in Minimal, the space after `//` (column 19). The attribute
  and the comment are separate inserted nodes; omnidiff paints each and leaves the gaps between
  them unpainted. `renderer`.
- After line 33, the same 2 bytes (1 Full), but attributed to the matcher: the human file marks
  this line's comment `insert_with_children` (the first one is `insert` plus `//`), and with that
  the renderer paints the line whole; omnidiff's mapping inserts the same nodes and the gaps stay.
  No graded node differs, so this is the same renderer gap, decided by how the insertion is
  expressed. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs`: paint the whitespace between adjacent inserted nodes
  on one row (and after `//`) the same way whether or not the insert is "with children"
- Ground truth questionable: no (the two identical insertions are encoded differently, `insert`
  vs `insert_with_children`, which changes only how they render)
