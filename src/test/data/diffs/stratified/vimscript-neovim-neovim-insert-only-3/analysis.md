# Accuracy analysis: vimscript-neovim-neovim-insert-only-3

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 151 nodes, 151 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 1414: renderer 0, matcher 0 |
| Painting, Full | 2 bytes wrong of 1414: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

neovim's sieve ftplugin wraps `setlocal fileformat=dos` (line 19) in a new
`if expand('%:p') !=# '' && !filereadable(expand('%:p'))` block with a comment above it, and adds a
history comment (after line 6).

## Painting

The run is the same with the human mapping.

- Full, 2 bytes, None -> Insert: the two spaces in front of `setlocal fileformat=dos` on after line
  22. The Full painting matches the statement (before line 19 -> after line 22 from column 2) and
  inserts the new indentation separately. omnidiff paints the statement the same way, but leaves the
  added indentation unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full rendering in src/diff/text.rs: indentation that exists only on the
  after side in front of a moved line should be painted Insert.
- Ground truth questionable: no
