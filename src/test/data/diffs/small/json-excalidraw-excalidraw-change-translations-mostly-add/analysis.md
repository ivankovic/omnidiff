# Accuracy analysis: json-excalidraw-excalidraw-change-translations-mostly-add

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 27 (19 visible) of 13381 nodes, 13381 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

The Norwegian locale file `nb-NO.json` is synced with new keys: in `welcomeScreen.app`,
`center_heading`'s text is emptied and two empty `center_heading_line2/3` keys are added (before
line 560); in `mermaid`, two empty keys are appended after `preview` (before line 614), followed
by new `ttd` and `chat` sections (50 lines).

## Mapping

All 27 mismatches are before-side nodes deleted by the APTED run over the whole file value,
reason `APTED("large_flat_subtree_container")` (`solve_large_flat_subtrees`: after the largest
flat container and the qualified names inside are diffed, APTED runs over the whole item). They
sit in the two sections that changed:

- `welcomeScreen` (before lines 558-570): the `welcomeScreen` and `app` pairs and their objects,
  the `center_heading` pair and its string `"Alle dine data ..."` (-> `""` in the ground truth,
  MatchButNotIdentical), and their punctuation: `:` after each key, `{` of each object, the
  string's two `"`, commas on lines 560, 561 and 563, and the closing `}` on lines 563 and 570.
- `mermaid` (before lines 610-616): the `mermaid` pair and object, its `:` and `{`, the commas on
  lines 611-614 and the closing `}` on line 616.

The ground truth keeps all of these (Identical punctuation, MatchButNotIdentical containers).
omnidiff matches the unchanged content inside the same sections (no mismatch is reported on the
other keys and values), so it keeps children while deleting their parents and the punctuation that
holds them together, and the after-side counterparts appear inserted. 19 visible (punctuation and
the two quotes), 8 invisible (pairs, objects, the string). Why APTED gave up these containers
rather than renaming them was not determined from the evidence. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the whole-item APTED call in `solve_large_flat_subtrees`
  (`large_flat_subtree_container`), or a parent-propagation pass afterwards: a JSON pair or
  object whose children are mostly matched, and the punctuation between them, should be matched
  too.
- Ground truth questionable: no
