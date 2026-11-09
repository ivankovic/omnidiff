# Accuracy analysis: rust-adding-a-variable-and-test-with-comments

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 42096 nodes, 42096 graded, N:M floor 0 visible |
| Painting, Minimal | 13 bytes wrong of 249112: renderer 12, matcher 1 |
| Painting, Full | 746 bytes wrong of 249112: renderer 746, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Handmade, on a 1600-line Rust file. Adds `let paint_row_len = line.trim_end().len();` with a
six-line comment in `render_code_row` (after lines 1209-1215), changes one argument `row_len` to
`paint_row_len` (after line 1229), and adds a commented test (after lines 1615-1634).

## Painting

The mapping matches the ground truth on every node.

Renderer-caused (identical with the human mapping):

- Minimal, 6 bytes, `None -> Insert`: the space after `//` on each of the six new comment lines
  (after lines 1209-1214, column 6). The painting inserts each comment line whole from `//`;
  omnidiff leaves that one space unpainted between the comment marker and its text.
- Minimal, 6 bytes, `Update -> Insert`, after line 1229 `paint_`: both sides paint only the new
  prefix of `row_len` -> `paint_row_len`, but the painting labels it an `Insert`, omnidiff a
  narrowed `Update`. `renderer`.
- Full, 746 bytes, `Move -> None`: the `).map(` tail of before line 1222 / after line 1229 and the
  whole multi-row closure argument under it (before lines 1223-1232, after lines 1230-1239). The
  longer `paint_row_len` pushes the arguments node 6 columns right, and the inserted lines above
  push it 7 rows down. `TextRanges::identical_or_move` exempts such a displaced multi-row node
  (`displaced_beside_an_edit_on_its_first_row`, whose comment names this fixture) only when
  `paint_displaced_moves` is off, i.e. under Minimal; Full paints it `Move` by design, and the
  Full painting does not. `renderer`.

Matcher-attributed, 1 byte (Minimal): the space after `//` on after line 1632, inside the new
test. The human file marks the test function `insert_with_children`, and under it the renderer
paints the comment whole; omnidiff inserts the same nodes and the renderer leaves the gap after
`//`, as on the six lines above. No graded node differs: it is the same gap, decided by how the
insertion is expressed. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` `identical_or_move` (Full's `paint_displaced_moves`
  for a multi-row node displaced by an edit on its first row); line-comment painting of the gap
  after `//`
- Ground truth questionable: no
