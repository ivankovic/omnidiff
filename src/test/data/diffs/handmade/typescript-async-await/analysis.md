# Accuracy analysis: typescript-async-await

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (0 visible) of 196 nodes, 196 graded, N:M floor 0 visible |
| Painting, Minimal | 12 bytes wrong of 443: renderer 12, matcher 0 |
| Painting, Full | 48 bytes wrong of 443: renderer 48, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A handmade fixture: the callback-style `fetchData(callback)` becomes an `async` function returning
`new Promise((resolve) => { setTimeout(...) })` (the `setTimeout` block is wrapped and indented,
`callback` becomes `resolve`), and the call `fetchData((data) => { console.log(data); });` becomes
an async IIFE `(async () => { const data = await fetchData(); console.log(data); })();`.

## Mapping

- Before line 7, 2 invisible, 0 visible. The ground truth maps the old call
  `fetchData((data) => {...})` to the IIFE's call expression and its `arguments` node to the
  IIFE's `arguments` (both MatchButNotIdentical). omnidiff leaves both containers unmapped (op
  Delete, reason `UnresolvedNode`) while agreeing on every visible node inside, including the
  brackets the ground truth moves. No visible effect. `grading-artifact` (invisible-only).

## Painting

All runs are the same with the human mapping, so all are the renderer's.

### Minimal, 12 bytes, None -> Move/Insert

Brackets only: the `)` of `function fetchData(...)` (before line 1, after line 1), the `((` and
`)` of `fetchData((data)` (before line 7), the closing `)` (before line 9), and on the after side
`(` and `()` of `(async ()` (line 9) and `)()` of `})();` (line 12). The Minimal painting moves
(or inserts) these brackets together with their partners, as the bracket rule requires (for
example `function fetchData(` is matched as moved, so its `)` is too). omnidiff's Minimal
punctuation filter drops the lone brackets, which splits pairs whose other half it paints.
`renderer`.

### Full, 48 bytes

Whitespace and punctuation around moved code: the spaces and ` {` of the signature line (Move),
the indentation of the re-indented `setTimeout` block and of the new lines (Insert), the `);` and
`)` closing the old and new calls (Move), plus one indentation run painted Move as gap text and a
`;` painted Move where the painting leaves it. omnidiff does not paint whitespace that belongs to
no node, and treats the punctuation differently from the painting. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `grading-artifact`
- Where a fix would go: `diff::text`: keep a bracket pair together in the Minimal punctuation
  filter; Full painting of re-indentation and of punctuation next to moved code.
- Ground truth questionable: no
