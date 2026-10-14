# Accuracy analysis: java-defects4j-mockito-31-returnssmartnulls

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 4 (3 visible) of 1113 nodes, 1117 graded, N:M floor 3 visible |
| Painting, Minimal | 9 bytes wrong of 6324: renderer 4, matcher 6 |
| Painting, Full | 11 bytes wrong of 6324: renderer 8, matcher 6 |
| Ground-truth invariant violations | none |

## The change

Defects4J Mockito-31. `ReturnsSmartNulls.formatMethodCall()` includes the invocation's arguments. A
new `String args = Arrays.toString(...)` is added, and `"()"` becomes `"(" + args.substring(1,
args.length() - 1) + ")"`. `java.util.Arrays` is imported. `description.md`: "Requires N:M mapping."

## Mapping

- **Split literal (3 visible, 1 invisible), `nm-floor`.** The ground truth puts the old string
  literal `"()"` (line 60) in an all-to-all group with both new literals `"("` and `")"` on line 62.
  omnidiff (`APTED("qualified_name")`) maps it to `"("` and inserts `")"` with its two quotes and
  fragment. These 3 visible nodes are exactly the fixture's N:M floor of 3.

## Painting

- **N:M, 5 bytes (Minimal and Full).** `")"` on line 62 (with a tab in Minimal) is painted Insert.
  The painting matches its `)` and quotes to the old literal. `nm-floor`.
- **Renderer, 4 bytes Minimal, 8 Full.** The painting records the split literal as Match entries per
  character: `"`, `(`, `)`, `"`. Their identical text is graded as Move, including `"(`, which stays
  at column 45-47. omnidiff leaves `"()` (line 60) and `"(` (line 62) unpainted, also with the human
  mapping. In Full the rendering also paints the shifted `;` Move. It leaves unpainted the space and
  tab that the Full painting inserts. `renderer`.

## Verdict

- Primary cause: `nm-floor`
- Other causes: `renderer`
- Where a fix would go: none for the N:M part (one-to-one cannot reach it); `src/diff/text.rs` for
  the in-place pieces of a split literal
- Ground truth questionable: no
