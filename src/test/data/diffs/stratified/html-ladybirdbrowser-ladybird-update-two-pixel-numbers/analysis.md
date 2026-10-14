# Accuracy analysis: html-ladybirdbrowser-ladybird-update-two-pixel-numbers

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 598 nodes, 598 graded, N:M floor 0 visible |
| Painting, Minimal | 30 bytes wrong of 1919: renderer 30, matcher 0 |
| Painting, Full | 30 bytes wrong of 1919: renderer 30, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A test's fuzzy-match tolerance on line 3 changes in two places:
`content="maxDifference=0-1;totalPixels=0-9"` becomes `maxDifference=0-2;totalPixels=0-64`.

## Painting

One painting (`Only one solution`), scored under both presets; runs are identical with the human
mapping.

- 30 bytes (15 per side, line 3 `;totalPixels=0-`), Update -> None (`renderer`). The attribute
  value is one leaf mapped as an Update. The painting updates the two numbers separately
  (`1` -> `2`, `9` -> `64`); omnidiff narrows by common prefix and suffix only, so one Update
  span runs from the first changed digit to the last and paints the unchanged
  `;totalPixels=0-` between them.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs, Update narrowing of a string leaf (split into several
  changed spans instead of one prefix/suffix-trimmed span)
- Ground truth questionable: no
