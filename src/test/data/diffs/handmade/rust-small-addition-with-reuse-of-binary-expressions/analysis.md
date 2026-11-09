# Accuracy analysis: rust-small-addition-with-reuse-of-binary-expressions

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 10 (5 visible) of 53170 nodes, 53170 graded, N:M floor 0 visible |
| Painting, Minimal | 140 bytes wrong of 326831: renderer 130, matcher 52 |
| Painting, Full | 181 bytes wrong of 326831: renderer 133, matcher 86 |
| Ground-truth invariant violations | none |

## The change

Handmade on a 3,500-line copy of omnidiff's own `ranges()`: the condition `if s.start_column ==
d.start_column && !crossed_backwards {` (line 374) becomes `if !column_shift_is_meaningful &&
!crossed_backwards {` (after line 399), with a 20-line comment and two new `let`s inserted above
it; the old comparison is reused, negated, in `let column_shift_is_meaningful = s.start_column !=
d.start_column && !shifted_within_its_own_line;` (after lines 397-398).

## Mapping

Five visible and five invisible mismatches, one cause, reason `APTED("qualified_name")`, line
374. The ground truth keeps the `if` (its `&&` and `!crossed_backwards` identical) and moves the
comparison `s.start_column == d.start_column` into the new `let`, updating `==` to `!=`. omnidiff
deletes the old `if_expression`'s header: `if`, `&&`, `!crossed_backwards`, the comparison's `==`
and the enclosing `binary_expression`s, inserting the new ones. The tree edit distance does not
keep an `if` whose condition was rebuilt around a new identifier, and the reuse of the
comparison in another statement is not picked up as a move with an update. `matcher`.

## Painting

Matcher-caused (Minimal 52 bytes, Full 86): the old `if` and `&& !crossed_backwards` painted
`Delete` and the new ones `Insert` where the paintings keep or move them, and the reused
comparison painted `Move` where the paintings update it.

Renderer- or painting-caused (present with the human mapping too):

- Minimal 28 bytes, `None -> Insert`: the space (or three, on bullet lines) after `//` on 18 of
  the new comment lines (after lines 374-394), the renderer's gap between `//` and the comment
  text. `renderer`.
- Minimal 60 bytes, Full about 62: the moved comparison (line 374 -> after line 397). The paintings
  update it whole (`s.start_column == d.start_column` -> `s.start_column != d.start_column`);
  omnidiff, even with the human mapping, paints the operator `Update`, the left operand `Move` and
  the right operand nothing (Minimal), or only the operator (Full). `renderer`.
- Minimal 42 bytes, `None -> Move`: `&& !crossed_backwards` on line 374 / after line 399. It is the
  identical tail of a row whose head changed, so Minimal's `node_untouched_on_its_row` exempts it
  as displaced; the Minimal painting marks it `Move`, against painting rule 3 (an identical line
  tail stays unpainted even when the line moved). `ground-truth`.
- Full 70 bytes, `None -> Move`: the indentation and `if ` of line 374 / after line 399. The `if`
  moved 25 rows down below the inserted lines but kept its column, which `identical_or_move`
  reads as an unrelated edit above; the Full painting marks it moved. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`, `ground-truth`
- Where a fix would go: `src/diff/text.rs` (painting of a moved-and-updated `binary_expression`;
  gap after `//`; Full row-only shifts); APTED (`APTED("qualified_name")`) keeping an `if` whose
  condition was rebuilt, plus a move-with-update for the reused comparison
- Ground truth questionable: yes: the Minimal painting marks the unchanged line tail `&&
  !crossed_backwards` as moved, against painting rule 3
