# Accuracy analysis: javascript-microsoft-typescript-small-change-3

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 193 nodes, 193 graded, N:M floor 0 visible |
| Painting, Minimal | 14 bytes wrong of 796: renderer 14, matcher 0 |
| Painting, Full | 3 bytes wrong of 796: renderer 3, matcher 0 |
| Ground-truth invariant violations | none |

## The change

TypeScript baseline `noImplicitAnyInContextuallyTypesFunctionParamter.js`, line 10:
`regexMatchList.forEach(function (match) { return ''.replace(match, ''); });` becomes
`regexMatchList.forEach(match => ''.replace(match, ''));`.

## Painting

There are no mapping mismatches; every run survives with the human mapping, so all of it is the
renderer's.

- Minimal, Move -> None, `match` on line 10, both sides (10 bytes). The parameter moves out of
  `formal_parameters` (col 34) to be the bare arrow parameter (col 24). omnidiff paints it Move;
  the Minimal painting treats it as unchanged (the Full painting does mark it Move, and omnidiff's
  Full rendering agrees). `renderer`.
- Minimal, Delete -> None, four single spaces on before line 10 (cols 32, 40, 42, 72): omnidiff
  paints the gaps between the deleted `function`, `(`, `)`, `{`, `return`, `;`, `}` as Delete; the
  painting deletes the tokens one by one and leaves the spaces out. `renderer`.
- Full, None -> Move / Insert, three single spaces (before col 49 in front of `''.replace`, after
  cols 29 and 32 around `=>`): the Full painting includes them in its Move and Insert spans,
  omnidiff does not. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` gap-whitespace handling between adjacent deletions (Minimal)
  and around Move spans (Full); Minimal move painting for a parameter that loses its parentheses
- Ground truth questionable: no
