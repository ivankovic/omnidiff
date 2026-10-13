# Accuracy analysis: java-defects4j-math-95-fdistributionimpl

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1164 nodes, 1164 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 13897: renderer 0, matcher 0 |
| Painting, Full | 1 bytes wrong of 13897: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Math-95. `FDistributionImpl.getInitialDomain` initialises `ret` to `1.0`, and only uses
the mean `d / (d - 2.0)` when `d > 2.0`. The existing assignment and its comment are wrapped in the
new `if`. They keep their 12-space indentation, which was over-indented before.

## Painting

One byte, Full only, and renderer-owned (it survives with the human mapping). On line 144 the Full
painting inserts the space between `ret` and the new `= 1.0`. The Full rendering starts the
insertion at `=`. Full is the preset that paints padding (only Minimal trims edge whitespace), and
this fixture has its own Full painting. So the Full rendering is off on its own terms: `renderer`.

Both paintings leave the wrapped `ret = d / (d - 2.0);` unpainted, and omnidiff agrees.
`java-defects4j-lang-28-numericentityunescaper` and `java-defects4j-mockito-11-delegatingmethod`
paint the same shape (a statement wrapped into a new block, indentation unchanged) as Move.
`java-defects4j-mockito-15-finalmockcandidatefilter` paints it Move in Full only.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` Full: where an insertion inside a line starts (the space
  before it)
- Ground truth questionable: yes, across fixtures only: this wrap is left unpainted, while lang-28
  and mockito-11 paint the same shape Move and mockito-15 paints it Move in Full only; Marko's call
  which is the convention
