# Accuracy analysis: javascript-d3-d3-nice-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 20 (14 visible) of 471 nodes, 471 graded, N:M floor 0 visible |
| Painting, Minimal | 142 bytes wrong of 1528: renderer 142, matcher 24 |
| Painting, Full | 233 bytes wrong of 1528: renderer 237, matcher 80 |
| Ground-truth invariant violations | none |

## The change

d3 `src/selection/append.js`: the ternary in the `return` on lines 16-18 inverts its condition
(`namespace && namespace !== document.documentElement.namespaceURI` becomes
`namespace === d3_nsXhtml && document.documentElement.namespaceURI === d3_nsXhtml`) and swaps its
two branches, `document.createElementNS(namespace, name)` and `document.createElement(name)`.

## Mapping

All 20 mismatches (14 visible, 6 invisible) are one disagreement, reason `APTED("qualified_name")`.
The ground truth reads the branches as swapped: before line 17 `createElementNS(...)` maps
Identical to after line 18, before line 18 `createElement(name)` to after line 17. omnidiff pairs
the branches by position (consequent with consequent), so each `call_expression`,
`member_expression` and `arguments` becomes MatchButNotIdentical, `createElementNS` and
`createElement` are Updates of each other, and before line 17 `namespace` and `,` are deleted
(the matching after nodes on line 18 inserted). The positional pairing costs two renames and a
two-node delete/insert; the swap costs nothing under the ground truth but needs both subtrees to
cross. The leaves `document`, `.`, `(`, `name`, `)` are paired Identical, just with the wrong
twin. `matcher`.

## Painting

Renderer-caused (142 bytes Minimal, 201 of 233 Full; all survive with the human mapping):

- None -> Move, the two swapped branches on lines 17-18, both sides (138 bytes in Minimal, the
  same spans split around the arguments in Full). The painting marks both calls Move. Rendered
  from the human mapping, which pairs them crosswise, omnidiff still paints neither branch.
  `renderer`.
- Minimal, Move -> None, `&&` on line 16, both sides (4 bytes). omnidiff paints the operator Move
  under either mapping; the Minimal painting leaves it unpainted. `renderer`.
- Full, None -> Move / Delete on line 16 (63 bytes): the Full painting marks `namespace` (same
  row and column on both sides), ` && ` and `document.documentElement.namespaceURI` as Move,
  since each changed parent when the condition was rewritten. omnidiff leaves them unpainted.
  `renderer`.

Matcher-caused (Full, 32 bytes): `namespace, name)` on before line 17 is Delete and on after line
18 Insert, painting Move, from the positional pairing above. `matcher`.

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`
- Where a fix would go: `diff::text` move painting (paint a sibling swap Move on at least one
  side); `APTED("qualified_name")` cost for swapped siblings
- Ground truth questionable: no (the Full painting's Move on the in-place `namespace` is its
  reparenting convention)
