# Accuracy analysis: c-nginx-nginx-enclose-function-call-in-if-return-value-check

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 64580 nodes, 64580 graded, N:M floor 0 visible |
| Painting, Minimal | 65 bytes wrong of 264001: renderer 65, matcher 0 |
| Painting, Full | 5 bytes wrong of 264001: renderer 5, matcher 0 |
| Ground-truth invariant violations | none |

## The change

nginx `ngx_event_openssl.c`: the bare statement `SSL_CTX_set_tmp_dh(ssl->ctx, dh);` on line 1379
becomes the condition of a new `if (... != 1) { ... return NGX_ERROR; }` error-handling block.

## Painting

All runs are identical with the human mapping, so they belong to the renderer.

- Minimal, `Move -> None`, line 1379, 65 bytes (33 before, 32 after): the call
  `SSL_CTX_set_tmp_dh(ssl->ctx, dh)` is mapped identical and stays on its row, but its column moves
  from 4 to 8 because `if (` is inserted in front of it. The Minimal painting leaves it unpainted
  and paints only `if (`, ` != 1) {` and the deleted `;`. `identical_or_move` in
  `src/diff/text.rs` paints it `Move`: the row's content changed around the node (the `;` went,
  ` != 1) {` came), so `node_untouched_on_its_row` does not exempt it, and the column change counts
  as a relocation. The before-side run starts on the call and runs over the `;` (painted Delete in
  the painting). `renderer`.
- Full, `None -> Insert`, line 1379 after, 5 bytes: the Full painting paints the 4 bytes of
  indentation in front of the new `if` and the space between `)` and `!=` as Insert; omnidiff
  leaves both unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `identical_or_move` in `src/diff/text.rs` (Minimal: a call wrapped in place
  into a new condition on the same row) and Full gap/indentation painting of inserted text
- Ground truth questionable: no
