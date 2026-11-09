# Accuracy analysis: ruby-mastodon-mastodon-smal-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 151 nodes, 151 graded, N:M floor 0 visible |
| Painting, Minimal | 6 bytes wrong of 857: renderer 6, matcher 0 |
| Painting, Full | 1 byte wrong of 857: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Line 12: `redirect_back fallback_location: admin_settings_path, notice: ...` becomes
`redirect_back_or_to admin_settings_path, notice: ...` (method renamed, keyword dropped).

## Painting

Both runs are the same with the human mapping, so both are the renderer's (or the painting's).

- Minimal, 6 bytes, after line 12 `_or_to` (columns 19-25): omnidiff paints it Update, as the
  narrowed part of the `redirect_back` -> `redirect_back_or_to` identifier update. The Minimal
  painting has it as Insert and does not record a match for the identifier. The Full painting
  records the same edit as a Match (update) of `redirect_back` -> `redirect_back_or_to`, which is
  what omnidiff renders. So the two paintings label the same suffix differently. `renderer` (the
  label of a narrowed update that is a pure insertion); arguably a painting-style choice.
- Full, 1 byte, before line 12 column 19: the space between `redirect_back` and
  `fallback_location:`. The Full painting deletes it with `fallback_location: `; the rendering
  leaves whitespace before a deleted pair unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: text renderer: label a narrowed Update whose before side is empty as
  Insert (Minimal), and include the separating space in a deleted keyword argument (Full).
- Ground truth questionable: no (the Minimal and Full paintings label the `_or_to` suffix
  differently, Insert vs Update, but both are defensible)
