# Accuracy analysis: vimscript-neovim-neovim-date-update-plus-bugfix

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 202 nodes, 202 graded, N:M floor 0 visible |
| Painting, Minimal | 1 bytes wrong of 1280: renderer 1, matcher 0 |
| Painting, Full | 1 bytes wrong of 1280: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Two lines of neovim's pyright compiler plugin. The `Last Change` date changes (line 4), and in the
errorformat continuation on line 21 a backslash is inserted: `\%C[ \t]\ %.%#,` becomes
`\%C[\ \t]\ %.%#,`.

## Painting

The fixture has one painting (`Only one solution`), so it is scored under both presets. The run is
the same with the human mapping.

- Update -> Insert, 1 byte, after line 21 column 10: the inserted `\`. omnidiff narrows the leaf's
  update to exactly that byte, and labels it Update. The painting labels it Insert. The date change
  on line 4 agrees. This is the same label rule as `css-shadcn-ui-ui-add-two-bytes` and
  `css-wordpress-wordpress-remove-webkit-prefix`. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: narrow Update rendering in src/diff/text.rs: when one side of the narrowed
  span is empty, label it Insert/Delete.
- Ground truth questionable: no
