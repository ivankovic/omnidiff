# Accuracy analysis: kotlin-nextcloud-android-add-param

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 354 nodes, 354 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 1662: renderer 0, matcher 0 |
| Painting, Full | 8 bytes wrong of 1662: renderer 8, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`navigateToAllFiles()` gets `@JvmOverloads` and a parameter `onlyPersonal: Boolean = false`,
and the call `MainApp.showOnlyPersonalFiles(false)` passes `onlyPersonal` instead of `false`.

## Painting

The fixture has one painting (`Only one solution`), scored under both presets. Minimal agrees
exactly.

- Full only, renderer, 8 bytes: the closing `) {` of the parameter list (before line 16, after
  line 17) and the closing `)` of `showOnlyPersonalFiles(...)` (before line 18, after line 19)
  are painted Move on both sides by the Full rendering; the painting leaves them unpainted. The
  runs are identical with the human mapping. The brackets did not move; they shifted because text
  inside the brackets changed length. The matching `(` stays unpainted, so the rendering also
  splits a bracket pair (rule 2 of text_painting_findings.md). That is wrong under Full's own
  terms, not only against a Minimal-style painting. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full-preset move painting in the text renderer: a closing bracket shifted
  by an edit inside its own pair should not be painted as moved.
- Ground truth questionable: no
