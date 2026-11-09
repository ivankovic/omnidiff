# Accuracy analysis: html-prettier-prettier-not-pure-html-includes-yaml-as-well

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 58 nodes, 58 graded, N:M floor 0 visible |
| Painting, Minimal | 71 bytes wrong of 181: renderer 71, matcher 0 |
| Painting, Full | 75 bytes wrong of 181: renderer 75, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A prettier test file has a YAML front matter between `---` lines before `<html>`. Its content
`    invalid:` / `invalid:` (lines 2-3) is replaced by four new lines
(`[`, `    ! should NOT format !`, `because: `, `this is INVALID yaml`), and two blank lines
are dropped.

## Painting

All runs are identical with the human mapping, so they are the renderer's.

- 71 bytes Minimal, 75 Full, lines 2-3 before and 2-5 after (`renderer`, with `parse-error` as
  the reason it is text-level). The front matter is not HTML, so tree-sitter makes the whole
  block, `---` to `---`, one `text` node; both the ground truth and omnidiff map it as an Update.
  omnidiff narrows it by common prefix/suffix and paints one Update span over everything in
  between. The paintings delete the old lines and insert the new ones (Minimal leaves the
  4-space indentation of `! should NOT format !` unpainted, Full paints it). So labels differ
  (Update vs Delete/Insert) on all bytes, and in Minimal the indentation inside the span is
  painted where the painting leaves it.

## Verdict

- Primary cause: `renderer`
- Other causes: `parse-error`
- Where a fix would go: src/diff/text.rs, rendering of a large updated text leaf (line diff,
  Delete/Insert for replaced lines)
- Ground truth questionable: no
