# Accuracy analysis: c-pixaranimationstudios-opensubdiv-change-license-comment

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 6583 nodes, 6583 graded, N:M floor 0 visible |
| Painting | no painting |
| Ground-truth invariant violations | none |

## The change

OpenSubdiv `regression/common/far_utils.h`: the 19-line modified-Apache license header (lines
4-22, one `//` comment per line) is replaced by two lines, `//   Licensed under the terms set forth
in the LICENSE.txt file available at` and `//   https://opensubdiv.org/license.`.
`description.md`: "Requires a N:M match for perfect solution". The graded mapping is 1:1, though
(N:M floor 0).

## Mapping

- Lines 4 and 16 before: the ground truth updates the first license line (`// Licensed under the
  Apache License, ...`) into the new first line (`// Licensed under the terms ...`), and the URL
  line `//       http://www.apache.org/licenses/LICENSE-2.0` (line 16) into the new URL line
  (after line 5). The other 17 old lines are deleted. omnidiff deletes all 19 and inserts both new
  lines (op Delete, reason `APTED("fast_fallback")`, the terminal
  `resolve_residual_forest_via_myers_lcs` in `src/diff/apted/common/residual.rs`). The 19 vs 2
  comments form an unequal gap of single-leaf entries of one kind. The wrap/reparent filter keeps
  only non-trivial entries, so nothing is left, and the gap goes to
  `resolve_unequal_segment_via_kind_only_anchors`. A kind-only anchor needs a minimum size and a
  unique hash, which comment leaves of one kind do not have. The two similarity alignments found no
  pair either (not traced further), so the gap was replaced atomically. Both pairings in the
  ground truth are reasonable (shared prefix `Licensed under the`; URL for URL). `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the unequal-gap branch of `resolve_residual_forest_via_myers_lcs`
  (`src/diff/apted/common/residual.rs`), which has no similarity anchor for comment leaves of one
  kind, or an earlier comment-block pass
- Ground truth questionable: no
