# Accuracy analysis: go-gohugoio-hugo-for-loop-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 3465 nodes, 3465 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 11166: renderer 4, matcher 0 |
| Painting, Full | 6 bytes wrong of 11166: renderer 6, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Three benchmark loops move to Go 1.24's `b.Loop()`. Line 207 `for i := 0; i < b.N; i++ {` becomes
`for b.Loop() {`. Lines 212 and 221 keep the counter: `for i := 0; b.Loop(); i++ {`.

## Painting

All runs are the same with the human mapping, so they are the renderer's.

- Minimal, `None -> Insert`, after lines 212 and 221, 2 bytes each: the `()` after `b.Loop`. In
  the tree it is a new `argument_list`, and its range holds only brackets, so Minimal's
  structural-punctuation filter drops it. The pair is dropped together, so
  `restore_paired_brackets` does not restore it. The painting inserts `()`, and it is the only
  visible sign that a field read became a call. (On line 207 the `()` is part of a larger
  inserted range, and both agree.) `renderer`.
- Full, `None -> Delete` / `None -> Move`, 6 bytes: single spaces on lines 212 and 221. One is
  the space before the deleted `i <` (before side). The others are the leading space of the moved
  ` b.` (both sides). The Full painting includes them in its Delete and Move ranges. Omnidiff's
  Full ranges stop at the token. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Minimal structural-punctuation filter in
  `src/diff/text/render_options.rs`, for an inserted empty argument list that creates a call. For
  Full: inter-token whitespace next to deleted and moved ranges.
- Ground truth questionable: no
