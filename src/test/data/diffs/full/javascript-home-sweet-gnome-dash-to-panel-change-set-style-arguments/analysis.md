# Accuracy analysis: javascript-home-sweet-gnome-dash-to-panel-change-set-style-arguments

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 51 (33 visible) of 26137 nodes, 26137 graded, N:M floor 0 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

dash-to-panel `src/panel.js`, method `_setPanelBoxStyle` (lines 1110-1121): instead of building a
`style` string and calling `this.panelBox.set_style(style)` at the end, it calls
`this.panelBox.set_style('')` up front (line 1111) and, inside the `if (!disable)` block, calls
`this.panelBox.set_style(...)` with a multi-line template string that adds theme padding (after
lines 1120-1126). Two new `let`s are added (after lines 1116-1117). The rest of the 26k-node file
is unchanged.

## Mapping

All 51 mismatches are inside `_setPanelBoxStyle`, reason `APTED("qualified_name")` throughout.
The ground truth reads the final `this.panelBox.set_style(style)` as moved into the `if` block
(and its argument replaced), and the old template string as moved from `style = ...` into that
call's arguments. Both moves cross a nesting level and both subtrees changed, so nothing pairs them
before the method-level APTED, which cannot express a move:

- The trailing `set_style` call, before line 1120 (12 visible, 8 invisible). omnidiff deletes
  `this` and `(`, and pairs `.`, `panelBox`, `.`, `set_style`, `)` with `.`, `Side`, `.`,
  `LEFT`, `)` of `getPadding(St.Side.LEFT)` on after line 1125 (Updates for the names), and the
  `call_expression`/`member_expression` containers likewise. The ground truth maps them to the
  new call on after line 1120. `matcher`.
- The template string, before line 1117 (13 visible, 4 invisible). The ground truth maps the
  `` ` ``, `${`, `}`, `px `, `px;`, `sideMargins`, `topBottomMargins` tokens and the
  `template_string` / `template_substitution` containers to the new template on after lines
  1121-1125. omnidiff deletes them all (and inserts the after ones). `matcher`.
- The `if` block's braces, before lines 1113 and 1118 (2 visible, 2 invisible): omnidiff deletes
  the `if_statement`, its `statement_block`, `{` and `}`, although the lines inside are matched.
  Consequence of the same APTED alignment. `matcher`.
- Line 1111, `let style = ''` vs `this.panelBox.set_style('')` (6 visible, 2 invisible). The ground
  truth deletes the declaration and inserts the call, `''` included. omnidiff pairs the `''` strings
  and their quotes (4 visible, 2 invisible) and `style` with `panelBox` (2 visible; printed op
  `MatchButNotIdentical`, an `identifier` paired with a `property_identifier`). Pairing the
  byte-identical `''` is defensible (`ambiguous`); renaming the local `style` into the property
  `panelBox` is not (`matcher`).
- The first substitution's operand, before line 1117 col 29 / after line 1122 col 17 (2
  invisible): the ground truth pairs `this.geom.topOffset + topBottomMargins` with the whole new
  `... + getPadding(St.Side.TOP)` (MatchButNotIdentical); omnidiff pairs it Identical with the
  byte-identical inner left operand. Both are defensible. `ambiguous`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Verdict

- Primary cause: `matcher`
- Other causes: `ambiguous`
- Where a fix would go: move detection for changed subtrees moved across one nesting level
  (`MovedSubtree` or a pre-APTED pass), before `APTED("qualified_name")` runs on the method
- Ground truth questionable: no
