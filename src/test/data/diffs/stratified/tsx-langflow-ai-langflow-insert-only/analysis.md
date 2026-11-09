# Accuracy analysis: tsx-langflow-ai-langflow-insert-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 250 nodes, 250 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 740: renderer 2, matcher 0 |
| Painting, Full | 10 bytes wrong of 740: renderer 10, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The Gmail icon component wraps its `<GmailIconSVG ref={ref} {...props} />` in a styled `<span
style={{…}}>`, so the one-line `return <GmailIconSVG … />;` (before line 7) becomes a parenthesised
multi-line return (after lines 7-19). Only insertions; the element itself moves into the span.

## Painting

Mapping is perfect and every run is identical with the human mapping, so all of it is
renderer-owned.

- Minimal, 2 bytes, None -> Insert: the inserted `(` after `return` (after line 7) and its `)`
  (after line 19). Both are lone structural punctuation on their range, so the Minimal filter
  drops both halves of the pair (`is_structural_only`, src/diff/text/render_options.rs); the
  painting paints them. `renderer`.
- Full, 2 bytes, Move -> None: the `;` of the return statement (before line 7 / after line 19).
  The statement grew around it, so the `;` changed row and column and omnidiff paints it Move. The
  painting leaves it alone. `renderer`.
- Full, 8 bytes, None -> Insert: the new 8-space indentation in front of the moved
  `<GmailIconSVG …/>` (after line 18). The Full painting inserts it; omnidiff paints the element
  itself as the Move and leaves the indentation in front of it unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Minimal structural-punctuation filter (src/diff/text/render_options.rs)
  for an inserted bracket pair; `identical_or_move` (src/diff/text.rs) for a `;` displaced by its
  statement growing; Full's painting of new indentation in front of a moved node
- Ground truth questionable: no
