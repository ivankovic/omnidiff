# Accuracy analysis: rust-multi-map-duplicate-calls

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 7 (4 visible) of 69 nodes, 76 graded, N:M floor 4 visible |
| Painting, Minimal | 30 bytes wrong of 105: renderer 26, matcher 50 |
| Painting, Full | 58 bytes wrong of 105: renderer 70, matcher 54 |
| Ground-truth invariant violations | none |

## The change

Handmade. `fn main() { foo(); foo(); foo(); bar(); }` becomes `fn main() { bar(); foo(); foo(); }`:
`bar();` moves to the top and one of three identical `foo();` lines goes. `description.md`:
"Requires N:M multi-map to solve perfectly".

## Mapping

All 7 mismatches (4 visible, 3 invisible) are the N:M floor. The ground truth maps the three old
`foo();` statements and the two new ones as one 3:2 all-to-all group. omnidiff maps two old
`foo();` (lines 2-3) to the two new ones (after lines 3-4), deletes the third (line 4, reason
`APTED("qualified_name")`), and moves `bar();`; a one-to-one mapping must leave one old `foo();`
unmatched, and the floor is exactly the 4 visible leaves of that statement. `nm-floor`.

## Painting

Both paintings encode the 3:2 group as one `Match`, which no one-to-one rendering can reproduce.

- Minimal, 30 bytes, `None/Delete -> Move`: the Minimal painting gives the group one span per
  statement on each side; the spans are byte-identical, so every `foo();` (before lines 2-4,
  after lines 3-4) resolves to `Move`. omnidiff paints the two kept `foo();` unchanged and the
  third `Delete`. With the human mapping the renderer does no better (it leaves `bar();` and one
  `foo();` unpainted and paints indentation `Move`), which is why the census splits the bytes
  between renderer and matcher; neither rendering can match a 3:2 group. `nm-floor`.
- Full, 58 bytes: the Full painting gives the group one multi-row span per side (before lines 2-4,
  after lines 3-4). The two spans differ in length, so the group resolves to `Update`: every
  `foo();` line, unchanged in text, is painted as updated (`None/Delete -> Update`, 50 bytes).
  The other 8 bytes are the indentation in front of the moved `bar();`, which the Full painting
  includes in its `Move`. `nm-floor` for the group; the indentation is `renderer`.

## Verdict

- Primary cause: `nm-floor`
- Other causes: `renderer`, `ground-truth`
- Where a fix would go: none in the matcher (omnidiff is at the N:M floor); how a 3:2 group of
  identical statements should be painted is Marko's call
- Ground truth questionable: yes: the Full painting's single multi-row span per side resolves the
  3:2 group to `Update`, painting five unchanged `foo();` lines as updated; the Minimal painting
  paints all of them `Move`
