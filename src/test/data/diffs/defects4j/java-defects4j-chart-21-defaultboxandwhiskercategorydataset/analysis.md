# Accuracy analysis: java-defects4j-chart-21-defaultboxandwhiskercategorydataset

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (0 visible) of 5894 nodes, 5894 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 49571: renderer 0, matcher 0 |
| Painting, Full | 0 bytes wrong of 49571: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Chart-21 in `DefaultBoxAndWhiskerCategoryDataset`: in `add(...)` the min/max outlier
bookkeeping after the `if (... ) { updateBounds(); }` at line 153 is moved into a new `else`
branch (after lines 157 and 189), and `updateBounds()` gains a full rescan of all items.

## Mapping

- **The `if` statement at before line 153 (1 invisible).** The ground truth maps it to the after
  `if` statement, which gained the `else` clause. omnidiff leaves it unmapped (op Delete, reason
  `APTED("large_flat_subtree")`), while every child of the `if` is mapped correctly. An
  `if_statement` has no bytes of its own outside its children, so no reader sees the
  difference, and both paintings agree byte for byte. The tree mapping is still wrong for that
  node: the fallback over the method's large flat body did not pair the two containers.
  `grading-artifact` (visible effect), `matcher` (tree).

## Verdict

- Primary cause: `grading-artifact`
- Other causes: `matcher`
- Where a fix would go: `APTED("large_flat_subtree")` (`solve_large_flat_subtrees`); a later
  container pass such as `solve_mutual_ancestors` could pair an `if` whose children all map into
  one after `if`
- Ground truth questionable: no
