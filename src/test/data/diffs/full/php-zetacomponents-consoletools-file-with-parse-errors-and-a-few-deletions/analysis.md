# Accuracy analysis: php-zetacomponents-consoletools-file-with-parse-errors-and-a-few-deletions

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 3813 nodes, 3813 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`generateSynopsis()` drops the `$allowsArgs` variable: its initialisation `$allowsArgs = true;`
(line 279) is deleted and the argument is removed from the `createOptionSynopsis(...)` call. The
file has parse errors (as the fixture name says) and this method sits inside them.

## Mapping

- Before lines 279-280, 2 visible, one wrong twin. Both `$` tokens are bare leaves inside an
  `ERROR` node (their paths end in `... ERROR:1 > $:1` / `$:2`), so tree-sitter did not build
  `variable_name` nodes for `$allowsArgs` and `$synopsis` here. The ground truth deletes the `$` of
  the deleted line 279 and maps the `$` of `$synopsis` (line 280) to after line 279. omnidiff maps
  the line-279 `$` to after line 279's `$` (same row and column) and deletes line 280's, reason
  `APTED("large_flat_subtree")`. With the variables flattened into loose identical `$` tokens,
  only position separates them, and the tie went to the one that keeps its row. A normal parse
  would have kept each `$` inside its variable and the deletion would carry it. `parse-error`,
  with the tie-break itself in the matcher.

## Verdict

- Primary cause: `parse-error`
- Other causes: `matcher`
- Where a fix would go: tie-breaking among identical leaves in the APTED run of
  `solve_large_flat_subtrees` (keep a token with the text that follows it on its line); the
  ERROR region itself is the grammar's.
- Ground truth questionable: no
