# Accuracy analysis: kotlin-yairm210-unciv-yet-another-vector2-removal

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 4416 nodes, 4416 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

A test replaces `Vector2` tile coordinates with `HexCoord`: `getTile(Vector2.Zero)` becomes
`getTile(HexCoord.Zero)`, `Vector2(1f,0f)` becomes `HexCoord(1,0)`, and on line 42
`game.getTile(Vector2(1f, 0f))` becomes `game.getTile(1,0)` (the inner call is unwrapped).

## Mapping

- Before line 42, the `,` of `Vector2(1f, 0f)`. The ground truth maps it to the `,` of
  `getTile(1,0)`, one level up after the `Vector2(...)` wrapper is removed (the `1f` -> `1` and
  `0f` -> `0` pairs across the same boundary are not in the mismatch list, so omnidiff has them).
  Omnidiff deletes the comma (reason `APTED("qualified_name")`), so a reader sees one `,` deleted
  and one inserted. An edit-distance tie on a single punctuation leaf whose parent chain changed;
  the ground truth is the consistent reading, since the arguments on both sides of it are kept.
  `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED call in `solve_qualified_name_groups`, or a punctuation pass that
  maps a separator whose two neighbours are both mapped to neighbours of an unmapped identical
  separator.
- Ground truth questionable: no
