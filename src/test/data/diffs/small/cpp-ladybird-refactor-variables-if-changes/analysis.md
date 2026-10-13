# Accuracy analysis: cpp-ladybird-refactor-variables-if-changes

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 141 (87 visible) of 12658 nodes, 12765 graded, N:M floor 65 visible |
| Painting, Minimal | 1034 bytes wrong of 57923: renderer 536, matcher 942 |
| Painting, Full | 1078 bytes wrong of 57923: renderer 300, matcher 890 |
| Ground-truth invariant violations | none |

## The change

`SVGFormattingContext.cpp` gains `SVGPatternBox` support. At lines 206-207,
`auto& svg_graphics_element = as<SVG::SVGGraphicsElement>(*dom_node);` and
`auto active_view_box = ...` become `Optional<SVG::ViewBox> active_view_box;` and an
`if (auto* ... = as_if<...>) ... else if (auto* ... = as_if<SVGFitToViewBox>) ...` chain (after
lines 207-211). In the mask/clip layout, the old `if (content_units == ObjectBoundingBox) {...}`
becomes the `else if` of a new `if (pattern_box && ...)` with a nested `if`. The two
`set_content_width`/`set_content_height` lines and `parent_node_state` are copied into the new
branches. `|| is<SVGPatternBox>(...)` is appended to two conditions.

## Mapping

- **N:M copies (45 visible, 29 invisible, `nm-floor`).** The ground truth uses all-to-all groups.
  One before statement is copied to two or three after sites: the `active_view_box` declaration
  (line 207) to the after declaration at line 207 and the assignments at lines 209 and 211;
  `auto& parent_node_state = ...` (line 527) to lines 544 and 550; the
  `set_content_width`/`set_content_height` calls (lines 528-529) to lines 541-542, 545-546 and
  551-552. A one-to-one mapping takes at most one copy, so the rest are Inserts. The fixture's N:M
  floor is 65 visible.
- **Wrong if (22 visible, 11 invisible, `matcher`, `APTED("qualified_name")`).** The ground truth
  maps the before `if (content_units == SVG::SVGUnits::ObjectBoundingBox) {` (line 525) to the
  identical `} else if (content_units == SVG::SVGUnits::ObjectBoundingBox) {` at line 549. Omnidiff
  maps the `if` keyword to the new outer `if (pattern_box && ...)` (line 538). It maps the condition
  to the nested `if (pattern.pattern_units() == SVG::SVGUnits::UserSpaceOnUse)` (line 540):
  `content_units` -> `pattern` and `ObjectBoundingBox` -> `UserSpaceOnUse` become Updates. It
  deletes the block's `{` and `}`. The after `else if` condition at line 549 is then new.
- **`parent_node` declaration (5 visible, 3 invisible, `matcher`).** The ground truth removes
  `auto* parent_node = mask_or_clip.parent();` (line 526). Omnidiff maps it onto
  `auto& parent_node_state = m_state.get(*mask_or_clip.parent());` (line 544), which the ground
  truth pairs with line 527. The after line merges both before lines. The ground truth's pairing
  follows the declared name.
- **Declaration folded into the if (13 visible, 9 invisible, `matcher`).** The ground truth maps
  `auto& svg_graphics_element = as<...>(*dom_node)` (line 206) into the condition
  `auto* svg_graphics_element = as_if<...>(*dom_node)` (line 208). Here `as` -> `as_if` is an Update
  and `&` -> `*` a match. It maps `svg_graphics_element.active_view_box()` to line 209. Omnidiff
  deletes line 206 and most of line 207. It sends line 207's `auto` and `active_view_box` to the
  `else if` branch (line 210 `auto`, line 211 `view_box`).
- **Final else (2 visible, 2 invisible, `matcher`).** Before line 520's `else VERIFY_NOT_REACHED();`
  is mapped to the new `else if (is<SVGPatternBox>(...))` (line 529). It should map to the final
  `else` (line 531).

## Painting

- **Matcher (Minimal 942, Full 890 bytes).** These come from the groups above. The after
  `else if (content_units == ...) {` is painted Insert where the painting has Move. The before `if`
  body lines 525-529 are left unpainted where the painting has Move. The rewrite of lines 206-211 is
  painted Move/Insert/Delete against the painting's other choice. Part of these bytes are the N:M
  copies of `set_content_width(`/`set_content_height(` (after lines 541-546), painted Insert where
  the painting has Move. A one-to-one mapping cannot avoid those. With the human mapping they render
  as Move.
- **Renderer (Minimal 536, Full 300 bytes).** These runs stay when the human mapping is rendered.
  Omnidiff paints the unchanged text of the folded declaration (`auto`, `svg_graphics_element =`,
  `<SVG::SVGGraphicsElement>(*dom_node)`) Move, where the painting leaves it unpainted. Omnidiff's
  Minimal also paints leading indentation Move (before line 529, after lines 542, 546, 552). The
  painting paints `);` and `;` and the unchanged tails of the N:M copies
  (`parent_node_state.content_width()`, after line 547's `parent_viewbox_transform = ...`) Move,
  where omnidiff leaves them unpainted.

## Verdict

- Primary cause: `matcher`
- Other causes: `nm-floor`, `renderer`
- Where a fix would go: APTED in the `qualified_name` call
  (`src/diff/solve_syntax_aware_matching.rs`). It should prefer the identical `else if` over a new
  wrapping `if`, and send a declaration folded into an `if` condition to the branch with the same
  name.
- Ground truth questionable: no
