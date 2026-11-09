# Accuracy analysis: java-defects4j-chart-17-timeseries

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 9199 nodes, 9199 graded, N:M floor 0 visible |
| Painting, Minimal | 14 bytes wrong of 74560: renderer 14, matcher 0 |
| Painting, Full | 4 bytes wrong of 74560: renderer 4, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Chart-17: `TimeSeries.clone()` (line 857) stops delegating to `createCopy(0,
getItemCount() - 1)`, which throws on an empty series, and instead calls `super.clone()` and
deep-clones `data`. The local variable's type changes from `Object` to `TimeSeries`.

## Painting

No mapping mismatches. Every painting run also appears with the human mapping, so all are the
renderer's. All are on line 857, around the kept `clone =` of the local variable declarator.

- Minimal, 14 bytes, Move -> None: `clone =` on both sides. The declarator is matched, its name
  and `=` keep their text but are pushed right because `Object` became `TimeSeries`, and the
  rest of the declarator is replaced. omnidiff paints the two kept tokens `Move`; the Minimal
  painting leaves them unpainted. The displaced-move rule that `MINIMAL` uses to suppress such
  paint does not fire here, apparently because the rest of the row changed too. `renderer`.
- Full, 4 bytes, None -> Move: the spaces on either side of `clone =`, on both sides. The Full
  painting paints ` clone = ` as one `Move`; omnidiff paints `clone =` without the flanking
  spaces. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` displaced-move suppression (`shifted_by_an_edit_beside_it`)
  for kept tokens of a declarator whose type and value both changed; flanking whitespace of a
  `Move` under `FULL`
- Ground truth questionable: no
