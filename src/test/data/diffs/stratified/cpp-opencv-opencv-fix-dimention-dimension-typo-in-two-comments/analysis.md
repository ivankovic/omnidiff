# Accuracy analysis: cpp-opencv-opencv-fix-dimention-dimension-typo-in-two-comments

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 45886 nodes, 45886 graded, N:M floor 0 visible |
| Painting, Minimal | 130 bytes wrong of 336630: renderer 130, matcher 0 |
| Painting, Full | 150 bytes wrong of 336630: renderer 150, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The typo `dimentions` -> `dimensions` is fixed three times in two doc comments (lines 1369, 1370 and
1377).

## Painting

All runs are the same with the human mapping.

- **Two fixes merged into one range (both presets, 130 bytes, `Update -> None`, `renderer`).** Lines
  1369 and 1370 are in the same `comment` leaf. Omnidiff paints a single Update from the first fix
  to the second. It covers the unchanged `ions.`, the newline and
  `     * @param newsz Array with new matrix size by all diment` / `dimens` on both sides. The
  paintings mark only the two fixes. The fix on line 1377 sits in a different comment and is painted
  correctly in Minimal.
- **Below the word (Full, 20 bytes, `None -> Update`, `renderer`).** The Full painting paints the
  whole word `dimentions` / `dimensions`. Omnidiff's Full rendering leaves `dimen` and `ions`
  unpainted on line 1377, and `dimen` on line 1369.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: narrowing inside a changed comment leaf in `src/diff/text.rs`. Separate
  edits would stay separate across unchanged text and newlines. Full would use word-level extents.
- Ground truth questionable: no
