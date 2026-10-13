# Accuracy analysis: java-defects4j-closure-110-scopedaliases

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 18 (11 visible) of 8205 nodes, 8212 graded, N:M floor 5 visible |
| Painting, Minimal | 72 bytes wrong of 41480: renderer 10, matcher 82 |
| Painting, Full | 83 bytes wrong of 41480: renderer 37, matcher 94 |
| Ground-truth invariant violations | none |

## The change

Closure-110: `ScopedAliases` also handles aliased function declarations. Besides new lines
(`isFunctionDecl`, `isHoisted`, a branch that rewrites the function into a `var`), the alias
value changes: before lines 368-369
`Node value = n.hasChildren() ? v.getInitialValue().detachFromParent() : null;` becomes after
lines 370-371 `Node value = v.getInitialValue() != null ? v.getInitialValue() : null;`, and
`Node varNode = parent;` (before 371) becomes `Node varNode = null;` (after 373).

## Mapping

All omnidiff reasons here are `APTED("qualified_name")` (APTED inside the method matched by
qualified name).

- The ternary condition, before line 368 / after line 370 (10 visible, 6 invisible). The ground
  truth deletes `n.hasChildren()` and matches the before `v.getInitialValue()` (inside
  `.detachFromParent()`) to both after occurrences, the new condition and the consequent (1:2).
  omnidiff maps `n.hasChildren()` to the new condition `v.getInitialValue()` (Update `n` -> `v`,
  `hasChildren` -> `getInitialValue`), maps the outer `v.getInitialValue().detachFromParent()`
  to the after consequent `v.getInitialValue()` and deletes the inner before
  `v.getInitialValue()`. Of the 10 visible, 5 (the after condition's tokens) are the N:M floor:
  one-to-one, only one after copy can match. The other 5 are omnidiff's choice to update a
  condition in which every identifier changed instead of deleting it. `matcher` (5) and
  `nm-floor` (5).
- `Node varNode = parent;` -> `= null;` (before 371, after 373), 1 visible: the ground truth's
  tree mapping matches the `identifier` `parent` to the `null_literal`; omnidiff deletes and
  inserts them (op Delete). Both of the ground truth's paintings paint exactly that, `parent`
  Delete and `null` Insert, so rendering the human mapping is what disagrees there (Update; the
  10 Minimal renderer bytes). `ground-truth`.

## Painting

- Matcher, Minimal 53 bytes: `n.hasChildren()` painted Update instead of Delete (15); the before
  and after consequent `v.getInitialValue()` left unpainted where the painting shows the
  duplicated expression as Move (19 + 19), because omnidiff matched the outer
  `.detachFromParent()` call instead of the inner expression. `matcher`.
- N:M, Minimal 19 bytes: the after condition `v.getInitialValue()` painted Update where the
  painting has Move. `nm-floor`.
- Full adds, from the renderer (37 bytes with the human mapping): `) {` after the extended
  `else if` condition (before 366, after 367) and the ternary's `?` and `:` painted Move because
  their columns shifted, the indentation of after line 371, and the `;` after `parent` / `null`.
  `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `nm-floor`, `ground-truth`, `renderer`
- Where a fix would go: APTED("qualified_name") in `src/diff/solve_syntax_aware_matching.rs`: an
  unwrapped call (`x.detachFromParent()` -> `x`) should keep the inner expression, and a
  condition with no shared identifier should not be updated in place.
- Ground truth questionable: yes: `parent` -> `null` on line 371/373 is a Match in the tree
  mapping but Delete + Insert in both paintings.
