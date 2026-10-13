# Accuracy analysis: java-defects4j-closure-108-scopedaliases

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 8468 nodes, 8468 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 42868: renderer 0, matcher 0 |
| Painting, Full | 228 bytes wrong of 42868: renderer 228, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-108: `ScopedAliases` records the declarations it injects in a new
`Set<Node> injectedDecls` (after line 259), clears it per scope (317), adds to it (434), and skips
those nodes when fixing JSDoc type nodes: line 581 `if (info != null) {` becomes
`if (info != null && !injectedDecls.contains(n)) {` (after 584).

## Painting

Mapping is exact. The fixture has a single painting, `Minimal`, which both presets are scored
against. Minimal matches it exactly; all 228 Full bytes survive with the human mapping
(renderer-owned).

- Full, Move -> None, 204 bytes: the `) {` after the extended condition and the unchanged `if`
  body (before lines 581-585, after 584-588: the `for` loop, `fixTypeNode(node);` and two `}`),
  102 bytes per side. Nothing here moved: the rows shifted by the three inserted lines and the
  condition grew in front of `) {`. Full's rendering is wrong on its own terms. `renderer`.
- Full, Insert -> None, 24 bytes: the indentation of the three inserted lines (after lines 259,
  317, 434). Full paints indentation by design; the Minimal painting does not. `grading-artifact`.

## Verdict

- Primary cause: `renderer`
- Other causes: `grading-artifact`
- Where a fix would go: Full displacement painting in `src/diff/text.rs` (row-shifted unchanged
  text after an edit on the same line, as in the pilot's displacement exemptions).
- Ground truth questionable: no.
