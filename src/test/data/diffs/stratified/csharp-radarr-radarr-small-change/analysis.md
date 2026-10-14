# Accuracy analysis: csharp-radarr-radarr-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1944 nodes, 1944 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 6993: renderer 0, matcher 0 |
| Painting, Full | 76 bytes wrong of 6993: renderer 76, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`UTorrentSettings` switches from implementing `IProviderConfig` to deriving from
`DownloadClientSettingsBase<UTorrentSettings>` (line 20), drops the now unused `using
NzbDrone.Core.ThingiProvider;` (before line 4), shortens `new UTorrentSettingsValidator()` to
target-typed `new ()` (line 22) and marks `Validate()` `override` (line 68).

## Painting

One painting (`Only one solution`) scores both presets. Minimal agrees exactly. Full, 76 bytes,
identical with the human mapping, so renderer-owned:

- Move -> None, before line 22 / after line 21: `();` of `new UTorrentSettingsValidator();`,
  which slides left when the type name is deleted (3 + 3 bytes).
- Move -> None, before line 68 / after line 67: `NzbDroneValidationResult Validate()`, which slides
  right behind the inserted `override` (35 + 35 bytes).

Both are identical nodes pushed sideways by an edit beside them on their own line. The
`shifted_by_an_edit_beside_it` exemption in `identical_or_move` (src/diff/text.rs) is meant for
exactly that, but under Full (`paint_displaced_moves`) it also requires the same row index, and the
deleted `using` above shifts every row by one, so Full paints them Move. The painting leaves them
unpainted, as rule 3 of text_painting_findings.md describes. `renderer` (Full's own exemption misses
because of an unrelated row shift).

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `identical_or_move` in src/diff/text.rs: under Full,
  `shifted_by_an_edit_beside_it` needs `s.start_row == d.start_row`, which an unrelated deletion
  above defeats
- Ground truth questionable: no
