# Accuracy analysis: cpp-godot-small-bugfix

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 13 (7 visible) of 11658 nodes, 11658 graded, N:M floor 0 visible |
| Painting, Minimal | 228 bytes wrong of 41354: renderer 213, matcher 33 |
| Painting, Full | 85 bytes wrong of 41354: renderer 83, matcher 49 |
| Ground-truth invariant violations | none |

## The change

In `AnimatedSprite2D::_validate_property`, `if (p_property.name == "frame" && playing) { ... }`
followed by `return;` becomes `if (p_property.name == "frame") { if (playing) { ... } return; }`.
The `playing` test moves into a new nested `if`, and the `return` moves inside the outer block
(lines 120-125).

## Mapping

- **Outer if dropped (7 visible, 6 invisible, `matcher`, `APTED("qualified_name")`).** The ground
  truth keeps before line 120's `if` as after line 120's outer `if`. It maps the before condition
  `p_property.name == "frame" && playing` to after `p_property.name == "frame"`, and it maps
  `playing` to the new inner `if (playing)` at line 121. Omnidiff deletes the before
  `if_statement`, its `(`, `)` and `playing`. It maps only the inner operand
  `p_property.name == "frame"` to the after condition, and it moves the before block `{ ... }` and
  its `}` to the new inner `if`'s block (after line 121:17 and 123:4). As a result, neither `if` is
  matched: the before `if` is deleted and both after `if`s are new. The ground truth's account is
  the coherent one.

## Painting

- **Renderer (Minimal 213 bytes, Full 83 bytes).** These runs stay when the human mapping is
  rendered. Both paintings mark the kept text as identical matched spans (`Move`): the outer
  condition, `{`, the body line (now one level deeper), `}` and `return;` (lines 120-125, both
  sides). Omnidiff leaves text that only shifted or was re-indented unpainted. Full also paints
  the indentation of the re-nested lines `Insert` and the removed space before `playing` `Delete`.
  Omnidiff does not paint these.
- **Matcher (Minimal 33 bytes, Full 49 bytes).** These come from the deleted outer `if`. The
  before condition `(p_property.name == "frame"` and `playing)` are painted `Delete`, and after
  `playing` is painted `Insert` where the painting has `Move`. In Full the after header
  `(p_property.name == "frame") {` is painted `Insert`, and the Move/Insert labels of the closing
  braces are swapped.

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`
- Where a fix would go: the APTED cost choice in the `qualified_name` call
  (`src/diff/solve_syntax_aware_matching.rs`) for a condition that splits into a nested `if`. On
  the renderer side, how `src/diff/text.rs` paints identical text that was re-nested.
- Ground truth questionable: no
