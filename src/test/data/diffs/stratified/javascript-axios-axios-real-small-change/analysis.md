# Accuracy analysis: javascript-axios-axios-real-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 3 (1 visible) of 118 nodes, 118 graded, N:M floor 0 visible |
| Painting, Minimal | 17 bytes wrong of 716: renderer 1, matcher 16 |
| Painting, Full | 84 bytes wrong of 716: renderer 72, matcher 16 |
| Ground-truth invariant violations | none |

## The change

axios `lib/helpers/isAxiosError.js`: adds `var utils = require('./../utils');` (after lines 3-4)
and replaces the guard `(typeof payload === 'object')` with `utils.isObject(payload)` in the
`return` on line 10 (after line 12).

## Mapping

- 1 visible, before line 10 col 18 `payload` (inside `typeof payload`). The ground truth maps it
  Identical to the `payload` argument of `utils.isObject(payload)` on after line 12. omnidiff
  deletes it, `APTED("fast_fallback")`: its old parent chain (`parenthesized_expression`,
  `binary_expression`, `unary_expression`) is deleted and the new `call_expression` inserted, and
  the fallback does not carry the leaf across. `matcher`.
- 2 invisible, the outer `binary_expression` (`... && (payload.isAxiosError === true)`) on both
  sides. The ground truth reads the old one as removed and the new one as new; omnidiff pairs
  them as MatchButNotIdentical, `APTED("fast_fallback")`. Under unit costs renaming a gutted
  container is cheaper than delete-and-insert, so this is the cost function speaking. Invisible:
  the operator and right operand are matched either way. `matcher`.

## Painting

Matcher-caused (16 bytes Minimal, 14 Full): the `payload` mismatch above. Minimal: before
` payload ` on line 10 is Delete, where the painting has `payload` as Move and the spaces
around it unpainted; after `payload` on line 12 is Insert, painting Move. Full: the same
`payload` on both sides, Delete / Insert -> Move. `matcher`.

Renderer-caused (1 byte Minimal, 72 Full; survive with the human mapping):

- Full, Move -> None, the unchanged tail `&& (payload.isAxiosError === true);` on before line 10
  and after line 12 (70 bytes). omnidiff's Full rendering paints it Move under either mapping
  (its parent `binary_expression` is new in the human mapping, renamed in omnidiff's); the
  painting leaves an identical line tail unpainted. `renderer`.
- Full, None -> Delete, the spaces around `payload` in `typeof payload ===` (2 bytes), and
  Minimal, Delete -> None, the space between `===` and `'object'` (1 byte): gap whitespace inside
  the deleted guard. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`
- Where a fix would go: Full move painting in `diff::text` (a matched subtree whose only change is
  its parent should not be painted Move); `APTED("fast_fallback")` for the reparented `payload`
- Ground truth questionable: no
