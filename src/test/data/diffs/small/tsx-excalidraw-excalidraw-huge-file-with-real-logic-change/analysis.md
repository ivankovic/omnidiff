# Accuracy analysis: tsx-excalidraw-excalidraw-huge-file-with-real-logic-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 228 (152 visible) of 129245 nodes, 129258 graded, N:M floor 13 visible |
| Painting | no painting on file |
| Ground-truth invariant violations | none |

## The change

A real logic change in excalidraw's 9.3k-line `App.tsx`: the global `lastPointerUp(event)` callback
is replaced by a `missingPointerEventCleanupEmitter` field (after lines 565-567), a new
`lastPointerMoveEvent` field, an eraser-button branch in `handleCanvasPointerDown` (after lines
5316-5364), `maybeCleanupAfterMissingPointerUp` rewritten from a method into an arrow-function field
(before lines 5549-5558 / after 5610-5618), and the eraser code switched from
`this.lastPointerDownEvent!`/`this.lastPointerUpEvent!` to local `pointerStart`/`pointerEnd` (before
lines 7696-7708). Marko's description.md: "Requiers N:M mapping".

## Mapping

All mismatches sit inside the changed members of the `App` class body; unchanged members match.

- Skeletons of changed class members deleted, 136 visible + 64 invisible, reason
  `APTED("large_flat_subtree")`, `matcher`. The ground truth keeps each changed member and maps
  its unchanged tokens identically; omnidiff deletes, on the before side, the `private`, the name,
  the `(`/`)`/`{`/`}`, `=`/`=>`, separating commas and the like of `addEventListeners` (before
  line 2413), `handleCanvasPointerMove` (4615), `handleCanvasPointerDown` (5203),
  `maybeCleanupAfterMissingPointerUp` (5549), `handleDraggingScrollBar` (5741),
  `onPointerUpFromPointerDownHandler` (7207) and the statements around them (5268, 5308, 5438,
  5762, 7696-7726, 7985), and inserts the after counterparts; their byte-identical inner content
  is not in the mismatch list, so it ends up matched. The class body is a flat container (at
  least 50 children), so `solve_large_flat_subtrees` resolves it through the Myers fast path
  (`resolve_child_sequence`, src/diff/apted/common/myers.rs). Members whose hash changed are
  leftovers. `anchor_leftovers_by_member_name` should pair them by name, but
  `nodes::member_identity_name` only knows Java, C/C++ and Kotlin members, not TypeScript
  `method_definition`/`public_field_definition` (checked in src/diff/nodes.rs). The leftover
  counts are unequal (the after side adds two fields, and one method became a field), so no
  positional pairing. The pool is far above `FLAT_UNMATCHED_RECURSE_MAX_TOTAL_SIZE` (2000 nodes;
  `handleCanvasPointerDown` alone is hundreds of lines). That leaves the atomic delete/insert
  branch, which fits the Delete-with-`large_flat_subtree` reason on every skeleton token. The
  branch taken was inferred from the code and the reasons, not traced at run time.
- N:M floor, 13 visible + about 8 invisible, `nm-floor`: the ground truth's all-to-all groups,
  `lastPointerUp` (2 before <-> 1 after, before lines 5552/5556, after 5616),
  `this.lastPointerDownEvent` (2 <-> 1, before 7698-7699), `this.lastPointerUpEvent` (4 <-> 1,
  before 7700-7708), and a 1-pair multi-map group on the `;` of before line 5556. A one-to-one
  mapping cannot satisfy them.
- 3 visible group members beyond the floor: inside the deleted skeletons above, omnidiff maps no
  member of some groups at all (Delete, `APTED("large_flat_subtree")`). `matcher`.
- 4 invisible, reason `MovedSubtree`: omnidiff maps before `this.lastPointerDownEvent` (line 7698)
  and `this.lastPointerUpEvent` (line 7700) to the new `const pointerStart =
  this.lastPointerDownEvent;` / `const pointerEnd = this.lastPointerUpEvent || …` (after lines
  7757-7758), which the ground truth inserts while mapping the old uses into the
  `pointerStart`/`pointerEnd` group. The expressions really did move into those declarations, so
  both readings are defensible. `ambiguous`.

## Verdict

- Primary cause: `matcher`
- Other causes: `nm-floor`, `ambiguous`
- Where a fix would go: `nodes::member_identity_name` (src/diff/nodes.rs) has no TypeScript/TSX
  case, so `anchor_leftovers_by_member_name` in the Myers flat path (src/diff/apted/common/myers.rs)
  cannot pair changed class members by name before the leftovers fall to atomic delete/insert
- Ground truth questionable: no
