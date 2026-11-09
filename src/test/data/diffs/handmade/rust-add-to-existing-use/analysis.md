# Accuracy analysis: rust-add-to-existing-use

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 69 nodes, 69 graded, N:M floor 0 visible |
| Painting, Minimal | 16 bytes wrong of 143: renderer 16, matcher 0 |
| Painting, Full | 4 bytes wrong of 143: renderer 4, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Handmade. `use std::fmt::Display;` becomes a braced, wrapped list `use std::fmt::{` / `Display,
Debug` / `};`, adding `Debug`.

## Painting

The mapping matches the ground truth everywhere. Every disagreement is present with the human
mapping too, so all of it is the renderer's.

- Minimal, 16 bytes. `Move -> None` on `Display` (before line 1, after line 2, 7 bytes each):
  `Display` changed row and column when the list was wrapped, and Minimal's displacement
  exemptions only cover a node slid along its own row, so it paints `Move`; the Minimal painting
  treats it as unchanged (the Full painting does paint it `Move`). `None -> Insert` on the new `{`
  (after line 1) and `}` (after line 3), 1 byte each: Minimal drops lone structural punctuation,
  the painting inserts both braces. `renderer`.
- Full, 4 bytes. `Move -> None` on the `;` (before line 1, after line 3): it moved two rows down
  behind the new `}` and Full paints it moved where the painting leaves it alone; `None -> Insert`
  on the 2-byte indentation in front of `Display` on after line 2, which the painting inserts.
  `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` `identical_or_move` (a use-list item rewrapped onto
  its own line, under Minimal) and the Minimal structural-punctuation filter for inserted braces
  that open a new list
- Ground truth questionable: no
