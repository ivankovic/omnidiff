# Accuracy analysis: java-defects4j-closure-10-nodeutil

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 39598 nodes, 39598 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 190790: renderer 2, matcher 0 |
| Painting, Full | 0 bytes wrong of 190790: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-10: `NodeUtil.mayBeString` for an expression asks whether *any* result may be a string
rather than whether *all* do: line 1417 `allResultsMatch(...)` becomes `anyResultsMatch(...)`.

## Painting

Mapping is exact; the only disagreement is 2 bytes in Minimal, renderer-owned (it survives with
the human mapping).

- Minimal, line 1417, None -> Update, 1 byte per side: the Minimal painting paints the word
  `all` -> `any` as the Update. The renderer narrows the identifier update to the differing
  characters `ll` / `ny` and leaves the shared leading `a` unpainted. The Minimal convention is to
  paint the changed word, not the changed characters. `renderer`.
- Full: no disagreement (whole identifier painted Update on both sides).

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Minimal update narrowing in `src/diff/text.rs` (narrow to word
  boundaries inside a camelCase identifier, not to characters).
- Ground truth questionable: no.
