# Accuracy analysis: csharp-sonarr-sonarr-move-plus-add

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 2867 nodes, 2867 graded, N:M floor 0 visible |
| Painting, Minimal | 95 bytes wrong of 11996: renderer 95, matcher 0 |
| Painting, Full | 18 bytes wrong of 11996: renderer 18, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The custom-format controller moves its `GetAll()` action (with its `[HttpGet]` and `[Produces]`
attributes, before lines 74-79) up to after lines 50-55, and adds two bulk actions in the space and
further down: `Update` with `[HttpPut("bulk")]` (after lines 81-101) and `DeleteFormats` with
`[HttpDelete("bulk")]` (after lines 109-117). `GetPresets()` also gains `.ToList()` (line 123).

## Painting

Mapping is perfect and every run is identical with the human mapping, so all of it is
renderer-owned.

Minimal, 95 bytes:

- Move -> None, 88 bytes: the indentation inside the moved `GetAll` block, before lines 75-79 and
  after lines 51-55 (8 or 12 spaces per row). The Minimal painting paints the move one entry per
  row, from the first real character of each row. omnidiff paints the whole six-row method as one
  Move range; Minimal trims whitespace only at the two ends of a range (rule 4), so the leading
  indentation of every row after the first stays painted. `renderer`.
- None -> Insert, 7 bytes: lone `{` / `}` rows of the inserted `Update` and `DeleteFormats` bodies
  and of the `if` and lambda blocks in `Update` (after lines 85, 87, 89, 94, 101, 112, 116),
  dropped by the Minimal structural-punctuation filter. `renderer`.

Full, 18 bytes:

- None -> Move, 16 bytes: the 8-space indentation of the first moved row (`[HttpGet]`, before line
  74 / after line 50). The Full painting's single Move entry starts at column 0; omnidiff's Move
  range starts at `[`. `renderer`.
- Move -> None, 2 bytes: the `;` of `var presets = GetPresets();` (before line 92 / after line
  123), pushed right by the inserted `.ToList()`. The `shifted_by_an_edit_beside_it` exemption of
  `identical_or_move` (src/diff/text.rs) needs the same row index under Full, and the inserted
  methods shift the line by 31 rows, so Full paints it Move. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Minimal's whitespace trimming in src/diff/text/render_options.rs, which
  leaves inner-row indentation inside a multi-row Move range; the Minimal structural-punctuation
  filter; and `identical_or_move`'s row-index gate under Full
- Ground truth questionable: no
