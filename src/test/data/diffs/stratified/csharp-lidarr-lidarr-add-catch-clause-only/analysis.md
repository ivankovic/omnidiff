# Accuracy analysis: csharp-lidarr-lidarr-add-catch-clause-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 5986 nodes, 5986 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 28502: renderer 2, matcher 0 |
| Painting, Full | 0 bytes wrong of 28502: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The commit adds a second `catch (HttpException e)` clause, with its own `_logger.Warn` call, in
front of the existing `catch (WebException e)` in `MediaCoverService.cs` (after-side lines 223-226).
Nothing else changes.

## Painting

Minimal only, 2 bytes, identical with the human mapping, so renderer-owned:

- None -> Insert, after lines 224 and 226: the lone `{` and `}` of the inserted catch block. The
  Minimal painting includes them in the Insert entry; omnidiff's Minimal rendering
  (`structural_punctuation: false`) drops a per-row range that is only structural punctuation
  (`is_structural_only` in src/diff/text/render_options.rs). `restore_paired_brackets` does not
  bring them back because both halves of the pair were dropped. `renderer`.

Full agrees exactly.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Minimal structural-punctuation filter in src/diff/text/render_options.rs
  (`ranges_for_options_impl`), for brace-only rows inside a wholly inserted construct
- Ground truth questionable: no
