# Accuracy analysis: java-defects4j-chart-9-timeseries

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 9226 nodes, 9226 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 74736: renderer 3, matcher 5 |
| Painting, Full | 0 bytes wrong of 74736: renderer 5, matcher 5 |
| Ground-truth invariant violations | none |

## The change

Defects4J Chart-9: in `TimeSeries.createCopy(RegularTimePeriod, RegularTimePeriod)` the empty-range
test `if (endIndex < 0)` (line 944) becomes `if ((endIndex < 0)  || (endIndex < startIndex))`.
Which pair of parentheses in the new condition is the old one is a genuine choice, and the fixture
carries four paintings for it: `Minimal`/`Full` x `inner`/`outer parenthesis`. The best-matching
ones here are the `inner parenthesis` paintings.

## Painting

No mapping mismatches; Full agrees with its painting byte for byte. The human tree mapping leaves
the condition's parenthesis nodes unlisted (ungraded), which is why rendering it differs from
rendering omnidiff's (the 3-5 bytes of "renderer" and "matcher" in the table): those runs are on
the parentheses only and do not affect omnidiff's own score.

- Minimal, 2 bytes, Insert -> None: the two spaces between `(endIndex < 0)` and `||` on after line
  944. omnidiff paints the inserted `)`, the spaces and `||` as one run; the Minimal painting
  paints `)` and `||` and leaves the gap unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` gap whitespace between two inserted tokens under `MINIMAL`
- Ground truth questionable: no (the parenthesis ambiguity is covered by the alternative
  paintings)
