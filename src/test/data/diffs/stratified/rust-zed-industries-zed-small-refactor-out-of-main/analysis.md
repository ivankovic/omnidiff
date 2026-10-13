# Accuracy analysis: rust-zed-industries-zed-small-refactor-out-of-main

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 18 (12 visible) of 1168 nodes, 1168 graded, N:M floor 0 visible |
| Painting, Minimal | 531 bytes wrong of 3204: renderer 496, matcher 35 |
| Painting, Full | 38 bytes wrong of 3204: renderer 736, matcher 752 |
| Ground-truth invariant violations | none |

## The change

zed `crates/gpui/examples/tree.rs`: a `#![cfg_attr(target_family = "wasm", no_main)]` line is
added at the top, the body of `fn main()` is moved into a new `fn run_example()` (after lines
33-45), and two new entry points follow: a non-wasm `fn main() { run_example(); }` (after
lines 47-50) and a wasm `pub fn start()`. Every old line shifts down one row.

## Mapping

- **`fn main() {` / `}` (before lines 32, 44), 12 visible + 6 invisible.** The ground truth
  keeps `main` as `main` (before line 32 -> after line 48, its block -> the new
  `{ run_example(); }`) and moves the old body into `run_example`, whose header and braces are
  new. omnidiff reads it as a rename: `main` -> `run_example` (Update), the block byte-identical
  (`IdenticalHash`), the function pair via `StructurallyIdenticalSubtrees` /
  `StructurallyIdenticalAncestor`, and the new `main` inserted. omnidiff's reading is the cheaper
  edit; the ground truth's is what the commit did (the fixture name says "refactor out of main")
  and its paintings agree with it. Both are defensible. `ambiguous`.

## Painting

- **Minimal, None -> Move, 496 bytes, the body (before lines 33-43, after lines 34-44).** The
  Minimal painting marks the moved body Move; omnidiff leaves it unpainted, and so does rendering
  the human mapping: Minimal does not paint a block whose only displacement is the one-row shift
  that the whole file has. `renderer` (the painting's choice is consistent with its tree).
- **Minimal, 35 bytes, matcher (the rename reading).** `main` painted Update (4), the
  `fn run_example() {` header and `}` left unpainted where the painting inserts them (19), and
  the new `fn main() {` / `}` painted Insert where the painting has them unpainted (12).
  `ambiguous`.
- **Full, 38 bytes.** 19 matcher-only (the `run_example` header and `}`; `ambiguous`), 19 also
  with the human mapping (`fn main` on before line 32 and `fn main() {` / `}` on after lines 48,
  50, which the Full painting marks Move from the old header; `renderer`). With its own mapping
  omnidiff's Full rendering paints the body Move and agrees with the painting there; rendering
  the human mapping does not (717 bytes more wrong), which is why `renderer_bytes` (736) exceeds
  `real_bytes` here.

## Verdict

- Primary cause: `renderer`
- Other causes: `ambiguous`
- Where a fix would go: `diff::text` Minimal Move painting of a body moved into another
  function when its only offset is a file-wide row shift; the main/run_example choice is none
  (ambiguous)
- Ground truth questionable: no
