# Accuracy analysis: cpp-godotengine-godot-small-but-complex-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 24 (17 visible) of 2333 nodes, 2333 graded, N:M floor 0 visible |
| Painting, Minimal | 422 bytes wrong of 12171: renderer 283, matcher 189 |
| Painting, Full | 407 bytes wrong of 12171: renderer 279, matcher 182 |
| Ground-truth invariant violations | 3 |

## The change

`Camera3DPreview` is rebuilt around a `FoldableContainer` and an `AspectRatioContainer`. In
`_update_sub_viewport_size`,
`sub_viewport->set_size(Node3DEditor::get_camera_viewport_size(camera))` becomes a local
`const Size2i camera_size = Node3DEditor::get_camera_viewport_size(camera);` plus
`centering_container->set_ratio(camera_size.aspect())` (after lines 86-87). The constructor's
initializer list `camera(p_camera), sub_viewport(memnew(SubViewport))` becomes body assignments.
About twenty lines of container setup are added, and the `TextureRect` display code is removed.

## Mapping

- **The extracted call (8 visible, 5 invisible, `matcher`, `APTED("qualified_name")`, one
  `UnresolvedNode`).** The ground truth follows `Node3DEditor::get_camera_viewport_size(camera)`
  from line 83 into the new local at line 86, unchanged. Omnidiff keeps the outer statement shape.
  It maps the before `call_expression` and `(camera)` onto `camera_size.aspect()` / `()` in line 87
  (`MatchButNotIdentical`), and it deletes `Node3DEditor`, `::`, `get_camera_viewport_size` and
  `camera`. The after copy of the call is then new.
- **Initializer list to assignments (3 visible, `matcher`, `APTED("qualified_name")`).** The ground
  truth maps `camera`, `p_camera` and `sub_viewport` in line 87's initializer list to
  `camera = p_camera;` (line 95) and `sub_viewport = memnew(SubViewport);` (line 113). Omnidiff
  deletes them.
- **`add_child(sub_viewport)` (6 visible, 2 invisible, `matcher`, `MovedSubtree`).** The ground
  truth deletes `add_child(sub_viewport);` (line 89) and makes
  `sub_viewport_container->add_child(sub_viewport);` (line 114) new. Omnidiff maps only the argument
  list `(sub_viewport)` across and leaves `add_child` deleted. Mapping the whole call would also be
  defensible. Mapping only its arguments is not. The pair comes from `MovedSubtree`
  (`src/diff/solve_moved_subtrees.rs`): the 4-node `(sub_viewport)` is byte-identical, wholly
  deleted and wholly inserted, and it just meets the pass's size floor (`MIN_MOVE_SUBTREE_SIZE`
  = 4), so the pass moves the argument list on its own out of a call it does not pair.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began printing them.

## Painting

- **In-order line painted as moved (renderer, about 230 bytes in each preset, `None -> Move`).**
  `RenderingServer::get_singleton()->viewport_attach_camera(...)` (before line 88, after line 116)
  is painted `Move` by both paintings. All the code around it is deleted or inserted, so it keeps
  its order relative to the matched code, and omnidiff leaves it unpainted. The run is present with
  the human mapping too.
- **Punctuation and whitespace (renderer, the rest of the renderer bytes).** Minimal paints `:`
  (line 86), `;` (after lines 86, 95, 113), `),` and the `camera(` / `sub_viewport(` heads.
  Omnidiff's Minimal leaves this punctuation unpainted. Full paints indentation and single spaces.
  Omnidiff also paints `->set_` in `set_size`/`set_ratio` as `Move`.
- **Matcher (Minimal 189, Full 182 bytes).** These come from the three mapping groups above. The
  extracted call is painted `Delete`/`Insert` where the painting has `Move`. The initializer names
  are painted `Delete` and the assignments `Insert` where the painting has `Move`. In Full,
  `(sub_viewport)` is painted `Move` where the painting has `Delete`/`Insert`.

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`, `ground-truth`
- Where a fix would go: APTED in the `qualified_name` call
  (`src/diff/solve_syntax_aware_matching.rs`) for extract-to-local and initializer-to-assignment
  rewrites. `MovedSubtree` (`src/diff/solve_moved_subtrees.rs`) moving a bare argument list out
  of an unpaired call. Move display of in-order lines in `src/diff/text.rs`.
- Ground truth questionable: yes. `invariants.txt` reports three violations. The Minimal painting
  paints the shared `set_` prefix as part of the `set_size`/`set_ratio` update (before line 83,
  after line 87). The mapping deletes `false` (line 87) and inserts `true` (line 109) inside
  subtrees it pairs, where its own convention is an Update.
