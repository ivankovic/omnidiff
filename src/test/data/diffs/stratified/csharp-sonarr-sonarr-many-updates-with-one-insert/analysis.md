# Accuracy analysis: csharp-sonarr-sonarr-many-updates-with-one-insert

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 496 nodes, 496 graded, N:M floor 0 visible |
| Painting, Minimal | 134 bytes wrong of 2359: renderer 134, matcher 0 |
| Painting, Full | 31 bytes wrong of 2359: renderer 31, matcher 0 |
| Ground-truth invariant violations | 10 (all rule [16]: the Minimal painting paints whole identifiers on lines 7, 19, 23, 31, 34) |

## The change

`SeriesSpecification` moves to the download-decision API: the interface
`IDecisionEngineSpecification` becomes `IDownloadDecisionEngineSpecification` (line 7), every
`Decision` becomes `DownloadSpecDecision` (lines 19, 23, 31, 34), and the one `Reject` call gains a
leading `DownloadRejectionReason.WrongSeries,` argument (line 31).

## Painting

Mapping is perfect and every run is identical with the human mapping.

- Minimal, 134 bytes, None -> Update on lines 7, 19, 23, 31, 34 (both sides): the Minimal painting
  paints each renamed identifier whole (`IDecisionEngineSpecification` against
  `IDownloadDecisionEngineSpecification`, `Decision` against `DownloadSpecDecision`), and on line 31
  also the identical `.Reject` behind it. omnidiff narrows each Update to the inserted words
  (`Download`, `DownloadSpec`) and leaves the shared rest unpainted. All ten identifier runs are
  exactly the ten invariant [16] violations, which say a Minimal reading should leave everything
  but the differing words alone. `ground-truth`.
- Full, 31 bytes, line 31: the identical `"Wrong series")`, pushed 49 columns right by the renamed
  receiver and the inserted argument, is a Match (Move) in the Full painting; omnidiff leaves it
  unpainted (15 + 15 bytes) through the `shifted_by_an_edit_beside_it` exemption of
  `identical_or_move` (src/diff/text.rs), which applies under Full when the row index is
  unchanged. And the space after the inserted `DownloadRejectionReason.WrongSeries,`, which the
  painting inserts and omnidiff does not paint (1 byte). `renderer`. Note that Full paintings are
  not uniform on displaced identical text: `csharp-sonarr-sonarr-ok-small-edit`'s Full painting
  leaves a `)` displaced by an edit on its row unpainted, this one paints the displaced argument.

## Verdict

- Primary cause: `ground-truth`
- Other causes: `renderer`
- Where a fix would go: the Minimal painting's whole-identifier Updates are ground truth (Marko's
  call); the Full bytes depend on whether Full should paint identical text displaced along its row
  (`identical_or_move`, src/diff/text.rs), on which the Full paintings themselves differ
- Ground truth questionable: yes: all ten Minimal identifier runs are invariant [16] violations (the
  whole identifier painted where only the inserted word differs)
