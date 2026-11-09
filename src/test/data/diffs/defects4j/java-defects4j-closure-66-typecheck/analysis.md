# Accuracy analysis: java-defects4j-closure-66-typecheck

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 3 (2 visible) of 25164 nodes, 25164 graded, N:M floor 0 visible |
| Painting, Minimal | 3 bytes wrong of 133905: renderer 0, matcher 3 |
| Painting, Full | 18 bytes wrong of 133905: renderer 3, matcher 20 |
| Ground-truth invariant violations | none |

## The change

The `if (!NodeUtil.isObjectLitKey(n, n.getParent()))` at line 515 gains an `else` branch: the
comment `// Object literal keys are not typeable` moves into it and `typeable = false;` is added
(lines 516-519 after).

## Mapping

- Line 517 `}` (before, closing the then-block): the ground truth maps it to the then-block's `}` on
  line 516 (`} else {`) and treats the else-block's `}` on line 519 as new. omnidiff
  (`APTED("large_flat_subtree")`) maps it Identical to line 519's `}` and inserts line 516's. It
  also leaves the then-block itself unmapped (1 invisible, `Delete`). The comment that ended the
  then-block moved into the else-block, so the before block's last two children (comment, `}`) line
  up with the else-block, and APTED picked the wrong twin of the brace. 2 visible (the two braces).
  `matcher`.

## Painting

Matcher-caused, both presets: `Insert -> None` on line 516's `} ` and `None -> Insert` on line 519's
`}`: omnidiff paints the then-block brace inserted and leaves the new else-block brace unpainted,
the reverse of the painting. Minimal 3 bytes; Full 18 bytes (with indentation).

Renderer-caused, Full only (with the human mapping): the then-block `}` is painted Move on both
sides (1 + 2 bytes) because it shifted from line 517 to line 516; the Full painting leaves it alone.
`renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: APTED fallback (large_flat_subtree): choosing between twin closing braces
  when an else branch is added
- Ground truth questionable: no
