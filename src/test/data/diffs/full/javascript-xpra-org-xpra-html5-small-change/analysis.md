# Accuracy analysis: javascript-xpra-org-xpra-html5-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 4 (4 visible) of 90862 nodes, 90862 graded, N:M floor 0 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

xpra-html5 `html5/js/Client.js` (a 90k-node file), lines 3380-3382:
`const pos = jQuery(win.div).position();` becomes `const pos = win.get_internal_geometry();`, and
the two uses `pos.left` / `pos.top` become `pos.x` / `pos.y`.

## Mapping

All 4 visible mismatches are on line 3380, reason `APTED("large_flat_subtree")` (APTED over the
largest flat container of the oversized enclosing subtree). The ground truth keeps `win.` and
treats the rest as replaced: before `win` (col 26) and the `.` of `win.div` (col 29) map Identical
to after `win` / `.` (cols 19, 22), the `.` of `.position` (col 34) is removed, and `div` and
`position` form a 2:1 group with `get_internal_geometry` of which exactly one pair is expected.
omnidiff instead pairs the `.` of `.position` with the after `.` (the outer `member_expression`'s
dot in both), deletes `win` and the inner `.`, and pairs neither `div` nor `position` with
`get_internal_geometry`. Pairing the outer dot alone would be defensible as an alternative reading
(the call `X.position()` with `X` and the name replaced), but omnidiff then does not rename the
property either, so its mapping keeps one `.` and nothing it belongs to; the ground truth's reading
keeps `win.` together. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: `APTED("large_flat_subtree")` on the method (why its edit script keeps
  the outer `.` rather than `win.` was not traced further)
- Ground truth questionable: no
