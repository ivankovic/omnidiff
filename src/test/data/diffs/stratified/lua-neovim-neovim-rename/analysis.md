# Accuracy analysis: lua-neovim-neovim-rename

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 526 nodes, 526 graded, N:M floor 0 visible |
| Painting, Minimal | 9 bytes wrong of 1369: renderer 9, matcher 0 |
| Painting, Full | 0 bytes wrong of 1369: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In `runtime/ftplugin/markdown.lua`, three keymap option tables rename the key `buffer = 0` to
`buf = 0` (lines 5, 9 and 12).

## Painting

- **Update -> Delete, before lines 5, 9 and 12, 3 x 3 bytes (`fer`), Minimal, `renderer`.** The
  Minimal painting paints only the removed `fer` of `buffer`, as a Delete. omnidiff maps
  `buffer` -> `buf` as an update, as do the ground truth and the Full painting (Match `buffer` ->
  `buf`). In Minimal it narrows the update to the removed suffix but keeps the label Update. The
  runs are the same with the human mapping. When an update only removes characters, the Minimal
  painting calls the removed characters a Delete.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Minimal narrowing of an Update in src/diff/text.rs: label the remainder
  Delete when the narrowed update is a pure deletion
- Ground truth questionable: no
