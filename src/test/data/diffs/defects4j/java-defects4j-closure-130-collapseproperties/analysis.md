# Accuracy analysis: java-defects4j-closure-130-collapseproperties

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (0 visible) of 11580 nodes, 11580 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 69389: renderer 0, matcher 0 |
| Painting, Full | 0 bytes wrong of 69389: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-130: `CollapseProperties.inlineAliases` skips names defined in externs: the condition on
line 172 gains a new leftmost operand, `!name.inExterns && name.globalSets == 1 && ...`.

## Mapping

- 2 invisible mismatches, no visible ones. Java's `&&` chain is left-nested, so a new leftmost
  operand adds a `binary_expression` level at the bottom of the chain. omnidiff maps the before
  `name.globalSets == 1` node to the new after node `!name.inExterns && name.globalSets == 1`
  (MatchButNotIdentical, `APTED("large_flat_subtree")`), where the ground truth expects the
  identical after `name.globalSets == 1` and places the after `&&` level in a 2:3 multi-map
  group. Every leaf maps as the ground truth says and the painting score is 0 bytes, so a
  reader sees nothing. `grading-artifact`.

## Painting

No disagreement under either preset (the single painting, "Only one solution", is matched
exactly).

## Verdict

- Primary cause: `grading-artifact`
- Other causes: none
- Where a fix would go: none needed for readers; the internal level choice is in the
  `large_flat_subtree` APTED pass.
- Ground truth questionable: no.
