# Accuracy analysis: css-playwright-add-class-selector

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 805 nodes, 805 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 3974: renderer 0, matcher 0 |
| Painting, Full | 43 bytes wrong of 3974: renderer 43, matcher 0 |
| Ground-truth invariant violations | none |

## The change

One line of a CSS file: `.test-file-test:hover {` becomes `.test-file-test-selected,
.test-file-test:hover {` (line 25). A new selector is prepended to the selector list.

## Painting

All runs are the same with the human mapping, so they are the renderer's.

- Move -> None, Full, 42 bytes (21 per side): `.test-file-test:hover` on line 25 is painted Move on
  both sides. It is unchanged, but moved from column 0 to column 26. The Full painting leaves it
  unpainted. `identical_or_move` in src/diff/text.rs exempts a single-row node that slid sideways
  only when `node_untouched_on_its_row` finds it inside the rows' common prefix or suffix. Here the
  two rows share the prefix `.test-file-test` (15 characters). The suffix length is capped at
  `source_row.len() - prefix` = 23 - 15 = 8 (`:hover {`), which does not reach column 0. The node is
  in fact the whole common tail of the row. `renderer`.
- None -> Insert, Full, 1 byte: the space after the inserted `.test-file-test-selected,` (column
  25). The Full painting has a separate one-byte Insert for it; omnidiff leaves it unpainted.
  `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `node_untouched_on_its_row` in src/diff/text.rs (the suffix cap when prefix
  and suffix overlap), plus the separator space next to an inserted node in Full.
- Ground truth questionable: no
