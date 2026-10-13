# Accuracy analysis: java-defects4j-csv-14-csvformat

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 6 (6 visible) of 14312 nodes, 14312 graded, N:M floor 0 visible |
| Painting, Minimal | 102 bytes wrong of 130790: renderer 60, matcher 42 |
| Painting, Full | 70 bytes wrong of 130790: renderer 28, matcher 42 |
| Ground-truth invariant violations | none |

## The change

Line 1039: the six character-literal bounds of the "needs quoting" test (`'0'`, `'9'`, `'A'`, `'Z'`,
`'a'`, `'z'`) are replaced by hex code points (`0x20`, `0x21`, `0x23`, `0x2B`, `0x2D`, `0x7E`); the
operators and the `c` operands are unchanged.

## Mapping

- Line 1039, the six `character_literal`s: the ground truth updates each to the
  `hex_integer_literal` in the same slot. omnidiff (`APTED("large_flat_subtree")`) deletes all six
  and inserts the six hex literals: the leaves differ in kind and share no text, so the
  edit-distance fallback prefers delete+insert (the same pattern as
  `java-defects4j-codec-4-base64`'s `false` -> `0`). 6 visible. `matcher`.

## Painting

Matcher-caused (absent with the human mapping), both presets, 42 bytes: the six literals are painted
`Delete` (before, 3 bytes each) and `Insert` (after, 4 bytes each) where both paintings paint
Update. In the Full runs they sit inside larger runs that start on the unpainted `c < ` or a space.
`matcher`.

Renderer-caused (also with the human mapping):

- Minimal, `Move -> None`, 60 bytes: the ten unchanged `|| c >` / `&& c <` runs between the literals
  are painted grey Move on both sides, because the longer hex literals shift them right. The
  same-row exemption (`node_uniquely_placed_on_its_row`) declines: `||`, `&&`, `c`, `<`, `>` each
  occur several times on the row, so "it stayed" cannot be read from the bytes. The Minimal painting
  leaves them unpainted. `renderer`.
- Full, `None -> Move`, 28 bytes: the Full painting marks the whole row Move, including the
  unshifted `c < ` at the start and the spaces around each literal; omnidiff paints the operator
  runs Move but not those. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: APTED fallback (large_flat_subtree): pairing replaced literals of a
  different kind slot by slot; renderer same-row exemption for repeated tokens
- Ground truth questionable: no
