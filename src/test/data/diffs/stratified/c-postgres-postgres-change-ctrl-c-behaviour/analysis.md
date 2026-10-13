# Accuracy analysis: c-postgres-postgres-change-ctrl-c-behaviour

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1393 nodes, 1393 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 17693: renderer 4, matcher 0 |
| Painting, Full | 25 bytes wrong of 17693: renderer 25, matcher 0 |
| Ground-truth invariant violations | none |

## The change

postgres `walwriter.c`: the walwriter now ignores SIGINT.
`pqsignal(SIGINT, SignalHandlerForShutdownRequest);` (line 106) becomes
`pqsignal(SIGINT, SIG_IGN);	/* no query to cancel */`. The last three lines of the block comment
above (lines 101-103, ` *`, `* We have no particular use for SIGINT ...`, `* reasonable to treat
like SIGTERM.`) are removed.

## Painting

The mapping is perfect and every run is identical with the human mapping: all renderer.

- Both presets, the shortened block comment, lines 101 and 104 before, 2 bytes Minimal (`*`), 6
  bytes Full (`\t *`): the comment is one token mapped as an Update. The painting deletes the three
  whole lines 101-103. omnidiff's prefix/suffix narrowing slides the deleted span by one line: it
  keeps line 101's ` *` (as a common prefix) and deletes the ` *` that begins the closing ` */` on
  line 104. The two are the same text edit; the painting's line-aligned one is what a reader
  expects. `renderer`.
- Minimal, `None -> Update`, 2 bytes, the `S` of `SignalHandlerForShutdownRequest` / `SIG_IGN` on
  line 106 / 103: the painting updates the whole identifier. omnidiff's narrowing keeps their one
  shared leading character unpainted and splits the word. `renderer`.
- Full, `None -> Move`, 18 bytes, `(SIGINT, ` on line 106 / 103: the Full painting marks the
  unchanged start of the call Move (the statement moved up three rows). omnidiff treats a row-only
  shift as an unrelated edit above and leaves it unpainted. `renderer`.
- Full, `None -> Insert`, 1 byte, the tab before the new trailing comment on line 103 after: not
  painted by omnidiff. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `intra_node_update_ranges` in `src/diff/text.rs` (line-aligned deletion
  inside a block comment; no split inside an identifier for a 1-character shared prefix); Full
  row-shift rule in `identical_or_move`
- Ground truth questionable: no
