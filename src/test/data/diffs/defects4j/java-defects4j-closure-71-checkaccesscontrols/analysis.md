# Accuracy analysis: java-defects4j-closure-71-checkaccesscontrols

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 14 (10 visible) of 8291 nodes, 8291 graded, N:M floor 0 visible |
| Painting, Minimal | 39 bytes wrong of 44522: renderer 0, matcher 39 |
| Painting, Full | 39 bytes wrong of 44522: renderer 0, matcher 39 |
| Ground-truth invariant violations | none |

## The change

Line 416: the first conjunct of `isOverride` changes from `t.inGlobalScope()` to
`parent.getJSDocInfo() != null`.

## Mapping

All 14 mismatches are one group on line 416. The ground truth deletes the call `t.inGlobalScope()`
and inserts the new comparison `parent.getJSDocInfo() != null`, call included. omnidiff
(`APTED("large_flat_subtree")`) maps the old call onto the new call nested inside the `!=`: `t` ->
`parent` and `inGlobalScope` -> `getJSDocInfo` as Updates, `.`, `(`, `)` and the `argument_list`
Identical, the `method_invocation` MatchButNotIdentical. 10 visible (the leaf tokens, both sides), 4
invisible (the two `method_invocation`s and two `argument_list`s).

The two calls share only their shape (`x.y()`); both names differ, so the edit-distance fallback
matched a structural twin with no common content. The ground truth's delete+insert is the reasonable
reading. `matcher`.

## Painting

Both runs are matcher-caused (none with the human mapping), same in both presets, 39 bytes:
`t.inGlobalScope()` (17 bytes) is painted Update or left unpainted where the painting deletes it,
and `parent.getJSDocInfo() ` (22 bytes) is painted Update where the painting inserts it. The labels
change inside the runs; the first-byte labels differ between presets (Minimal `None -> Delete`, Full
`Update -> Delete` on the before side). `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED fallback (large_flat_subtree): matching two calls with no shared
  identifier
- Ground truth questionable: no
