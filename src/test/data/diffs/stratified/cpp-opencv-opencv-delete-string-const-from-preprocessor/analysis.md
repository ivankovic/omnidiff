# Accuracy analysis: cpp-opencv-opencv-delete-string-const-from-preprocessor

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 154 nodes, 154 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 1778: renderer 4, matcher 0 |
| Painting, Full | 4 bytes wrong of 1778: renderer 4, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`#define CV_VERSION_STATUS "-pre"` becomes `#define CV_VERSION_STATUS ""` (line 11), a release
build.

## Painting

- **Emptied string labelled Update (4 bytes in each preset, `Update -> Delete`, `renderer`).** The
  string literal is matched as an Update. Omnidiff narrows it to the content and paints `-pre` on
  the before side Update. Nothing is left to paint on the after side. The single painting ("Only one
  solution") labels the removed `-pre` Delete. The runs are the same with the human mapping. This is
  the mirror of the ladybird `Static` case: a narrowed Update whose other side is empty reads as a
  pure removal.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs`. A narrowed Update whose other side is empty would be
  labelled Delete (or Insert).
- Ground truth questionable: no
