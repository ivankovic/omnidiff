# Accuracy analysis: css-wordpress-wordpress-reformat-and-fix-lint-errors

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 80 nodes, 80 graded, N:M floor 0 visible |
| Painting, Minimal | 170 bytes wrong of 264: renderer 170, matcher 0 |
| Painting, Full | 134 bytes wrong of 264: renderer 134, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A minified two-rule stylesheet is expanded onto six lines. The lint fix inserts `*` into the second
selector (`.wp-block-post-comments-form .block-editor-warning *` becomes
`.wp-block-post-comments-form *.block-editor-warning *`). Both declarations gain a `;`.

## Painting

All runs are the same with the human mapping, so they are the renderer's. omnidiff and both
paintings agree that the `*` is inserted.

- Minimal, 169 bytes, Move -> None: before line 1 from the first `{` (column 30) to the end, except
  the space before `.block-editor-warning`, and after lines 3-5. The Minimal painting leaves the
  reflow unpainted, as in the other WordPress reformat fixtures. On after line 5 the inserted `;`
  lies inside the Move range of `pointer-events: auto;`, so it is not painted Insert. `renderer`.
- Minimal, 1 byte, None -> Insert: the inserted `;` at the end of after line 2 is left unpainted.
  The mapping is exact, so the node is inserted. Minimal sets `structural_punctuation: false`
  (src/diff/text/render_options.rs), which drops a range made only of brackets and separators (`;`
  is in `STRUCTURAL_PUNCTUATION`). The Minimal painting paints the inserted `;`. `renderer`.
- Full, 134 bytes. The Full painting marks the reformatted declarations as Update
  (`pointer-events:none` -> `pointer-events: none`; the space is not a token), the selector parts
  and braces as Move, and the new indentation and `;` as Insert. omnidiff's labels differ per side.
  On the first declaration it paints Move on before line 1 but nothing on after line 2 (22 bytes).
  It leaves the second rule unpainted on before line 1 (`.block-editor-warning
  *{pointer-events:auto`, 44 bytes) but paints it Move on after lines 4-5. It also leaves the after
  `{` on line 1 and the space before `*` on line 4 unpainted, where the painting has Move. The
  displaced-node exemptions in `identical_or_move` compare each node with its counterpart from that
  side's point of view, so they do not have to agree. Why each side resolved as it did was not
  traced further. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `identical_or_move` in src/diff/text.rs: reflow under Minimal, and symmetric
  Move decisions for a mapped pair under Full. Also the Minimal `structural_punctuation` filter in
  src/diff/text/render_options.rs, which drops an inserted lone `;`.
- Ground truth questionable: no
