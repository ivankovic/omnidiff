# Accuracy analysis: go-gin-gonic-gin-replace-replaced-with-replaceall

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 4964 nodes, 4964 graded, N:M floor 0 visible |
| Painting, Minimal | 64 bytes wrong of 10744: renderer 64, matcher 0 |
| Painting, Full | 2 bytes wrong of 10744: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

One test line (line 252) changes both `strings.Replace(x, " ", "", -1)` calls to
`strings.ReplaceAll(x, " ", "")`.

## Painting

All runs are the same with the human mapping, so they are the renderer's.

- Minimal, `Move -> None`, 58 bytes: the unchanged arguments `, " ", ""` (both calls) and
  `), strings.` on line 252, on both sides. The inserted `All` (3 bytes) shifts them right.
  Omnidiff paints them grey Move. The Minimal painting paints only `All` (Insert, twice) and
  `, -1` (Delete, twice). The long string literal argument has the same shift and is not
  painted, so the Move here comes from how the edited argument lists are rendered. The exact
  rule behind it was not traced further. `renderer`.
- Minimal, `Update -> Insert`, 6 bytes: omnidiff narrows `Replace` -> `ReplaceAll` to the added
  `All`, which is right, but labels it Update. The painting calls a pure suffix addition an
  Insert. `renderer`.
- Full, `None -> Move`, 2 bytes: the final `)` of `assert.Equal(...)` on both sides. The Full
  painting moves every shifted piece of the line, including this `)`. Omnidiff paints the other
  pieces Move but not this one. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs`. Minimal Move painting of same-line text that an edit
  beside it shifted (`shifted_by_an_edit_beside_it`), and the label of a narrowed Update whose old
  text is a prefix of the new one.
- Ground truth questionable: no
