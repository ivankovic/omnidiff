# Accuracy analysis: vimscript-neovim-neovim-add-one-dict-item

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 44302 nodes, 44302 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 230084: renderer 2, matcher 0 |
| Painting, Full | 6 bytes wrong of 230084: renderer 6, matcher 0 |
| Ground-truth invariant violations | none |

## The change

One entry, `'concerto'`, is added to the long filetype dictionary literal in neovim's
test_filetype.vim (new after line 197). Each line of the dictionary starts with Vim's `\` line
continuation.

## Painting

Both runs are the same with the human mapping, so they are the renderer's.

- Minimal, 2 bytes, None -> Insert: the `\ ` at the start of the inserted line (after line 197,
  columns 4-6). The Minimal painting's Insert starts at the backslash. omnidiff paints the entry
  from its first token. In tree-sitter-vim 0.4.0 the continuation is the hidden extra
  `_line_continuation` (in `extras`, grammar.js), so no node covers those bytes. `renderer`.
- Full, 6 bytes: the same `\ ` plus the line's 4-space indentation, which the Full painting inserts
  separately. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs: when an inserted node starts a continuation line, extend
  its range back over the `\` extra; under Full, also over the indentation.
- Ground truth questionable: no
