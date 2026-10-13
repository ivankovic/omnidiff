# Accuracy analysis: tsx-mui-material-ui-move-colour-to-a-new-attribute

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 16 (12 visible) of 2221 nodes, 2221 graded, N:M floor 0 visible |
| Painting | no painting |
| Ground-truth invariant violations | none |

## The change

On a `<Typography>` element the attribute `color="inherit"` (before line 146) is removed and the
colour moves into a new `sx={{ color: 'inherit' }}` attribute placed after
`className="imageTitle"` (after line 147); `className` itself is unchanged and shifts up a line.

## Mapping

- Positional pairing of the attributes (12 visible, 4 invisible; `matcher`, reason
  `APTED("qualified_name")`). The ground truth keeps `className="imageTitle"` identical (before
  line 147 -> after line 146) and maps `color="inherit"` into the new pair `color: 'inherit'`
  (`color` -> `color`, `=` -> `:`, `"` -> `'`, `inherit` -> `inherit`), inserting `sx` and `=`.
  omnidiff pairs the attributes by position: `color="inherit"` -> `className="imageTitle"`
  (`color` and `inherit` updated to `className` and `imageTitle`) and `className="imageTitle"`
  -> `sx={{ color: 'inherit' }}` (`className` -> `sx`, `imageTitle` -> `inherit`, its quotes
  deleted). The ground-truth pairing crosses (`color` precedes `className` before, its target
  follows it after), which the ordered APTED call over the `ProductCategories` qualified-name
  block cannot express; it settled for relabelling. The byte-identical `className="imageTitle"`
  attribute is not pre-matched before APTED runs (the pre-match there,
  `prematch_identical_statement_siblings`, covers statement siblings, not JSX attributes).

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: identical-sibling pre-match before `APTED("qualified_name")` in
  src/diff/solve_syntax_aware_matching.rs, extended to JSX attributes (or a move pass over
  reordered identical attributes)
- Ground truth questionable: no
