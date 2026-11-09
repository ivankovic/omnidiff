# Accuracy analysis: html-mozilla-firefox-firefox-add-two-elements

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 587 nodes, 587 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 2574: renderer 0, matcher 0 |
| Painting, Full | 2 bytes wrong of 2574: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Two test cases are added after line 34: `<img sizes=automatic>` and `<img sizes="auto ,xyz">`
(after lines 35-36).

## Painting

- Full, 2 bytes, after line 35 cols 0-1, None -> Insert (`renderer`, also with the human
  mapping). The Full painting inserts both new lines including their 2-space indentation.
  omnidiff paints the indentation of the second line (it lies between the two inserted
  elements) but not the leading indentation of the first inserted line. Minimal agrees exactly.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full-mode painting of the leading indentation of an inserted line in
  src/diff/text.rs
- Ground truth questionable: no
