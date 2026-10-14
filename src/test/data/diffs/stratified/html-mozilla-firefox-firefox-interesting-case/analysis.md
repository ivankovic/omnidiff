# Accuracy analysis: html-mozilla-firefox-firefox-interesting-case

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 104 nodes, 104 graded, N:M floor 0 visible |
| Painting, Minimal | 188 bytes wrong of 964: renderer 188, matcher 0 |
| Painting, Full | 188 bytes wrong of 964: renderer 188, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Inside a `<script>` block, `document.featurePolicy` is renamed to `document.permissionsPolicy`
twice (lines 9 and 11).

## Painting

All runs are identical with the human mapping, so they are the renderer's.

- 188 bytes per preset (94 per side), lines 9-11, Update -> None (`renderer`). The script body is
  one `raw_text` leaf, mapped as an Update. omnidiff narrows it to one span from the first
  change to the last, so the unchanged `Policy.getAllowlistForFeature('fullscreen'),`,
  `[wildcard_origin]);` and `assert_true(document.` between the two renames are painted Update.
  The Minimal painting updates only `feature` -> `permissions` twice.
- In Full the painting updates the whole identifier `featurePolicy` -> `permissionsPolicy`, so
  the span starts 6 bytes later on line 9 and the final `Policy` on line 11 (6 per side) is
  None -> Update; same narrowing limitation (`renderer`).

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs, Update narrowing of a large leaf into several changed
  spans (or a token diff of script `raw_text`)
- Ground truth questionable: no
