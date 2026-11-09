# Accuracy analysis: kotlin-nextcloud-android-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 209 nodes, 209 graded, N:M floor 0 visible |
| Painting, Minimal | 30 bytes wrong of 1436: renderer 30, matcher 0 |
| Painting, Full | 100 bytes wrong of 1436: renderer 100, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`ActivitiesRepository.getActivities(lastGiven: Long, callback: LoadActivitiesCallback)` gets a
new first parameter `lifecycleScope: CoroutineScope` (before line 21, after line 19); an import
is added and a KDoc comment removed.

## Painting

The mapping is perfect and every run is the same with the human mapping: all renderer.

- Minimal, 30 bytes: `lastGiven: Long` on both sides is painted Move by omnidiff and left
  unpainted by the painting. The parameter did not move; it is shifted one position by the new
  parameter in front of it.
- Full, 100 bytes: the same, extended over `lastGiven: Long, callback: LoadActivitiesCallback)`
  on both sides (the second parameter, the comma and the closing bracket are also painted Move;
  the `(` is not).
- Same pattern as kotlin-nextcloud-android-add-param and ruby-mastodon-mastodon-add-func-and-
  attribute: a sibling shifted by an insertion in front of it is rendered as a move. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: move painting in the text renderer: parameters whose order relative to
  their mapped siblings is unchanged should not be painted as moved when a new sibling is
  inserted in front.
- Ground truth questionable: no
