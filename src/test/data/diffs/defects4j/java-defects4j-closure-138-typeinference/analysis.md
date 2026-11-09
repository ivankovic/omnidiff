# Accuracy analysis: java-defects4j-closure-138-typeinference

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 29 (20 visible) of 17352 nodes, 17352 graded, N:M floor 0 visible |
| Painting, Minimal | 111 bytes wrong of 86158: renderer 2, matcher 111 |
| Painting, Full | 151 bytes wrong of 86158: renderer 24, matcher 133 |
| Ground-truth invariant violations | 2 |

## The change

Closure-138 (second file of the same commit): in `TypeInference.traverseName`, the condition
`if (var != null && !(var.isTypeInferred() && unflowableVarNames.contains(varName))) {` (before
lines 632-633) is reduced to `if (var != null) {`; the test is split into new locals
`isInferred`, `unflowable` and `nonLocalInferredSlot` placed between the existing comments
(after 638-648), and the old body is wrapped in a new
`if (!unflowable && !nonLocalInferredSlot) { ... }` (after 650-655).

## Mapping

- `var.isTypeInferred()` extracted into `boolean isInferred = var.isTypeInferred();`, 10 visible
  + 4 invisible (before 633, after 638). The ground truth deletes it from the condition and
  inserts the new one; omnidiff matches the two (`IdenticalHash` on the call,
  `IdenticalHashOfAncestor` on its tokens: it is an identical subtree on both sides), i.e. shows
  the extraction as a move. An extraction is faithfully described either way. `ambiguous`.
- The argument list `(varName)` of the deleted `unflowableVarNames.contains(varName)` (before
  633), 6 visible + 2 invisible: omnidiff maps it alone to the argument list of the new
  `getSlot(varName)` on after line 648 (`APTED("large_flat_subtree")`), while the `contains`
  call it belongs to is deleted. A bare argument list carried into an unrelated call. `matcher`.
- `// inferred variable.` (before 641 -> after 643), 1 visible: identical, and its neighbouring
  comments are matched, but omnidiff deletes and re-inserts this one (op Delete,
  `APTED("large_flat_subtree")`). `matcher`.
- The outer condition's closing `)` (before 633:74), 1 visible: omnidiff leaves the condition's
  `parenthesized_expression` and this `)` unmatched (op Delete, `APTED("large_flat_subtree")`)
  although the `if` is matched; it paints the after `) {` on line 632 Insert. `matcher`.
- The block braces, 2 visible: omnidiff maps the before block (and its `{`, before 633:76) to the
  new inner block of `if (!unflowable ...)` (after 650:51) via `APTED("large_flat_subtree")`; the
  ground truth maps the `{` to the outer block (after 632). The ground truth's own tree mapping is
  inconsistent here: it pairs the before block's `}` with the inner block's `}` (after 655) and
  inserts the outer `}` (after 656). invariants.txt flags both pairs (rule 3). omnidiff's choice
  agrees with the ground truth's closing brace, the ground truth's opening brace and its paintings
  agree with each other. `ground-truth`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Painting

- Minimal, 111 bytes, all matcher-owned except 2: `var.isTypeInferred()` painted Move / left
  unpainted where the painting deletes and inserts it (43 bytes, `ambiguous`); the stolen
  `(varName)` painted Move (19, `matcher`); the comment painted Delete + Insert (42, `matcher`);
  the condition's `)` / `) {` (4, `matcher`); the inner `if`'s ` {` and `}` (3, `ground-truth`).
- Full, 151 bytes: the same groups with indentation (matcher 133), plus renderer runs that
  survive with the human mapping (24 bytes): the outer `) {` and a `}` painted Move after the
  condition shrank and rows shifted, one deleted space left unpainted, and the `}` lines on after
  655-656 where the inconsistent human brace mapping renders the wrong `}` as inserted.
  `renderer` / `ground-truth`.

## Verdict

- Primary cause: `matcher`
- Other causes: `ambiguous`, `ground-truth`, `renderer`
- Where a fix would go: the `large_flat_subtree` APTED pass
  (`src/diff/solve_large_flat_subtrees.rs`): do not map a lone argument list out of a deleted
  call, keep identical sibling comments, and keep the matched `if`'s condition parentheses.
- Ground truth questionable: yes: invariant 3 twice on after lines 632/656 and 650/655; the tree
  mapping matches the before block's `{` to the outer block and its `}` to the inner block.
