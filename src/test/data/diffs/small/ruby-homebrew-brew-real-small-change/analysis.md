# Accuracy analysis: ruby-homebrew-brew-real-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 3131 nodes, 3131 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | 1 |

## The change

A GitHub-releases livecheck spec: one release fixture entry is removed, and the block body
`match = release["tag_name"]&.match(regex)` / `next if match.blank?` / `match[1]` (before lines
152-155) becomes the single expression `release["tag_name"]&.[](regex, 1)` (after line 146).

## Mapping

- Before line 155, the `1` in `match[1]`: the ground truth maps it to the `1` argument of
  `&.[](regex, 1)` on after line 146. Omnidiff deletes it (reason `APTED("fast_fallback")`).
- Why: the ground truth also maps the before call `release["tag_name"]&.match(regex)` (line 152)
  to the after call. The after `1` is inside that call, but the before `1` is in a different
  statement (`match[1]`, an `element_reference`). An ordered tree mapping that keeps the call pair
  cannot also map the `1` (ancestor rule), so APTED drops the leaf. Both readings are defensible:
  the index `1` has the same role (capture group 1), but it moved from a separate statement into a
  new argument list, and delete plus insert of a one-character literal is a fair rendering.
  `ambiguous`.

## Ground truth

`invariants.txt` reports one violation: the ground truth deletes the method name `match` (before
line 152, columns 39-44) and inserts the operator `[]` (after line 146, columns 31-33). The parent
calls are matched and nothing else occupies that child position, so by the invariant the two are
the same element (an update). This does not cause the mapping mismatch above.

## Verdict

- Primary cause: `ambiguous`
- Other causes: `ground-truth`
- Where a fix would go: none: ambiguous. (A cross-ancestor leaf move would need a pass after
  APTED.)
- Ground truth questionable: yes: `match` -> `[]` in a matched call is delete plus insert where
  the invariant expects an update (invariants.txt, rule 18).
