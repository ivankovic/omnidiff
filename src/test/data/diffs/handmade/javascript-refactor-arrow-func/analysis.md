# Accuracy analysis: javascript-refactor-arrow-func

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 172 nodes, 172 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 354: renderer 4, matcher 0 |
| Painting, Full | 14 bytes wrong of 354: renderer 14, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Handmade. Two callbacks `function(n) { return ...; }` passed to `numbers.map` and
`numbers.filter` (before lines 3-5 and 7-9) become arrow functions `n => n * 2` and
`n => n % 2 === 0` (after lines 3 and 5).

## Painting

There are no mapping mismatches; every run survives with the human mapping, so all of it is the
renderer's.

- Minimal, None -> Delete, the `;` after the returned expression and the closing `}` of each old
  function body (before lines 4-5 and 8-9, 4 bytes). The Minimal painting deletes them, as it
  deletes `) {`. omnidiff's Minimal rendering agrees on `) {` but leaves `;` and `}` unpainted, so
  its rendering paints the opening brace of the pair and not the closing one. `renderer`.
- Full, Move -> None, `);` closing each call (before lines 5 and 9, after lines 3 and 5, 8 bytes):
  the arguments' `)` moves up two lines when the body collapses, and omnidiff's Full rendering
  paints it Move; the painting treats it as unchanged. `renderer`.
- Full, None -> Delete / Insert, single spaces (6 bytes): the space after `return` (before lines 4
  and 8) and the spaces around each `=>` (after lines 3 and 5) are painted in the Full painting,
  not by omnidiff. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` (Minimal: paint a deleted `}` with its deleted `{`; Full: no
  Move for a closing `)` whose only change is its row; edge spaces of Insert / Delete spans)
- Ground truth questionable: no
