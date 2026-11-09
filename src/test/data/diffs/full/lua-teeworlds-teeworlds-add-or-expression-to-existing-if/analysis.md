# Accuracy analysis: lua-teeworlds-teeworlds-add-or-expression-to-existing-if

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (0 visible) of 11411 nodes, 11411 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `bam.lua`, line 507 adds a third alternative to an `elseif` condition: `arch == "ia64" or arch ==
"amd64"` becomes `arch == "ia64" or arch == "amd64" or arch == "arm64"`.

## Mapping

- **Old condition paired with the new outer `or` (0 visible, 2 invisible), `matcher`.** `or` is
  left-associative, so the old `binary_expression` `arch == "ia64" or arch == "amd64"` is unchanged
  and becomes the left operand of a new outer `... or arch == "arm64"`. The ground truth maps the
  old expression to that identical inner node and marks the outer one new. omnidiff maps it to the
  outer node (op MatchButNotIdentical, `APTED("fast_fallback")`) and leaves the inner node
  unmatched. APTED keeps the node at the same depth instead of following the identical subtree one
  level down. All leaves agree with the ground truth, so the reader sees the same diff.

## Verdict

- Primary cause: `matcher` (invisible only)
- Other causes: none
- Where a fix would go: wrap detection after APTED (`APTED("fast_fallback")`): re-pair a matched
  node with its partner's child when the before subtree is identical to that child
- Ground truth questionable: no
