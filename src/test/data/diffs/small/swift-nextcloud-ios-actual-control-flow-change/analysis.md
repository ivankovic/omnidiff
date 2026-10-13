# Accuracy analysis: swift-nextcloud-ios-actual-control-flow-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (0 visible) of 7401 nodes, 7401 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `iOSClient/Networking/NCNetworking+Download.swift`, a `#if ... return #endif` early exit
(before lines 222-224) becomes `#if ... #else`, and the `#endif` moves to after line 272. The rest
of the closure becomes the `#else` branch. A `// MARK: -` comment gets a title, and one comment is
added.

## Mapping

- **`#endif` renamed into `#else` (0 visible, 2 invisible), `matcher`.** The ground truth deletes
  the old `#endif` directive (before line 224) and inserts both the new `#else` (after line 223)
  and the new `#endif` (after line 272). omnidiff pairs the old `#endif` with the new `#else` at
  the same position (op MatchButNotIdentical, `APTED("qualified_name")`): APTED renames one
  directive into a different one instead of deleting it. Only the `directive` nodes are counted, and
  they are not graded as visible, so the effect on the counts is invisible. A reader would still
  see `#endif` -> `#else` as an edit rather than a removal plus an insertion.

## Verdict

- Primary cause: `matcher` (invisible only)
- Other causes: none
- Where a fix would go: `APTED("qualified_name")` update cost: directives with different keywords
  (`#endif` vs `#else`) should not be renamed into each other
- Ground truth questionable: no
