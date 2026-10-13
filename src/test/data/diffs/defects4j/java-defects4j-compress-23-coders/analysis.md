# Accuracy analysis: java-defects4j-compress-23-coders

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 5 (3 visible) of 4757 nodes, 4757 graded, N:M floor 0 visible |
| Painting, Minimal | 9 bytes wrong of 23680: renderer 4, matcher 7 |
| Painting, Full | 59 bytes wrong of 23680: renderer 51, matcher 11 |
| Ground-truth invariant violations | none |

## The change

Line 112: `dictSize |= (coder.properties[i + 1] << (8 * i));` becomes `dictSize |=
(coder.properties[i + 1] & 0xffl) << (8 * i);`, masking the byte before the shift. The parentheses
now enclose the mask instead of the whole shift.

## Mapping

- Line 112, the before `( ... )` and the `<<` (3 visible), plus the `parenthesized_expression` and
  the shift `binary_expression` (2 invisible): the ground truth maps the before parentheses to the
  after inner `(coder.properties[i + 1] & 0xffl)` and the before `<<` to the after outer `<<`, with
  the before shift in a 1:2 group with the after shift and mask. omnidiff
  (`APTED("qualified_name")`) deletes the before parentheses, the `<<` and the
  `parenthesized_expression`, and inserts the after ones. The edit swaps the nesting (shift inside
  parens becomes parens inside shift), and the edit-distance fallback does not carry the operator
  and brackets across the inversion. `matcher`.

## Painting

Matcher-caused (absent with the human mapping):

- Minimal, 7 bytes on line 112: `Delete -> None` on the before `<<` (2), `Insert -> Move` on the
  after `(` (1) and `) <<` (4). omnidiff paints the operator and brackets deleted and re-inserted.
- Full, 11 bytes: the same tokens, with the spaces around them.

Renderer-caused (also with the human mapping):

- Both presets: the paintings mark the before `(` and `)` Move (they now enclose a different
  subexpression; the `)` moved from column 64 to 61). The human mapping maps them, but omnidiff
  leaves them unpainted. Minimal 4 bytes. `renderer`.
- Full only: the Full painting marks `coder.properties[i + 1] ` Move on both sides (re-parented into
  the new `&`, same columns) and a space; omnidiff paints nothing that kept its row and column. 51
  bytes. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: APTED fallback (qualified_name) on a nesting swap; secondarily Full move
  painting of re-parented nodes that kept their columns
- Ground truth questionable: no
