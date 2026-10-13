# Accuracy analysis: csharp-sonarr-sonarr-fix-comment-typo

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 4508 nodes, 4508 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 23999: renderer 0, matcher 0 |
| Painting, Full | 1 bytes wrong of 23999: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The commit fixes a doubled comment marker on line 234: `// // but we can still process other series
if needed.` becomes `// but we can still …`.

## Painting

Full only, 1 byte, identical with the human mapping, so renderer-owned:

- None -> Delete, before line 234 column 26: the space in front of the second `//`. Removing
  `// ` from `// // but` takes away one `//` and one of the two spaces around it. Both paintings
  delete `//` (column 27); the Full painting additionally deletes the space before it. omnidiff's
  Full rendering paints only `//` and leaves both spaces unpainted, so the removed space is not
  accounted for. Which of the two spaces is "the" removed one is arbitrary, but Full painting
  neither leaves the deletion short by a byte. `renderer`.

Minimal agrees exactly.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the in-comment Delete range under Full in src/diff/text.rs (the removed
  space is trimmed off)
- Ground truth questionable: no
