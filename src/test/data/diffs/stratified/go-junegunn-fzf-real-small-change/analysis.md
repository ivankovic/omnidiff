# Accuracy analysis: go-junegunn-fzf-real-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 148 nodes, 148 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 433: renderer 2, matcher 0 |
| Painting, Full | 3 bytes wrong of 433: renderer 3, matcher 0 |
| Ground-truth invariant violations | none |

## The change

fzf's Windows tty helper. `import "os"` becomes the grouped `import ( "os" )` (after lines 5-7).
`TtyIn` now returns `(*os.File, error)`, and a new `TtyOut` function is added (after lines 14-21).

## Painting

All runs are the same with the human mapping, so they are the renderer's.

- Minimal, `None -> Insert`, 2 bytes: the `(` (after line 5) and `)` (after line 7) of the new
  import group. Each sits alone in its range, so Minimal's structural-punctuation filter drops
  both. A pair that is dropped together is not restored. In the painting they are the only
  visible sign of the regrouping, and it inserts them. (The `(` and `, error)` of the new result
  list are painted by both.) `renderer`.
- Full, `Move -> None`, 2 bytes: the `{` of `func TtyIn() ... {` (before line 12, after line 14),
  shifted right by the wrapped result type. Omnidiff paints it alone as Move; the Full painting
  leaves it unpainted. `renderer`.
- Full, `None -> Insert`, 1 byte: the new indentation tab before `"os"` (after line 6). The
  painting inserts it, and omnidiff does not paint it. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Minimal structural-punctuation filter in
  `src/diff/text/render_options.rs`, for an inserted bracket pair that is the whole edit. For
  Full: a lone shifted `{` painted Move, and inserted indentation.
- Ground truth questionable: no
