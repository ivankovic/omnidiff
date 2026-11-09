# Accuracy analysis: rust-zed-industries-zed-add-only-2

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 247586 nodes, 247586 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 1183465: renderer 0, matcher 0 |
| Painting, Full | 16 bytes wrong of 1183465: renderer 16, matcher 0 |
| Ground-truth invariant violations | none |

## The change

zed `crates/project/src/lsp_store.rs` (~590 KB): a `cx: &mut AsyncApp` parameter is threaded
through: one argument `cx,` (after line 551), one parameter line (after line 3775), `, cx` in
`initialization_options(delegate, cx)` (after line 3778) and `_: &mut AsyncApp,` (after line
13991). Insertions only.

## Painting

- **Full, Move -> None, 16 bytes:** `).await?` on before line 3777 / after line 3778. The inserted
  `, cx` pushes the tail right, and the parameter inserted above shifts the line down two rows;
  Full paints the unchanged tail Move, the painting leaves it alone (an identical line tail). The
  same with the human mapping. `renderer`.

Minimal agrees with the painting byte for byte; the mapping has no mismatches.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` Full displacement handling for a line tail shifted in both
  row and column by inserts
- Ground truth questionable: no
