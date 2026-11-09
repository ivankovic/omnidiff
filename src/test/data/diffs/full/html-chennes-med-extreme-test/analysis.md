# Accuracy analysis: html-chennes-med-extreme-test

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 458 (320 visible) of 20684 nodes, 20684 graded, N:M floor 0 visible |
| Painting | no painting |
| Ground-truth invariant violations | none |

## The change

A regenerated Doxygen source page: file numbers in link targets change (`a02336` -> `a02333`,
`a02228` -> `a02225`, ...), a copyright year changes, and the five tooltip
`<div class="ttc" id="aa0...">` blocks at lines 180-184 are re-sorted by their new ids. Each
before tooltip has a near-identical after twin (only the file numbers differ): before 180
(`mficlo`) = after 181, before 181 (`mmhcre`) = after 180, before 182 (`mfiope`) = after 184,
before 183 = after 183, before 184 (`str`) = after 182.

## Mapping

All 458 mismatches are inside the four reordered tooltips (before lines 180, 181, 182, 184).
The ground truth pairs the tooltips by POSITION (before 180 -> after 180, ... as
`match_but_not_identical`, updating `mficlo` -> `mmhcre` and so on), except that before 182's
identical `ttdoc` subtree is mapped to its content twin in after 184 (17 entries).

omnidiff pairs the tooltip containers through `StructurallyIdenticalSubtrees` /
`StructurallyIdenticalAncestor` (the `div`s all have the same shape), with descendants inheriting
the pairing, while some identical leaves go to their content twin through `IdenticalHash` /
`IdenticalHashOfAncestor`:

- Before 180 -> after 184 and before 182 -> after 181 (160 visible, 69 invisible; `matcher`):
  neither the position nor the content twin. The shape match picks an arbitrary same-shaped
  tooltip, so names, ids and texts all read as updated, and a few leaves (e.g. `mficlo(fid,`)
  are pulled to the real twin by hash, splitting the tooltip.
- Before 181 -> after 180 and before 184 -> after 182 (160 visible, 69 invisible; `ambiguous`):
  these are the content twins (a move of an almost-identical tooltip); the ground truth's
  positional pairing with updates is the other defensible reading.

## Verdict

- Primary cause: `matcher`
- Other causes: `ambiguous`, `ground-truth`
- Where a fix would go: `StructurallyIdenticalAncestor` / `StructurallyIdenticalSubtrees`
  should prefer content similarity among same-shaped sibling candidates (the fixture's test file
  already flags this mechanism)
- Ground truth questionable: yes: it pairs the re-sorted tooltips by position with Updates but
  maps one `ttdoc` subtree of before 182 to its content twin in after 184, so one tooltip is
  split across two after tooltips; the convention for re-sorted generated blocks is Marko's
  call
