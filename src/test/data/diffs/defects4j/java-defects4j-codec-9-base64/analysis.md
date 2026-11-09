# Accuracy analysis: java-defects4j-codec-9-base64

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 9843 nodes, 9843 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 81774: renderer 0, matcher 0 |
| Painting, Full | 3 bytes wrong of 81774: renderer 3, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Line 827: the chunk-size argument `MIME_CHUNK_SIZE` becomes `isChunked ? MIME_CHUNK_SIZE : 0`, so
the encoded length ignores chunking when it is off.

## Painting

Minimal matches exactly. Full, 3 bytes, renderer (same with the human mapping), line 827: the Full
painting includes the space before `MIME_CHUNK_SIZE` in its Move on both sides (`None -> Move`, 1
byte each side) and paints the space between `MIME_CHUNK_SIZE` and `: 0` as Insert (`None ->
Insert`, 1 byte). omnidiff's Full rendering leaves those single spaces unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs Full gap painting of separator spaces next to a moved or
  inserted token
- Ground truth questionable: no
