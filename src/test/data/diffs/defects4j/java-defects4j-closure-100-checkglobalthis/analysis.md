# Accuracy analysis: java-defects4j-closure-100-checkglobalthis

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1668 nodes, 1668 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 11245: renderer 0, matcher 0 |
| Painting, Full | 2 bytes wrong of 11245: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-100: `CheckGlobalThis` gains a check on the parent's token type in `shouldTraverse`
(new lines 99-105), and `isInPrototypeContext`'s fallback `return false;` (before line 146)
becomes `return parent != null && NodeUtil.isGet(parent);` (after line 153).

## Painting

Mapping is exact and Minimal matches its painting. The 2 Full bytes are renderer-owned (they
survive with the human mapping).

- Full, before line 146 / after line 153, Move -> None, 1 byte per side: the `;` of the
  rewritten `return` is matched to itself, and because the expression in front of it grew (and
  the rows shifted by the 7 inserted lines) the Full renderer paints the `;` Move. The Full
  painting leaves it unpainted: the terminator of an updated statement did not move in any sense
  a reader perceives. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full displacement painting in `src/diff/text.rs` (a statement's own
  terminator shifted only by an edit in front of it on the same line).
- Ground truth questionable: no.
