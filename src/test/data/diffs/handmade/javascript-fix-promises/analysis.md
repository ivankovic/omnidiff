# Accuracy analysis: javascript-fix-promises

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 235 nodes, 235 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 593: renderer 80, matcher 80 |
| Painting, Full | 30 bytes wrong of 593: renderer 120, matcher 90 |
| Ground-truth invariant violations | none |

## The change

Handmade. The `Promise` executor gains a `reject` parameter, `resolve("data");` (line 4) is
wrapped in `try { ... } catch (error) { reject(error); }` (after lines 4-8), and the caller
`fetchData().then(...)` is split over lines and gains a `.catch(...)` (after lines 13-19).

## Painting

There are no mapping mismatches, and omnidiff's own rendering agrees with the Minimal painting
exactly. All 30 Full bytes also appear when the human mapping is rendered, so they are the
renderer's:

- Full, None -> Move / Insert, indentation of the wrapped `resolve("data");` (28 bytes): the Full
  painting marks the 12 spaces of before line 4 as Move and the 16 spaces of after line 5 as
  Insert. omnidiff paints neither. `renderer`.
- Full, Move -> None, the final `;` of the `fetchData()...` statement (before line 11, after line
  19, 2 bytes): omnidiff paints it Move, the painting leaves it unpainted. `renderer`.

The human-mapping rendering is worse than omnidiff's own (renderer 80 / 120 bytes against 0 / 30):
rendered from the human file, the `.then((data) => { ... })` callback on before lines 9-11 and
after lines 14-16 is painted Move. With zero graded mismatches, the two mappings differ only in
what the rendering sees beyond the graded pairs (the fixture's test notes Identical vs
MatchButNotIdentical inside multi-map groups); this was not traced further. It costs omnidiff
nothing.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` Full rendering (indentation of re-nested lines; trailing `;`
  Move)
- Ground truth questionable: no
