# Accuracy analysis: lua-luals-lua-language-server-add-gsub-function-call

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (0 visible) of 11996 nodes, 11996 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `script/library.lua`, line 152 changes from
`compileBuf[#compileBuf+1] = convertLink(uri, des)` to
`compileBuf[#compileBuf+1] = convertLink(uri, des):gsub('[\r\n]', '...')`. The existing call becomes
the receiver of a new `:gsub` method call.

## Mapping

- **Wrapped call paired with the wrapper (0 visible, 2 invisible), `matcher`.** The ground truth
  maps the before `function_call` `convertLink(uri, des)` to the identical inner call in the after
  line, and marks the outer `...:gsub(...)` call as new. omnidiff maps the before call to the outer
  call (op MatchButNotIdentical, `APTED("qualified_name")`) and leaves the inner call node
  unmatched. APTED keeps the node at the same depth instead of following the subtree one level
  down. Every leaf maps as in the ground truth (`convertLink`, the arguments), so a reader sees the
  same diff. Only the two inner `function_call` nodes differ.

## Verdict

- Primary cause: `matcher` (invisible only; no reader-visible effect)
- Other causes: none
- Where a fix would go: wrap detection after `APTED("qualified_name")`: when a matched node's whole
  before subtree is identical to a child of its after partner, re-pair it with the child
- Ground truth questionable: no
