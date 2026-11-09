# Accuracy analysis: html-twbs-bootstrap-remove-one-line-in-yaml-metadata-preamble

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 628 nodes, 628 graded, N:M floor 0 visible |
| Painting, Minimal | 17 bytes wrong of 2504: renderer 17, matcher 0 |
| Painting, Full | 17 bytes wrong of 2504: renderer 17, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The YAML front matter of a Bootstrap example page loses one line, `include_js: false` (before
line 8). description.md: "Not pure HTML".

## Painting

One painting (`Only one solution`), scored under both presets; the run is identical with the
human mapping.

- 17 bytes, before line 8 `include_js: false`, Update -> Delete (`renderer`, with `parse-error`
  as the reason it is text-level). The front matter parses as one HTML `text` node, mapped as an
  Update by both the ground truth and omnidiff. The narrowed span covers exactly the removed
  line, and nothing on the after side, but omnidiff still labels it Update; the painting deletes
  it. An Update whose narrowed after side is empty could be painted as a Delete.

## Verdict

- Primary cause: `renderer`
- Other causes: `parse-error`
- Where a fix would go: src/diff/text.rs, label of a narrowed Update span with an empty
  counterpart (Delete / Insert instead of Update)
- Ground truth questionable: no
