# Accuracy analysis: java-defects4j-mockito-15-finalmockcandidatefilter

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 589 nodes, 589 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 3170: renderer 0, matcher 0 |
| Painting, Full | 168 bytes wrong of 3170: renderer 168, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Mockito-15. In `FinalMockCandidateFilter`'s anonymous `thenInject()`, the field injection
first tries `new BeanPropertySetter(fieldInstance, field).set(matchingMock)`. It falls back to the
existing `new FieldSetter(...).set(matchingMock);` only when that returns false. A new import is
added. The wrapped statement keeps its 28-space indentation.

## Painting

Full only, 168 bytes, renderer-owned (identical with the human mapping). The Full painting carries
`new FieldSetter(fieldInstance, field).set(matchingMock);` (line 25 before, line 27 after),
indentation included, as a Match, which renders as Move. The Minimal painting leaves it unpainted.
omnidiff paints it Identical in both presets. `identical_or_move` in `src/diff/text.rs` paints Move
only on a meaningful start-column change, and the statement did not change column. 84 bytes per
side. `renderer`.

The paintings here follow the "Move under Full only" reading that `solve_wrap_growth` cites for
`rust-add-if`. Other fixtures with the same shape (a statement wrapped into a new block, indentation
unchanged) disagree: `java-defects4j-lang-28-numericentityunescaper` and
`java-defects4j-mockito-11-delegatingmethod` paint it Move in both presets.
`java-defects4j-math-95-fdistributionimpl` and `java-defects4j-math-103-normaldistributionimpl`
leave it unpainted in both.

## Verdict

- Primary cause: `renderer`
- Other causes: `ground-truth`
- Where a fix would go: `src/diff/text.rs` `identical_or_move` (Full): a statement wrapped into a
  new block without a column change is painted Identical
- Ground truth questionable: yes, across fixtures: the wrap is Move in Full only here, Move in both
  presets in lang-28 and mockito-11, and unpainted in math-95 and math-103; Marko's call which is
  the convention
