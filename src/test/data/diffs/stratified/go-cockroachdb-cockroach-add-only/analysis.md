# Accuracy analysis: go-cockroachdb-cockroach-add-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 875 nodes, 875 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 4104: renderer 0, matcher 0 |
| Painting, Full | 296 bytes wrong of 4104: renderer 296, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Adds a package variable `initialRetryBackoffOverride` (after lines 20-25) and wraps the existing
`if useFastRetry { ... }` (before line 28) as the `else if` branch of a new
`if initialRetryBackoffOverride > 0 { ... }` (after lines 34-36).

## Painting

Minimal matches exactly. In Full, all 296 bytes are present with the human mapping too, so they
are the renderer's.

- `Move -> None`, before lines 28-34 and after lines 36-42, 296 bytes. These are the whole
  `if useFastRetry { opts = retry.Options{...} }` statement on both sides. In the tree it moved
  from the function body into the `alternative` of the new if, and its first row now starts at
  column 8 (`} else if`) instead of column 1. Omnidiff's Full rendering paints the moved subtree
  grey Move. The Full painting leaves it unpainted, because a reader sees the same block in the
  same order, only wrapped by an inserted condition. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Full move painting in `src/diff/text.rs` (`identical_or_move`). A
  subtree that moves only because it was wrapped in an inserted parent, in the same order, should
  stay unpainted.
- Ground truth questionable: no
