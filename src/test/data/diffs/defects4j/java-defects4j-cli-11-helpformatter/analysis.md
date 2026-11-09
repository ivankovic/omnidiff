# Accuracy analysis: java-defects4j-cli-11-helpformatter

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 7688 nodes, 7688 graded, N:M floor 0 visible |
| Painting, Minimal | 14 bytes wrong of 60826: renderer 14, matcher 0 |
| Painting, Full | 1 bytes wrong of 60826: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-11: in `HelpFormatter.appendOption` (line 632) the test `(option.getArgName() !=
null)` becomes `option.hasArgName()`, so an empty argument name no longer prints `<>`.

## Painting

No mapping mismatches. Every painting run also appears with the human mapping, so all are the
renderer's. All are on line 632.

- Minimal, 14 bytes, Move -> None: `option.` of the second operand, on both sides. Both mappings
  pair it; its parent changed (from inside `( ... != null)` to the operand itself) and it shifted
  one column left with the dropped `(`. omnidiff paints it `Move`; the Minimal painting leaves it
  unpainted (the Full painting agrees with the `Move`). `renderer`.
- Full, 1 byte, None -> Delete: the space before the deleted `!= null`. The Full painting starts
  the deleted run on the space; omnidiff starts it at `!=`. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` move painting under `MINIMAL` for a re-parented node that
  only lost an enclosing bracket; gap whitespace before a deleted run under `FULL`
- Ground truth questionable: no
