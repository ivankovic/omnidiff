# Accuracy analysis: java-defects4j-csv-1-extendedbufferedreader

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1201 nodes, 1201 graded, N:M floor 0 visible |
| Painting, Minimal | 1 bytes wrong of 9397: renderer 21, matcher 20 |
| Painting, Full | 1 bytes wrong of 9397: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Line 58: `if (current == '\n')` becomes `if (current == '\r' || (current == '\n' && lastChar !=
'\r'))`, so a lone CR also counts as a line break.

## Painting

omnidiff's own rendering is off by 1 byte in each preset, both renderer-owned:

- Minimal, `Insert -> None`, line 58 after side, 1 byte: the space between the inserted `current ==
  '\r' ||` and the inserted `(`. The Minimal painting leaves it out as two separate inserts;
  omnidiff paints the gap between two inserted ranges. `renderer`.
- Full, `None -> Insert`, line 58 after side, 1 byte: the space between `'\n'` and the inserted
  `&&`, which the Full painting includes; omnidiff does not. `renderer`.

The Minimal renderer share (21 bytes) is larger than what omnidiff shows: rendering the HUMAN
mapping also paints `current ==` Move on both sides (20 bytes). The ground truth lists the leaves of
the before `current == '\n'` but not its `binary_expression`, so the rendered human mapping moves
the leaves one by one, where omnidiff maps the whole comparison and Minimal leaves it unpainted.
That part is an artifact of the sparse ground truth, not something a reader of omnidiff sees.
`grading-artifact`.

## Verdict

- Primary cause: `renderer`
- Other causes: `grading-artifact`
- Where a fix would go: src/diff/text.rs gap painting between adjacent inserted ranges (Minimal) and
  before an inserted operator (Full)
- Ground truth questionable: no
