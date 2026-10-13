# Accuracy analysis: html-hugo-tag-to-selfclosing-tag

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (0 visible) of 184 nodes, 184 graded, N:M floor 0 visible |
| Painting | no painting |
| Ground-truth invariant violations | none |

## The change

Line 6 `<meta charset="utf-8" />` loses its self-closing slash: `<meta charset="utf-8">`. In the
tree a `self_closing_tag` becomes a `start_tag`.

## Mapping

- One invisible container (`matcher`, `APTED("fast_fallback")`). The ground truth maps the before
  `self_closing_tag` (line 6 col 5) to the after `start_tag` as MatchButNotIdentical. omnidiff
  deletes it: the two nodes have different kinds, and no pass pairs them, so the final fallback
  leaves it unmatched. Its children (`<`, `meta`, the `charset` attribute) are not in the
  mismatch list, so they are mapped as the ground truth expects; only `/>` vs `>` differs, which
  is Delete/Insert under either mapping. A reader sees no difference.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: pairing a `self_closing_tag` with a `start_tag` whose children all map
  (bottom-up propagation across this kind change); no visible effect today
- Ground truth questionable: no
