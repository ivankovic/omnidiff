# Accuracy analysis: rust-tauri-api-build-2

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 909 nodes, 909 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 3550: renderer 0, matcher 0 |
| Painting, Full | 33 bytes wrong of 3550: renderer 33, matcher 0 |
| Ground-truth invariant violations | none |

## The change

From tauri-apps/tauri (`examples/api/src-tauri/build.rs`, commit cde0ff77), kept as a performance
fixture: `std::env::var("__TAURI_WORKSPACE__").map_or(false, |v| v == "true")` ->
`.is_ok_and(|v| v == "true")` (line 29).

## Painting

The mapping matches the ground truth on every node and Minimal matches its painting exactly. Full
differs by 33 bytes, identical with the human mapping:

- `None -> Move` on the closure `|v| v == "true")` (both sides, 16 bytes each) and `None ->
  Delete` on the space after `false,` (1 byte). The Full painting deletes `false, ` and marks the
  `(` and the closure as moved, since `map_or` -> `is_ok_and` shifted them. omnidiff paints the
  `(` `Move` but not the closure: it is the unchanged tail of the row, so
  `node_untouched_on_its_row` (`in_suffix`) exempts it as displaced even under Full;
  `paint_displaced_moves` only lifts the row-index condition, not this one. The space is left
  unpainted between the deleted `false,` and the closure. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` `shifted_by_an_edit_beside_it` /
  `node_untouched_on_its_row` under Full (a displaced row tail that the Full painting moves)
- Ground truth questionable: no
