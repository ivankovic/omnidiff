# Accuracy analysis: javascript-add-event-listener

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 146 nodes, 146 graded, N:M floor 0 visible |
| Painting, Minimal | 36 bytes wrong of 401: renderer 36, matcher 0 |
| Painting, Full | 17 bytes wrong of 401: renderer 17, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Handmade. `button.onclick = handleClick;` (line 7) becomes
`button.addEventListener('click', handleClick);`, and a second listener with an arrow function is
appended (after lines 8-10).

## Painting

The mapping matches the ground truth everywhere; every painting run survives with the human
mapping, so all of it is the renderer's.

- None -> Move, `button.` and `handleClick` on line 7, both sides (36 bytes Minimal; in Full
  `button.` only, 14 bytes). The painting marks the object and the handler as Move because they
  were reparented from an `assignment_expression` into a `call_expression`. `button.` keeps its
  row and column; `handleClick` keeps its row and its order but shifts from column 18 to 34.
  omnidiff's Minimal rendering paints neither; its Full rendering paints `handleClick` Move
  (agreeing with the Full painting) but leaves the in-place `button.` unpainted. `renderer`.
- Full only, single spaces on line 7 (3 bytes): before col 15 (the space before `=`) is Delete in
  the painting, and the spaces in front of `handleClick` on both sides are Move. omnidiff paints
  none of them. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` move painting (Move for reparented but in-place nodes), if
  that convention is wanted
- Ground truth questionable: no (Move for reparented text at the same position is the painter's
  convention; omnidiff's renderer does not follow it)
