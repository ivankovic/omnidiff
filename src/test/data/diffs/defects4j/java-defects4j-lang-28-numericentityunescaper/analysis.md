# Accuracy analysis: java-defects4j-lang-28-numericentityunescaper

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 771 nodes, 771 graded, N:M floor 0 visible |
| Painting, Minimal | 46 bytes wrong of 4800: renderer 46, matcher 0 |
| Painting, Full | 78 bytes wrong of 4800: renderer 78, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Lang-28. `NumericEntityUnescaper.translate` now writes a surrogate pair for code points
above `0xFFFF`. The existing `out.write(entityValue);` is wrapped in the `else` branch of a new
`if(entityValue > 0xFFFF) { ... } else { ... }`. Its line keeps its text and its 16-space
indentation.

## Painting

All runs are renderer-owned: rendering the human mapping gives the same bytes.

- **`None -> Move`, line 63 before / line 68 after.** Both paintings carry `out.write(entityValue);`
  as a Match, which renders as Move: the statement moved into the new `else` block. omnidiff's
  rendering leaves it unpainted on both sides. The mapping is the same as the ground truth's, but
  the statement's parent changed. This is 23 bytes per side in Minimal (46). In Full it is 39 bytes
  per side (78), because the Full painting also matches the 16-space indentation.
  `identical_or_move` in `src/diff/text.rs` paints a matched node Move only when its start column
  changes (or it crosses backwards). A row-only shift counts as an unrelated edit above, and this
  statement kept its column. `renderer`.

The convention is not uniform across fixtures. `java-defects4j-mockito-11-delegatingmethod` paints a
statement wrapped into a new `else` the same way (Move). `java-defects4j-math-95-fdistributionimpl`
(`ret = d / (d - 2.0);` wrapped into a new `if`) and
`java-defects4j-math-103-normaldistributionimpl` (a `return` wrapped into a new `try`) have the same
shape, also with unchanged indentation. Both leave the wrapped statement unpainted in both
paintings. omnidiff's rendering of a correctly mapped wrap matches math-95.
`java-defects4j-mockito-15-finalmockcandidatefilter` is a third variant: it leaves the wrap
unpainted in Minimal and paints it Move in Full.

## Verdict

- Primary cause: `renderer`
- Other causes: `ground-truth`
- Where a fix would go: `src/diff/text.rs` `identical_or_move`: a statement wrapped into a new block
  without a column change is painted Identical (if the paintings here are the convention)
- Ground truth questionable: yes, across fixtures: the same wrap shape is painted Move here and in
  mockito-11 but left unpainted in math-95 and math-103, and Move in Full only in mockito-15; which
  one is the convention is Marko's call
