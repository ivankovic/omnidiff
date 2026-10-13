# Accuracy analysis: html-mozilla-firefox-firefox-remove-li-around-button

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (0 visible) of 26816 nodes, 26816 graded, N:M floor 0 visible |
| Painting | no painting |
| Ground-truth invariant violations | none |

## The change

In the pdf.js viewer, the `<li>` wrappers around the `<button>`s of two menus
(`viewsManagerSelectorOptions`, before lines 83-102, and `viewsManagerStatusActionOptions`,
before lines 158-177) are removed; the buttons become direct children of `<menu>` and are
re-indented.

## Mapping

- Two invisible `<menu>` containers (`matcher`, `UnresolvedNode`). The ground truth maps the
  before `<menu id="viewsManagerSelectorOptions" ...>` (line 82 col 25) and
  `<menu id="viewsManagerStatusActionOptions" ...>` (line 157 col 27) to their after
  counterparts as MatchButNotIdentical. omnidiff leaves both unresolved (deleted). No visible
  node is mismatched, so the buttons, their attributes and the menu tags are mapped as
  expected; only the container nodes, whose children changed from `<li>`s to `<button>`s, are
  not paired. A reader sees no difference.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: bottom-up propagation for a parent whose children were unwrapped (its
  start/end tags and grandchildren map); no visible effect today
- Ground truth questionable: no
