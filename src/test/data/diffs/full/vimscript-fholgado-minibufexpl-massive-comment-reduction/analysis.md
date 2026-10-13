# Accuracy analysis: vimscript-fholgado-minibufexpl-massive-comment-reduction

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 6 (6 visible) of 19454 nodes, 19454 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

The commit cuts the documentation header of minibufexpl.vim from about 300 comment lines to about
20 (before lines 7-302 become after lines 7-22). It re-indents the remaining header lines by one
space, and removes a block at the end of the file (before lines 2081-2337).

## Mapping

All 6 mismatches are visible comment leaves. In Vim script each comment line is one leaf, so a
re-indented line has a different hash.

- Far anchor, 4 visible. The ground truth maps before lines 19-21 (`"`, `" Name Of File: ...`, `"
  Description: ...`) to after lines 19-21 as Updates: the author added one space after each `"`. It
  deletes before line 57 (`" `). omnidiff maps before 57 `" ` Identical to after 19 `" `, which is
  the same text 38 lines down, and deletes before 19-21 (reason `APTED("fast_fallback")`).
  Line-based `diff` makes the same anchor choice (`58,302c20,22`). Once that lone leaf is anchored,
  the lines before it cannot pair with after lines 20-21, which come after the anchor. `matcher`.
- Twin `"`, 2 visible. After line 23 is a lone `"`. The ground truth maps it from before line 303,
  the `"` just above the kept `"=====` rule, and deletes before line 59. omnidiff maps it from
  before line 59 (op Identical) and deletes 303. Both are identical one-character comments; the
  ground truth's choice follows the neighbouring line, omnidiff's follows sequence order.
  `ambiguous`.

## Verdict

- Primary cause: `matcher`
- Other causes: `ambiguous`
- Where a fix would go: phase 6 residual Myers LCS (`resolve_residual_forest_via_myers_lcs`,
  src/diff/apted/common/residual.rs): a trivial one-token comment should not anchor the alignment
  far from its context, or the near-identical re-indented leaves should be paired first.
- Ground truth questionable: no
