# Accuracy analysis: css-wordpress-wordpress-remove-one-rule

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 382 nodes, 382 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 1071: renderer 0, matcher 0 |
| Painting, Full | 120 bytes wrong of 1071: renderer 120, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A minified one-line stylesheet loses one rule:
`.wp-block-tag-cloud.alignfull{padding-left:1em;padding-right:1em}`.
Nothing else changes.

## Painting

The fixture has one painting (`Only one solution`), so it is scored under both presets. The run is
the same with the human mapping.

- Full, 120 bytes (60 per side), Move -> None: `.wp-block-tag-cloud
  a{display:inline-block;margin-right:5px}`, the rule right after the deleted one (before column
  180, after column 115), is painted Move. It only slid left on the same row. Later rules slid too,
  but are not painted. `identical_or_move` (src/diff/text.rs) exempts a single-row node slid by an
  edit beside it when `node_untouched_on_its_row` finds it in the rows' common prefix or suffix. The
  deleted rule and its successor both start with `.wp-block-tag-cloud`, so the common prefix runs 19
  characters into the successor (to column 134). The suffix is then capped at `row.len() - prefix`,
  which no longer reaches the successor's start on either side. The same overlap causes the 42 Full
  bytes in `css-playwright-add-class-selector`. Minimal reads the rows differently
  (`read_rows_carrying_several_edits`) and has no disagreement. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `node_untouched_on_its_row` in src/diff/text.rs: compute the suffix without
  letting an overlapping prefix cap it, or test against the edit's actual span.
- Ground truth questionable: no
