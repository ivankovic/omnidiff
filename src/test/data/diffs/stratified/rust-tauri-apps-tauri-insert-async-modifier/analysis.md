# Accuracy analysis: rust-tauri-apps-tauri-insert-async-modifier

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1794 nodes, 1794 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 5230: renderer 0, matcher 0 |
| Painting, Full | 4 bytes wrong of 5230: renderer 4, matcher 0 |
| Ground-truth invariant violations | none |

## The change

From tauri-apps/tauri (`crates/tauri/src/event/plugin.rs`, commit 3cb73d08): four `#[command]`
functions become `async` (`pub fn listen` -> `pub async fn listen`, and `unlisten`, `emit`,
`emit_to`; after lines 68, 78, 87, 96).

## Painting

The mapping matches the ground truth on every node and Minimal matches its painting exactly. Full
differs by 4 bytes, identical with the human mapping: `None -> Insert`, the space in front of each
inserted `async` (column 3, between `pub` and `async`). The Full painting includes it in the
`Insert`; omnidiff starts the `Insert` at `async`. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs`, Full: include the separating space in front of an
  inserted modifier (4 bytes)
- Ground truth questionable: no
