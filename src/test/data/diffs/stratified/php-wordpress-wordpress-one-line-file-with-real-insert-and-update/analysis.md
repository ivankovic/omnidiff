# Accuracy analysis: php-wordpress-wordpress-one-line-file-with-real-insert-and-update

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 390 nodes, 390 graded, N:M floor 0 visible |
| Painting, Minimal | 8 bytes wrong of 934: renderer 8, matcher 0 |
| Painting, Full | 350 bytes wrong of 934: renderer 350, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A generated one-line asset file: `'wp-url'` is appended to the `'dependencies'` array and the
`'version'` hash changes. The mapping is perfect; every painting run is the same with the human
mapping.

## Painting

### Minimal, 8 bytes, Move -> None (renderer)

`=> '` in front of the updated `'version'` hash, 4 bytes on each side. The Minimal painting
updates only the hash inside the quotes; omnidiff also paints the `=>` and the opening quote as
Move. Other tokens shifted along the same row by the insertions are not painted Move in Minimal,
so it is specific to the neighbourhood of the updated string; the rule that fires was not
determined. Same with the human mapping. `renderer`.

### Full, 350 bytes

- `('react', ... 'wp-theme'` (173 bytes each side, columns 42-215), None -> Move. The Full
  painting pairs this run as a byte-identical Match, which grades as Move, although it keeps its
  row and its column on both sides. omnidiff leaves it unpainted. `ground-truth` (Marko's call).
- The closing `')` of the version value (2 bytes each side), None -> Move: shifted 10 columns by
  the insertion and painted Move by the Full painting; omnidiff paints the rest of the shifted
  tail Move in Full (no run there) but not this `')`. `renderer`.

## Verdict

- Primary cause: `ground-truth`
- Other causes: `renderer`
- Where a fix would go: the Full painting's Match over the unshifted prefix (Marko's call);
  `diff::text` for the `=> '` Move in Minimal and the closing `')` in Full.
- Ground truth questionable: yes: the Full painting marks `('react', ... 'wp-theme'`, at the same
  row and column on both sides, as a byte-identical Match (Move).
