# Accuracy analysis: rust-tauri-apps-tauri-add-to-list

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 4188 nodes, 4188 graded, N:M floor 0 visible |
| Painting, Minimal | 11 bytes wrong of 15971: renderer 11, matcher 0 |
| Painting, Full | 188 bytes wrong of 15971: renderer 188, matcher 0 |
| Ground-truth invariant violations | none |

## The change

From tauri-apps/tauri (`crates/tauri-bundler/src/bundle.rs`, commit 0aa48fb9): `IosSettings,` is
added to a long `pub use self::{ settings::{ ... } }` list (line 22), and rustfmt reflows the
rest: `Settings,` moves from the end of line 22 to the start of line 23.

## Painting

The mapping matches the ground truth on every node; all disagreements are identical with the
human mapping, so they are the renderer's.

- Minimal, 11 bytes, `Move -> None`: `, Settings,` at the end of before line 22. `Settings` was
  reflowed onto the next row, so it changed row and column and is painted `Move`; only the before
  side is painted; why the after side's `Settings,` (start of after line 23) is not was not traced.
  The painting treats the reflow as no change. `renderer`.
- Full, 188 bytes, `Move -> None`: every item after the insertion point (`MacOsSettings` ...
  `RpmSettings` pushed right on line 22, `Settings, SettingsBuilder` on line 23) is painted
  `Move`. Under Full `paint_displaced_moves` is on, and the reflowed items also changed rows; the
  Full painting marks only the inserted `IosSettings, `. Plus 1 byte, `None -> Insert`: the space
  before `IosSettings` that the Full painting includes. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` `identical_or_move`: items of a list reflowed by an
  insertion (Full's `paint_displaced_moves`, Minimal's row-changing reflow)
- Ground truth questionable: no
