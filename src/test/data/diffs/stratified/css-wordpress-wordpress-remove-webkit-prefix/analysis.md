# Accuracy analysis: css-wordpress-wordpress-remove-webkit-prefix

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1480 nodes, 1480 graded, N:M floor 0 visible |
| Painting, Minimal | 8 bytes wrong of 4138: renderer 8, matcher 0 |
| Painting, Full | 8 bytes wrong of 4138: renderer 8, matcher 0 |
| Ground-truth invariant violations | none |

## The change

One property in a minified one-line stylesheet: `-webkit-appearance:revert` becomes
`appearance:revert` in the `select` rule.

## Painting

The fixture has one painting (`Only one solution`), so it is scored under both presets. The run is
the same with the human mapping.

- Update -> Delete, 8 bytes, before line 1 at byte 1495: `-webkit-`. omnidiff maps the property name
  Update, narrows the update to the removed prefix, and paints it Update. The painting marks the
  same 8 bytes Delete. The extent agrees and only the label differs. A narrowed update whose other
  side is empty is a pure deletion (the mirror case of `css-shadcn-ui-ui-add-two-bytes`).
  `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: narrow Update rendering in src/diff/text.rs: when one side of the narrowed
  span is empty, label it Delete/Insert.
- Ground truth questionable: no
