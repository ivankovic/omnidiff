# Accuracy analysis: java-defects4j-math-35-elitisticlistpopulation

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 914 nodes, 914 graded, N:M floor 0 visible |
| Painting, Minimal | 46 bytes wrong of 8842: renderer 46, matcher 0 |
| Painting, Full | 2 bytes wrong of 8842: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Math-35. Both `ElitisticListPopulation` constructors replace the direct field assignment
`this.elitismRate = elitismRate;` with a call to the validating setter
`setElitismRate(elitismRate);` (lines 51 and 65).

## Painting

All runs are renderer-owned: the human mapping renders the same bytes.

- **Minimal, 44 bytes (`Move -> None`).** On lines 51 and 65 the right-hand-side `elitismRate`
  becomes the argument of `setElitismRate(...)`. It is matched Identical, and omnidiff's Minimal
  rendering paints it Move on both sides (11 bytes x 4). The Minimal painting leaves it unpainted.
  The Full painting has it as a Match (Move), and there omnidiff agrees, so the two paintings differ
  on purpose here. `renderer`.
- **Whitespace, 2 bytes per preset (`Delete <-> None`).** These are the spaces around `=` in the
  deleted `this.elitismRate = `. Minimal paints the space before `=` as Delete on each line, and the
  Minimal painting leaves it. Full leaves one space unpainted on each line, and the Full painting
  deletes it. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs`: Minimal Move painting for an identical identifier
  re-parented from an assignment into a call argument; whitespace at the edge of deleted tokens
- Ground truth questionable: no
