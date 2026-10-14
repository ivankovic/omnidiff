# Accuracy analysis: kotlin-nextcloud-android-remove-function

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 373 nodes, 373 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 3159: renderer 0, matcher 0 |
| Painting, Full | 8 bytes wrong of 3159: renderer 8, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`NotificationUtils.kt` removes the last function of the object,
`@JvmStatic fun createUploadNotificationTag(...)` (before lines 26-27), and the blank line before
it.

## Painting

The fixture has only a `Minimal` painting; the Full preset is scored against it too. Minimal
agrees exactly.

- Full only, 8 bytes: the 4-space indentation of before lines 26 and 27. The Full rendering paints
  the indentation of a deleted line as Delete, which is what Full is for; the Minimal painting
  leaves indentation unpainted by convention. The runs are identical with the human mapping.
  `grading-artifact`.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: none
- Where a fix would go: none (a Full painting of this fixture would agree).
- Ground truth questionable: no
