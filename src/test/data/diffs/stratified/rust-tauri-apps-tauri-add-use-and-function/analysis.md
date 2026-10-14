# Accuracy analysis: rust-tauri-apps-tauri-add-use-and-function

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 251 nodes, 251 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 1508: renderer 0, matcher 0 |
| Painting, Full | 37 bytes wrong of 1508: renderer 37, matcher 0 |
| Ground-truth invariant violations | none |

## The change

From tauri-apps/tauri (`crates/tauri-runtime/src/monitor.rs`, commit 267368fd): `PhysicalRect`
is added to `use crate::dpi::{PhysicalPosition, PhysicalSize};` (line 5), and a documented field
`pub work_area: PhysicalRect<i32, u32>,` to `struct Monitor` (after lines 17-18).

## Painting

The mapping matches the ground truth on every node and Minimal matches its painting exactly. Full
differs by 37 bytes, identical with the human mapping:

- `None -> Move`, `PhysicalPosition, ` on line 5, both sides (36 bytes). The Full painting has a
  `Match` of `PhysicalPosition,` and the space after it to the same text at the same columns; a
  byte-identical match resolves to `Move`, so the painting marks an unmoved item as moved.
  omnidiff leaves it unpainted. `ground-truth`.
- `None -> Move`, 1 byte: the space before `PhysicalSize` (after line 5, column 48), which the
  Full painting includes in the move of ` PhysicalSize`; omnidiff moves only the name.
  `renderer`.

## Verdict

- Primary cause: `ground-truth`
- Other causes: `renderer`
- Where a fix would go: ground truth (Marko's call) for the identity `Match`; the 1-byte space
  is not worth a renderer change
- Ground truth questionable: yes: the Full painting matches the unmoved `PhysicalPosition, ` to
  itself, which resolves to `Move`
