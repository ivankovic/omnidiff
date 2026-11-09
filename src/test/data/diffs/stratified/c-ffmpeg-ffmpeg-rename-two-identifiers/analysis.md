# Accuracy analysis: c-ffmpeg-ffmpeg-rename-two-identifiers

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 9882 nodes, 9882 graded, N:M floor 0 visible |
| Painting, Minimal | 8 bytes wrong of 40800: renderer 8, matcher 0 |
| Painting, Full | 2 bytes wrong of 40800: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

FFmpeg `libavformat/tee.c`: the options array `options` is renamed `tee_options`, at its
definition (line 68) and its one use `.option = options,` (line 79), which is also re-aligned to
`.option     = tee_options,`.

## Painting

All runs are the same with the human mapping.

- **Minimal, Update -> Insert, 8 bytes:** the added `tee_` prefix on after lines 68 and 79.
  omnidiff renders the rename as an Update narrowed to the added prefix; the Minimal painting
  inserts the prefix. `renderer`.
- **Full, Move -> None, 2 bytes:** the `=` on line 79, pushed right by the re-alignment
  whitespace. Full paints it Move; the painting leaves it (the Full painting matches the whole
  identifiers instead). `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text`: Update narrowing of a prefix-extended identifier (Minimal),
  Full displacement of a token moved only by whitespace on its own line
- Ground truth questionable: no
