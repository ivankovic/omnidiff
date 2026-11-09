# Accuracy analysis: rust-tauri-apps-tauri-rename-mod

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 38 nodes, 38 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 340: renderer 0, matcher 0 |
| Painting, Full | 6 bytes wrong of 340: renderer 6, matcher 0 |
| Ground-truth invariant violations | 2 |

## The change

From tauri-apps/tauri (`crates/tauri-cli/src/migrate/migrations/mod.rs`, commit bda83041): `pub
mod v2_rc;` -> `pub mod v2_beta;` (line 6).

## Painting

The mapping matches the ground truth on every node and Minimal matches its painting exactly.
Full differs by 6 bytes, identical with the human mapping: `Update -> Move` on `v2_` (line 6,
both sides). Under Full omnidiff highlights the renamed identifier whole
(`whole_identifier_updates`), `v2_rc` -> `v2_beta` as one `Update`. The Full painting splits it into
a `Match` of `rc` -> `beta` and a separate `Match` of the unchanged `v2_` -> `v2_`, which, being
byte-identical in the same place, resolves to `Move`. Invariant [16] flags both sides (a Full
painting marks a renamed identifier entire). `ground-truth`.

## Verdict

- Primary cause: `ground-truth`
- Other causes: none
- Where a fix would go: ground truth (Marko's call): the Full painting's split of `v2_rc` ->
  `v2_beta`
- Ground truth questionable: yes: the Full painting matches the kept prefix `v2_` to itself as a
  separate entry (resolving to `Move`) instead of updating the whole name (invariant [16], 2
  violations)
