# Accuracy analysis: csharp-sonarr-sonarr-ok-small-edit

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 909 nodes, 909 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 4446: renderer 0, matcher 0 |
| Painting, Full | 2 bytes wrong of 4446: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`IsSatisfiedBy` takes a `ReleaseDecisionInformation information` instead of `SearchCriteriaBase
searchCriteria` (line 23), reads `searchCriteria` from it in a new first statement (after line 24),
and the now unused `using NzbDrone.Core.IndexerSearch.Definitions;` goes (before line 5).

## Painting

Full only, 2 bytes, identical with the human mapping, so renderer-owned:

- Move -> None, before line 23 / after line 22: the closing `)` of the parameter list, which
  slides right because the replaced parameter is longer. The `shifted_by_an_edit_beside_it`
  exemption in `identical_or_move` (src/diff/text.rs) covers a single-row node slid sideways by an
  edit on its row, but under Full (`paint_displaced_moves`) it requires the same row index, and the
  deleted `using` above moves the line from row 23 to row 22. So Full paints `)` Move while its
  `(` is unpainted. The Full painting leaves it alone. `renderer`.

Minimal agrees exactly (a lone `)` is dropped as structural punctuation).

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `identical_or_move` in src/diff/text.rs: under Full,
  `shifted_by_an_edit_beside_it` needs `s.start_row == d.start_row`, which the deleted `using` above
  defeats
- Ground truth questionable: no
