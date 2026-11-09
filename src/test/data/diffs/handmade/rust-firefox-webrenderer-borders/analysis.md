# Accuracy analysis: rust-firefox-webrenderer-borders

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 4916 nodes, 4916 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 23332: renderer 4, matcher 0 |
| Painting, Full | 4 bytes wrong of 23332: renderer 4, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Real-world file from mozilla/firefox (`gfx/wr/webrender/src/prim_store/borders.rs`, commit
df7d2a75), kept as a performance fixture: four `common.prim_rect.size()` become
`common.prim_size`, and four expected sizes in `test_struct_sizes` shrink (216 -> 208, 104 -> 96
twice, 88 -> 80; lines 363-367).

## Painting

The mapping matches the ground truth on every node. The 4 wrong bytes (both presets) are present
with the human mapping too:

- `None -> Update`, the leading `2` of `216` -> `208` (line 363) and the leading `8` of `88` ->
  `80` (line 367), on both sides. Both paintings mark the whole number as updated; omnidiff's
  `intra_node_update_ranges` strips the common prefix of the two literals and paints only `16` /
  `08` and `8` / `0`. Under Full, `whole_identifier_updates` widens names but not numeric
  literals. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` `intra_node_update_ranges`: treat a numeric literal as
  a whole token (do not split a number at a shared digit)
- Ground truth questionable: no
