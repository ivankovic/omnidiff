# Accuracy analysis: css-madmaxms-theme-obsidian-2-add-gnome-44-and-a-few-changes

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 125 (75 visible) of 70234 nodes, 70234 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

A GNOME Shell theme update. In three rules (before lines 4708-4717) the commit adds a second
selector,
`.quick-menu .quick-toggle-arrow`. It adds a `box-shadow` declaration to three rules (after lines
4752,
4770, 4774). It appends a 132-line "GNOME 44" block at the end of the file (after lines 5047-5178).

## Mapping

- Shape twin in the appended block, 68 visible and 44 invisible. Before lines 4712-4714
  (`.quick-menu-toggle .quick-toggle-arrow:ltr { border-radius: 0 5px 5px 0; }`) and 4715-4717 (the
  `:rtl` twin) get a second selector in the after file (lines 4713-4719). The ground truth keeps
  them in place. omnidiff maps the whole `rule_set` (reason `StructurallyIdenticalSubtrees`) and all
  its leaves (reason `StructurallyIdenticalAncestor`) to the new rules at after 5108-5110
  (`.quick-menu-toggle .quick-toggle:rtl{ border-radius:0 5px 5px 0; }`) and 5102-5104 (`...:ltr{
  border-radius:5px 0 0 5px; }`). These have the same one-selector shape and the same border-radius
  values, but a different class name (`quick-toggle` vs `quick-toggle-arrow`) and the opposite
  direction (`ltr` and `rtl` cross over). The in-place rules lost their shape when the selector list
  grew, so the shape pass found the far twins first. 56 of the mismatches are the after-side nodes
  of lines 5102-5110, which the ground truth inserts. `matcher`.
- Container deleted around an inserted declaration, 7 visible and 6 invisible. In the rules at
  before 4747, 4762 and 4767 a `box-shadow` declaration is inserted. omnidiff keeps the declarations
  mapped, but deletes the `rule_set`, `block`, `{` and `}` (op Delete, reason
  `APTED("fast_fallback")`), and also deletes the `/* coloraccent */` comment on line 4768. The
  ground truth maps all of these Identical / MatchButNotIdentical. The comment has a second
  identical after-side twin on the inserted line 4774, which may be why it was left unpaired (not
  verified). `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the shape pairing behind `StructurallyIdenticalSubtrees` (prefer an in-place
  candidate that shares its selector prefix over a shape twin 400 lines away), and the phase 6
  fallback (`resolve_residual_forest_via_myers_lcs`, src/diff/apted/common/residual.rs) for
  unmatched containers whose children are all matched.
- Ground truth questionable: no
