# Accuracy analysis: java-defects4j-closure-34-codeprinter

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 6811 nodes, 6811 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 44271: renderer 0, matcher 0 |
| Painting, Full | 1 bytes wrong of 44271: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

One condition at line 334 gains a conjunct: `getLastChar() != ' '` becomes `getLastChar() != ' ' &&
op.charAt(0) != ','`.

## Painting

Minimal matches exactly. Full, `None -> Insert`, line 334 after side, 1 byte: the space between `'
'` and the inserted `&& op.charAt(0) != ','`. The Full painting includes the separating space in the
insertion; omnidiff's Full rendering starts the Insert at `&&`. The run is the same with the human
mapping, so it is the renderer's. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs Full gap handling: leading separator whitespace of an
  inserted operand
- Ground truth questionable: no
