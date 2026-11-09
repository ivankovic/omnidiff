# Accuracy analysis: java-refactor-constants

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 3 (3 visible) of 180 nodes, 181 graded, N:M floor 1 visible |
| Painting, Minimal | 23 bytes wrong of 594: renderer 0, matcher 23 |
| Painting, Full | 76 bytes wrong of 594: renderer 60, matcher 26 |
| Ground-truth invariant violations | none |

## The change

Handmade. The literal `3.14159`, used twice in `Circle` (lines 9 and 13), is extracted into a new
constant `private static final double PI = 3.14159;` (after line 2) and both uses become `PI`.
`description.md`: "Requires N:M mapping".

## Mapping

- Before lines 9 and 13, after line 2, the three `3.14159` literals. The ground truth puts all
  three in one 2:1 all-to-all group (both old literals moved into the constant). omnidiff deletes
  both old literals and inserts the new one, reason `APTED("qualified_name")`: the tree edit
  distance pass does not pair a literal with a node in a different subtree, and no move pass
  picked either twin up. A one-to-one mapping can satisfy two of the three (one literal mapped to
  the constant, the other deleted), so 1 visible mismatch is the N:M floor (`nm-floor`) and 2 are
  a move the matcher did not detect (`matcher`). Which of the two twins to map is `ambiguous`; the
  ground truth avoids the choice by mapping both.

## Painting

Matcher-caused (all 23 Minimal bytes, 22 of Full's):

- `Delete -> Move`, before lines 9 and 13, `3.14159` (7 + 7 bytes, both presets): the literals
  are deleted rather than moved, as above.
- `Insert -> None` (Minimal, ` 3.14159;`, 9 bytes) and `Insert -> Move` (Full, `3.14159;`, 8 bytes)
  on after line 2: the initializer is inserted rather than being the moved literal.

Renderer-caused (Full only, 60 bytes, all present with the human mapping too):

- `Move -> None`, the unchanged tails `* radius * radius;` (before line 9, after line 10) and
  `* radius;` (before line 13, after line 14), 54 bytes. The `radius` and `*` nodes slid left
  because `PI` is shorter than `3.14159`, and they also moved down a row because of the inserted
  constant line. `TextRanges::identical_or_move` exempts a single-row node slid by an edit beside
  it (`shifted_by_an_edit_beside_it`) only when `s.start_row == d.start_row` under Full, so the
  row shift turns the column shift into a `Move`. `renderer`.
- After line 2, 6 bytes: Full paint leaves the indentation (4) and the space before `3.14159`
  unpainted where the painting includes them in the `Insert`, and paints the `;` as `Insert` where
  the painting leaves it out. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `nm-floor`, `renderer`, `ambiguous`
- Where a fix would go: a move pass that maps a deleted literal to an identical inserted literal
  in a new declaration (2 of the 3 nodes); Full painting of tails displaced by an edit on a row
  that also shifted: the `s.start_row == d.start_row` gate in `src/diff/text.rs`
  `identical_or_move`
- Ground truth questionable: no
