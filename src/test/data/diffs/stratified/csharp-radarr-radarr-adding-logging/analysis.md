# Accuracy analysis: csharp-radarr-radarr-adding-logging

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1362 nodes, 1362 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 5537: renderer 4, matcher 0 |
| Painting, Full | 39 bytes wrong of 5537: renderer 39, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The commit adds logging to the TMDb user-list request generator: a `Logger.Trace` before the
total-pages request (after line 55), an `if (jsonResponse.TotalPages > 1) { Logger.Debug(...) }`
block (after lines 61-64), and it rewrites the per-page `Logger.Debug("Importing TMDb movies from:
{0}", request.Url)` (before line 65) into a `Logger.Debug` with new text and arguments plus a new
`Logger.Trace` that carries `request.Url` (after lines 72-73).

## Painting

Mapping is perfect and every disagreeing run is identical with the human mapping, so all of it is
renderer-owned.

Minimal, 4 bytes:

- None -> Insert, after lines 62 and 64: the lone `{` and `}` of the inserted if block, dropped by
  the Minimal structural-punctuation filter (`is_structural_only`,
  src/diff/text/render_options.rs); the painting includes them. 2 bytes. `renderer`.
- None -> Update, before line 65 / after line 73: the closing `}` of the format placeholder at the
  end of the updated string (`{0}` -> `{2}`). The painting's Update covers the whole string
  content; omnidiff's narrowed Update stops before the trailing `}`, which both strings share. 2
  bytes. `renderer`.

Full, 39 bytes:

- None -> Move/Update on before line 65 and after line 73: the identical `("` after `Debug`
  (2 + 2), `}", ` after the string (the `}` of the narrowing above plus the identical `", `), and
  ` request.Url);` (17 on the before side; 4 + 14 on the after side, where ` request.Url);` now
  stands 36 columns further right behind the inserted arguments). The Full painting records each
  of these identical pieces as a matched Move (`("`, `", ` and ` request.Url);` are Match entries
  of their own). omnidiff leaves identical text that only shifted rows, or shifted along its row
  because of inserted text, unpainted. `renderer` (Full painting holds displaced identical text
  as Move).

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Minimal structural-punctuation filter (src/diff/text/render_options.rs),
  the Update narrowing that drops the shared trailing `}` (src/diff/text.rs), and whether Full
  paints identical text displaced by an insertion
- Ground truth questionable: no
