# Accuracy analysis: java-defects4j-closure-141-purefunctionidentifier

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 11 (7 visible) of 7857 nodes, 7861 graded, N:M floor 3 visible |
| Painting, Minimal | 118 bytes wrong of 43672: renderer 170, matcher 142 |
| Painting, Full | 197 bytes wrong of 43672: renderer 193, matcher 58 |
| Ground-truth invariant violations | none |

## The change

Closure-141: `PureFunctionIdentifier.getCallableDefinitions` also follows `||` and `?:`. The
early guard `if (!NodeUtil.isGetProp(name) && !NodeUtil.isName(name)) { return null; }` (before
lines 179-181) is inverted into a wrapper `if (NodeUtil.isGetProp(name) || NodeUtil.isName(name))
{` (after 177) around the unchanged body, followed by a new `else if (... OR || ... HOOK)` branch
and a final `else { return null; }` (after 196-216). The body was not re-indented.

## Mapping

- The guard's block `{ return null; }`, 4 visible + 3 invisible. The ground truth turns the
  guard into the wrapper: the `if_statement` (before 179) matches the new outer `if`, the guard's
  `{` and `}` become the wrapper's braces (after 177 and 196), and its `return null;` matches the
  two new `return null;` (1:2, after 212 and 215). omnidiff maps the guard's `if` keyword and
  condition to the outer `if` too, but moves the whole identical block `{ return null; }` to the
  inner `else { return null; }` (after 211-213; reason `MovedSubtree`) and leaves the
  `if_statement` node itself unmapped (op Delete, `APTED("qualified_name")`). The block is torn
  from the `if` it belongs to. `matcher` (4 visible).
- The second copy of `return null;` (after 215), 3 visible + 1 invisible: the 1:2 group, one
  copy necessarily unmatched one-to-one; equals the N:M floor. `nm-floor`.

## Painting

- Renderer, Minimal 72 bytes (survive with the human mapping), None -> Move: the painting marks
  `List<Definition> result = Lists.newArrayList();` and the guard's `NodeUtil.isName(name))`
  as moved (the declaration and the `if` line swapped order) and `return result;` (now inside the
  wrapper). omnidiff's move attribution paints none of them. `renderer`.
- Matcher, Minimal 34 bytes: the wrapper's `{` (after 177) and `} ` (after 196) painted Insert
  where the painting has Move; the moved block's interior indentation and `}` painted Move on both
  sides, and the inner ` {` (after 211) left unpainted where the painting has Insert. `matcher`.
- N:M, 12 bytes: after line 215 `return null;` painted Insert, Move in the painting. `nm-floor`.
- Full, 197 bytes: mostly the same unattributed moves with their indentation and inner spaces,
  plus the deleted guard indentation (`renderer`, 193 with the human mapping), and the block and
  `return null;` runs above (`matcher` / `nm-floor`).

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`, `nm-floor`
- Where a fix would go: move attribution in `src/diff/text.rs`; `MovedSubtree` should not take a
  block away from an `if` whose keyword and condition are matched to the same target.
- Ground truth questionable: no.
