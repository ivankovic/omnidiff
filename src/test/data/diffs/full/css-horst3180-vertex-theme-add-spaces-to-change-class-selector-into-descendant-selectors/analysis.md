# Accuracy analysis: css-horst3180-vertex-theme-add-spaces-to-change-class-selector-into-descendant-selectors

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 16 (8 visible) of 205146 nodes, 205146 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In a 5,129-line GTK theme stylesheet, the commit inserts a space in eight selectors on lines 5066,
5075 and 5079 (`UnityDecoration.top` becomes `UnityDecoration .top`), and splits line 5079 in two.
As `description.md` says, this whitespace change alters the AST: a `class_selector` with a tag
becomes a `descendant_selector` holding the tag and a tag-less `class_selector`.

## Mapping

- 8 visible (`.`) and 8 invisible (`class_selector`) mismatches, lines 5066, 5075 and 5079 of
  before, one pair per selector. The ground truth maps each before `class_selector`
  (`UnityDecoration.top`, MatchButNotIdentical) to the inner after `class_selector` (`.top`), maps
  its `.` Identical, and inserts the new `descendant_selector`. omnidiff deletes the before
  `class_selector` and its `.` (op Delete, reason `APTED("fast_fallback")`). The `tag_name` and
  `class_name` leaves agree with the ground truth; they are not in the mismatch list. So of each
  selector, only the old `.` shows as deleted. `matcher`.
- Why the phase 6 fallback did not pair the `.` leaves is undetermined. I did not run the engine.
  The fallback works on maximal unmatched subtrees. Here every unmatched container keeps a matched
  child, so only the bare `.` leaves and the containers stay in the residual. The ground truth's
  pairing does not preserve ancestry (the before `tag_name` is inside `class_selector`, the after
  one is not), so a pure tree edit distance would not produce it either.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: phase 6 residual resolution, `resolve_residual_forest_via_myers_lcs` in
  src/diff/apted/common/residual.rs, or an earlier pass that recognises a container whose first
  child moved out into a new wrapper.
- Ground truth questionable: no
