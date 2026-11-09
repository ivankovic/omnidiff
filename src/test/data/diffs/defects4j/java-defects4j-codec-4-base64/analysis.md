# Accuracy analysis: java-defects4j-codec-4-base64

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 10100 nodes, 10100 graded, N:M floor 0 visible |
| Painting, Minimal | 6 bytes wrong of 81276: renderer 0, matcher 6 |
| Painting, Full | 6 bytes wrong of 81276: renderer 0, matcher 6 |
| Ground-truth invariant violations | none |

## The change

Line 225, the no-argument `Base64()` constructor: `this(false);` becomes `this(0);`, delegating to
the line-length constructor instead of the url-safe one.

## Mapping

- Line 225, the sole argument `false` -> `0`: the ground truth maps the `false` literal to the
  `decimal_integer_literal` as an update. omnidiff (`APTED("large_flat_subtree_container")`) deletes
  `false` and inserts `0`; the two leaves have different kinds and no common text, so the
  edit-distance fallback prefers delete+insert. 1 visible. The sole argument of the same call in the
  same position is replaced, which the ground truth reasonably reads as an update. `matcher`.

## Painting

Matcher-caused (none with the human mapping), the same in both presets, 6 bytes on line 225: `Delete
-> Update` on `false` and `Insert -> Update` on `0`. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED fallback (large_flat_subtree_container): pairing a lone replaced
  literal of a different kind in the same argument slot
- Ground truth questionable: no
