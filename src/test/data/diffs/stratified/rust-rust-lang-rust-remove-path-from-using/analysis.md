# Accuracy analysis: rust-rust-lang-rust-remove-path-from-using

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 3401 nodes, 3401 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 17418: renderer 4, matcher 4 |
| Painting, Full | 0 bytes wrong of 17418: renderer 6, matcher 6 |
| Ground-truth invariant violations | none |

## The change

From rust-lang/rust (`compiler/rustc_mir_build/src/builder/expr/stmt.rs`, commit c12ab08c): `use
rustc_span::source_map::Spanned;` -> `use rustc_span::Spanned;` (line 5).

## Mapping

Two visible mismatches, both `::` on line 5, reason `APTED("fast_fallback")`. The ground truth's
tree deletes the first `::` (column 15, after `rustc_span`) and keeps the second (column 27, after
`source_map`). omnidiff keeps the first and deletes the second. The ground truth's own painting
(`Only one solution`) agrees with omnidiff: it deletes `source_map::` (0-based columns 16-28),
i.e. the second `::`. Which of the two separators went is a free choice in principle, but the
tree and the painting of the same file pick different ones. `ground-truth`.

## Painting

omnidiff's rendering matches the painting exactly under both presets (0 real bytes). The census
renderer/matcher shares (Minimal 4 and 4, Full 6 and 6) come from rendering the human tree, whose
`::` choice contradicts its painting, and cancel out. `ground-truth`.

## Verdict

- Primary cause: `ground-truth`
- Other causes: none
- Where a fix would go: ground truth (Marko's call): the tree and the painting delete different
  `::` of `rustc_span::source_map::Spanned`
- Ground truth questionable: yes: the tree deletes the `::` after `rustc_span` while the painting
  deletes the one after `source_map`; omnidiff agrees with the painting
