# Accuracy analysis: vimscript-neovim-neovim-insert-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 183 nodes, 183 graded, N:M floor 0 visible |
| Painting, Minimal | 3 bytes wrong of 1926: renderer 3, matcher 0 |
| Painting, Full | 3 bytes wrong of 1926: renderer 3, matcher 0 |
| Ground-truth invariant violations | none |

## The change

neovim's asm ftplugin adds `#` as a comment leader. A history comment line is inserted (after line
8), and `,:#` is appended to the `setl comments=` value (line 14).

## Painting

The fixture has one painting (`Minimal`), so it is scored under both presets. The run is the same
with the human mapping.

- Update -> Insert, 3 bytes, after line 14 columns 37-40: `,:#`. The option value is one leaf, so
  omnidiff maps it Update, narrows the update to the appended bytes, and labels them Update. The
  painting labels them Insert. The extent agrees. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: narrow Update rendering in src/diff/text.rs: when one side of the narrowed
  span is empty, label it Insert/Delete.
- Ground truth questionable: no
