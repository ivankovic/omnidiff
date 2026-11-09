# Accuracy analysis: tsx-shadcn-ui-ui-order-of-class-names

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 238 nodes, 238 graded, N:M floor 0 visible |
| Painting, Minimal | 36 bytes wrong of 1116: renderer 36, matcher 0 |
| Painting, Full | 36 bytes wrong of 1116: renderer 36, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In a Tailwind class string on line 16 the first two classes swap: `bg-border shrink-0` becomes
`shrink-0 bg-border`; the rest of the string is unchanged.

## Painting

Runs are identical with the human mapping, so they are the renderer's.

- 36 bytes per preset (18 per side, line 16 cols 9-26), Update -> Move (`renderer`). The class
  list is one `string_fragment` leaf mapped as an Update; omnidiff narrows it to the differing
  span `bg-border shrink-0` / `shrink-0 bg-border` and paints it Update. Both paintings mark the
  two classes as swapped (Move each; Full also matches the space between them). The renderer has
  no word-level move inside a string leaf.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs, token-level move detection when narrowing an Update of
  a string leaf
- Ground truth questionable: no
