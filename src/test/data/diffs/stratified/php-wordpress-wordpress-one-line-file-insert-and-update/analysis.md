# Accuracy analysis: php-wordpress-wordpress-one-line-file-insert-and-update

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 292 nodes, 292 graded, N:M floor 0 visible |
| Painting, Minimal | 8 bytes wrong of 711: renderer 8, matcher 0 |
| Painting, Full | 99 bytes wrong of 711: renderer 99, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A generated one-line asset file: `wp-a11y`, `wp-notices` and `wp-theme` are inserted into the
`'dependencies'` array and the `'version'` hash changes. The mapping is perfect; every painting
run is the same with the human mapping.

## Painting

### Minimal, 8 bytes, Move -> None (renderer)

`=> '` in front of the updated `'version'` hash, 4 bytes on each side. The Minimal painting
updates only the hash inside the quotes; omnidiff also paints the `=>` and the opening quote as
Move. Other tokens shifted along the same row by the insertions are not painted Move in Minimal,
so it is specific to the neighbourhood of the updated string; the rule that fires was not
determined. Same with the human mapping. `renderer`.

### Full, 99 bytes

- `('react', 'react-dom', 'react-jsx-runtime', ` (44 bytes each side, before and after columns
  42-86), None -> Move. The Full painting pairs this run as a byte-identical Match, which grades
  as Move, although it keeps its row and its column (nothing in front of it changed). omnidiff
  leaves it unpainted. Calling text that did not move a Move looks questionable;
  `ground-truth` (Marko's call).
- Single spaces between shifted items (2 before, 3 after) and the closing `')` (2 each side),
  None -> Move, and the spaces in front of `'wp-notices'` and `'wp-theme'` (2), None -> Insert.
  omnidiff does paint the shifted items themselves Move in Full, but whitespace has no node and
  the closing `')` is left unpainted. `renderer`.

## Verdict

- Primary cause: `ground-truth`
- Other causes: `renderer`
- Where a fix would go: the Full painting's Match over the unshifted prefix (Marko's call);
  `diff::text` for the `=> '` Move in Minimal and for separating whitespace / closing `')` in Full.
- Ground truth questionable: yes: the Full painting marks
  `('react', 'react-dom', 'react-jsx-runtime', `, at the same row and column on both sides, as a
  byte-identical Match (Move).
