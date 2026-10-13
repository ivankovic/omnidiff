# Accuracy analysis: java-defects4j-closure-67-analyzeprototypeproperties

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 3 (0 visible) of 8707 nodes, 8707 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 44818: renderer 0, matcher 0 |
| Painting, Full | 0 bytes wrong of 44818: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The condition of the `if` at lines 316-318 gains a fourth conjunct: the lone `) {` on line 318
becomes `&& assign.getParent().getType() == Token.EXPR_RESULT) {`.

## Mapping

All 3 mismatches are invisible and concern the left-nested `binary_expression` chain (`((n != null
&& isVarOrSimpleAssignLhs(..)) && n.getType() == GETPROP)` before, one level deeper after).

- The ground truth lists the after `n != null` `binary_expression` (line 316, the innermost of the
  four) as `insert`, while its three children `n`, `!=`, `null` are listed `identical` to the
  children of the before `n != null`, whose own `binary_expression` is not listed. The grader turns
  the unlisted before chain into a 3-before/3-after multi-map group.
- omnidiff (`APTED("large_flat_subtree")`) maps before `n != null` Identical to after `n != null`
  and leaves the genuinely new outermost after `binary_expression` unmapped, so the group has 2
  pairs, not 3, and the after `n != null` is not "new".

omnidiff's answer is the natural one: the node and all of its children are identical. Marking an
after node inserted whose every child maps into one unlisted before node of identical text looks
like a slip in the ground truth. Nothing is visible: both paintings and both renderings agree.
`ground-truth`.

## Verdict

- Primary cause: `ground-truth`
- Other causes: `grading-artifact`
- Where a fix would go: ground truth (Marko's call); nothing in the engine
- Ground truth questionable: yes: after n != null binary_expression (line 316) is listed insert
  while its children are identical to the unlisted before n != null; the new node is the outermost
  conjunction
