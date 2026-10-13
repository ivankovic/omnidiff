# Accuracy analysis: python-pytorch-pytorch-add-param-to-many-places-and-update-one

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 8937 nodes, 8937 graded, N:M floor 0 visible |
| Painting, Minimal | 48 bytes wrong of 38652: renderer 48, matcher 0 |
| Painting, Full | 64 bytes wrong of 38652: renderer 64, matcher 0 |
| Ground-truth invariant violations | 2 (one per painting, same leaf pair) |

## The change

In `torch/nn/attention/_fa3.py`, a new `block_table: torch.Tensor | None = None` parameter is added
to three functions and threaded through their calls. In one call the positional placeholder
`None,  # page_table,` (line 299) becomes `block_table,  # block_table,` (after line 301). The
mapping agrees with the ground truth everywhere.

## Painting

- **Update -> Delete/Insert (Minimal 48 bytes), None -> Delete/Insert (Full 64 bytes), before line
  299 and after line 301, `ground-truth`.**
  - The human tree mapping pairs the argument `None` -> `block_table` (update), the `,` (same
    text) and the comment `# page_table,` -> `# block_table,` (update).
  - omnidiff renders exactly that, and the same runs appear with the human mapping.
  - Both paintings instead paint the whole line as deleted and re-inserted, the `,` included. In
    Full the indentation is painted too.

  invariants.txt flags this ([10], in both paintings): the painting paints a leaf pair (`,`, before
  row 299 col 12, after row 301 col 19) as gone and new while the tree mapping calls it the same
  text. The ground truth contradicts itself here, and no matcher or renderer change can satisfy
  both halves.

## Verdict

- Primary cause: `ground-truth`
- Other causes: none
- Where a fix would go: ground truth (Marko's call): the painting and the tree mapping of line
  299/301 disagree
- Ground truth questionable: yes. On this line the paintings replace the whole line while the
  human tree mapping updates `None` -> `block_table` and keeps the `,` (invariant [10]).
