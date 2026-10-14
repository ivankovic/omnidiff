# Accuracy analysis: rust-rust-lang-rust-update-comment

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 398 nodes, 398 graded, N:M floor 0 visible |
| Painting, Minimal | 3 bytes wrong of 1556: renderer 3, matcher 0 |
| Painting, Full | 70 bytes wrong of 1556: renderer 70, matcher 0 |
| Ground-truth invariant violations | none |

## The change

From rust-lang/rust (`src/tools/miri/tests/fail/branchless-select-i128-pointer.rs`, commit
05834b2d): an expected-error annotation is shortened, `//~^ ERROR: constructing invalid value:
encountered a dangling reference` -> `//~^ ERROR: encountered a dangling reference` (line 17).

## Painting

The mapping matches the ground truth on every node; every disagreement is present with the human
mapping too.

- Both presets, 3 bytes, before line 17: the painting deletes `: constructing invalid value`
  (columns 26-54); omnidiff's narrowed comment update deletes `constructing invalid value:`
  (columns 28-55, after trimming). The deleted text sits between two `: ` separators and can be
  placed on either side of one of them; both readings remove the same words. `ambiguous`.
- Full only, 68 bytes, `None -> Move`: the Full painting marks the kept tail `: encountered a
  dangling reference` as moved (it shifted 28 columns left on the same row); omnidiff narrows the
  update to the deleted words and leaves the shared suffix unpainted, as painting rule 3 (an
  identical line tail stays unpainted) describes. `renderer` (it is the Full painting's generous
  reading against the renderer's narrowing).

## Verdict

- Primary cause: `renderer`
- Other causes: `ambiguous`
- Where a fix would go: `src/diff/text.rs` `intra_node_update_ranges` under Full (paint the
  shifted common suffix of a comment `Move`), if the Full convention is meant to hold; the 3-byte
  placement is a free choice
- Ground truth questionable: no
