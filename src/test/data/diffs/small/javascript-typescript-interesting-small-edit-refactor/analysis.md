# Accuracy analysis: javascript-typescript-interesting-small-edit-refactor

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 3 (3 visible) of 89 nodes, 89 graded, N:M floor 0 visible |
| Painting, Minimal | 41 bytes wrong of 408: renderer 36, matcher 15 |
| Painting, Full | 10 bytes wrong of 408: renderer 7, matcher 11 |
| Ground-truth invariant violations | none |

## The change

TypeScript baseline `tupleElementTypes3.js`, line 7: the down-levelled destructuring
`var _a = [0, undefined], a = _a[0], b = _a[1];` becomes `var [a, b] = [0, undefined];`.

## Mapping

3 visible, line 7, reason `APTED("fast_fallback")`. The ground truth deletes the temporary `_a`
(col 5) and maps `a` (col 26) and `b` (col 37) Identical into the new `array_pattern`. omnidiff
pairs `_a` with the pattern's `a` as an Update and deletes the old `a` and `b` (inserting the new
`b`). The first declarator survives with its name slot changed from an `identifier` to an
`array_pattern`, and the fallback fills the name with the nearest identifier instead of carrying
the declared names across from the deleted declarators. `matcher`.

## Painting

Matcher-caused (11 bytes of Minimal, 9 of Full): from the mapping above. `_a` is painted Update
(the human: Delete), the old ` a ` and ` b ` Delete and the new ` b` Insert (the human: Move), and
the new `a` is unpainted (Minimal; narrowed Update) or Update (Full) where the painting has Move.
`matcher`.

Renderer-caused (30 bytes of Minimal's 41, 1 of Full's 10; 36 / 7 bytes with the human mapping):

- Minimal, None -> Move, the array literal ` [0, undefined]` on line 7, both sides (30 bytes). The
  painting marks `= [0, undefined]` Move (it moved from column 8 to 12); omnidiff's Minimal
  rendering leaves the array unpainted under either mapping, while its Full rendering paints it
  Move and agrees. `renderer`.
- With the human mapping only: the moved `b` is left unpainted on both sides, and the new `[`,
  `,`, `]` and the deleted `,` after the array are left unpainted where the painting marks them
  Insert / Delete (Minimal punctuation). `renderer`.
- Full, None -> Move, the space before `=` on after line 7 (1 byte). `renderer`.

## Verdict

- Primary cause: `matcher` (it owns all mapping mismatches and the disagreement under both
  presets; the renderer's larger byte count is one Minimal-only run)
- Other causes: `renderer`
- Where a fix would go: `APTED("fast_fallback")` (carry declarator names into a new
  `array_pattern` rather than renaming the deleted temporary); Minimal move painting of a moved
  initializer
- Ground truth questionable: no
