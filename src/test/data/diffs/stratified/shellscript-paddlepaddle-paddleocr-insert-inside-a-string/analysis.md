# Accuracy analysis: shellscript-paddlepaddle-paddleocr-insert-inside-a-string

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 32 nodes, 32 graded, N:M floor 0 visible |
| Painting, Minimal | 12 bytes wrong of 598: renderer 12, matcher 0 |
| Painting, Full | 12 bytes wrong of 598: renderer 12, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Three lines of `prepare.sh` (3-5) switch the SDK archive from `paddlex_hps_PaddleOCR-VL_sdk` to
`paddlex_hps_PaddleOCR-VL-1.5_sdk`: `-1.5` is inserted inside a URL, a file name and a path.

## Painting

One painting (`Only one solution`), scored under both presets; the runs are the same in both and
with the human mapping.

- Renderer, 12 bytes: the inserted `-1.5` on after lines 3, 4 and 5 (4 bytes each). Each
  containing word is one node, mapped as an Update, and the rendering narrows the update to the
  added characters but keeps the label Update. The painting labels the added characters Insert.
  The before side has nothing at that point, so the narrowed difference is a pure insertion.
  Same pattern as the `_or_to` suffix in ruby-mastodon-mastodon-smal-change. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Update narrowing in the text renderer: when the narrowed before range is
  empty, label the after range Insert.
- Ground truth questionable: no
