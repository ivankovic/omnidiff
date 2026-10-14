# Accuracy analysis: html-gohugoio-hugo-template-not-pure-html-2

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 4 nodes, 4 graded, N:M floor 0 visible |
| Painting, Minimal | 160 bytes wrong of 206: renderer 160, matcher 0 |
| Painting, Full | 160 bytes wrong of 206: renderer 160, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A Hugo shortcode template gets a new first line, `{{- warnf "The %q shortcode was deprecated ..."
.Name .Position -}}`, above the unchanged `{{- $noop := .Inner -}}`. description.md: template use
gives suboptimal tree-sitter results.

## Painting

One painting (`Only one solution`), scored under both presets; runs are identical with the human
mapping.

- 160 bytes, after lines 1-2 (`renderer`, with `parse-error` as the reason it is text-level).
  The whole file parses as a single HTML `text` node, so both the ground truth and omnidiff map
  it as an Update. The renderer narrows the Update by common prefix and suffix: both texts begin
  with `{{- `, so it keeps the new line's `{{- ` unpainted and paints from `warnf` through the
  `{{-` of line 2 as Update. The painting inserts exactly line 1 (157 bytes) and leaves line 2
  unpainted. A line-aware (or Myers) text diff inside the updated leaf would find the insertion.

## Verdict

- Primary cause: `renderer`
- Other causes: `parse-error`
- Where a fix would go: src/diff/text.rs, Update narrowing of a large text leaf (diff the text
  instead of trimming common prefix/suffix)
- Ground truth questionable: no
