# Accuracy analysis: java-defects4j-jacksoncore-12-utf8streamjsonparser

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 49002 nodes, 49002 graded, N:M floor 0 visible |
| Painting, Minimal | 12 bytes wrong of 252935: renderer 12, matcher 0 |
| Painting, Full | 2 bytes wrong of 252935: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Line 3622, the arguments of a `JsonLocation` constructor: `getTokenCharacterOffset()` becomes
`_tokenInputTotal` and `getTokenLineNr()` becomes `_tokenInputRow`; the `-1L` between them is kept.

## Painting

Both runs are renderer-owned (same with the human mapping).

- Minimal, `Move -> None`, line 3622, 6 bytes each side (`, -1L,`): the unchanged middle argument is
  shifted left by the shorter replacement before it, and omnidiff paints it grey Move. Its edits sit
  on both sides, so the prefix/suffix test fails, and the unique-placement test requires the same
  columns. The Minimal painting leaves it unpainted. `renderer`.
- Full, `None -> Move`, 1 byte each side: the Full painting's Move covers `, -1L, ` including the
  trailing space; omnidiff's Full Move stops before it. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs same-row move exemptions (node_untouched_on_its_row /
  node_uniquely_placed_on_its_row) for a shifted node between two edits
- Ground truth questionable: no
