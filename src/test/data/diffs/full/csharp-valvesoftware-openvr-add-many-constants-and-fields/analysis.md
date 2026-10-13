# Accuracy analysis: csharp-valvesoftware-openvr-add-many-constants-and-fields

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 106 (58 visible) of 146334 nodes, 146334 graded, N:M floor 0 visible |
| Painting | no painting on file |
| Ground-truth invariant violations | none |

## The change

An OpenVR SDK header update (`headers/openvr_api.cs`, 8.3k lines): 21 hunks that only add lines,
mostly new enum constants and struct fields. Among them both `VREvent_Mouse_t` (before lines
6137-6142) and `VREvent_Scroll_t` (before lines 6143-6149) gain a `public uint cursorIndex;` field
(after lines 6164 and 6171).

## Mapping

All 106 mismatches (58 visible) are one cluster, the two adjacent event structs before lines
6137-6149 / after lines 6159-6172; the rest of the 146k-node file matches the ground truth.

- The two structs are swapped, `matcher`. After `VREvent_Mouse_t` gains `cursorIndex` it has four
  fields (`float x; float y; uint button; uint cursorIndex;`), the same shape as before
  `VREvent_Scroll_t` (`float xdelta; float ydelta; uint unused; float viewportscale;`), only leaf
  values differ. `solve_hash_descent` pairs these two as `StructurallyIdenticalSubtrees`
  (MatchButNotIdentical) and maps all their leaves with `StructurallyIdenticalAncestor`: the struct
  name `VREvent_Scroll_t` -> `VREvent_Mouse_t`, `xdelta` -> `x`, `ydelta` -> `y`, `unused` ->
  `button`, `float viewportscale` -> `uint cursorIndex` (52 mismatches). The before
  `VREvent_Mouse_t` is then left to `APTED("qualified_name")`, which maps it onto the after
  `VREvent_Scroll_t` (45 mismatches, mostly Identical-op tokens such as the attribute and `public
  float`). The after `cursorIndex` field of `VREvent_Mouse_t`, which the ground truth inserts, is
  mapped to `viewportscale` (8 mismatches, `StructurallyIdenticalAncestor`: it is the fourth field
  of the same shape match). A structural-shape match is accepted across declarations whose names
  differ although each name has an exact counterpart (`VREvent_Mouse_t` and `VREvent_Scroll_t` exist
  on both sides, in the same order). The ground truth is the obvious reading: each struct keeps its
  name and gains one field.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the `StructurallyIdenticalSubtrees` pairing in
  src/diff/solve_hash_descent.rs, which accepts a shape match between two differently named structs
  although each name has an exact counterpart
- Ground truth questionable: no
