# Accuracy analysis: rust-data-structure

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 316 nodes, 316 graded, N:M floor 0 visible |
| Painting, Minimal | 61 bytes wrong of 812: renderer 61, matcher 0 |
| Painting, Full | 61 bytes wrong of 812: renderer 61, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Handmade. Adds a derive attribute and an `impl Point` block with `new` and
`distance_from_origin` above `main`, turns three `Point { x: 1, y: 2 }` struct literals into
`Point::new(1, 2)` calls (before lines 8-10, after lines 19-21), and extends the `println!`
format string, wrapping its arguments onto a new line (after lines 25-26).

## Painting

The mapping matches the ground truth everywhere; every disagreement is present with the human
mapping too, so all of it is the renderer's.

- Minimal, 61 bytes, `Move -> None`. The `y` values `2`, `4`, `6` (before lines 8-10, after lines
  19-21, 6 bytes): `y: ` in front of them was removed, so they slid 3 columns left (and down 11
  rows); their neighbours on the row changed, so Minimal's displacement exemption does not apply
  and omnidiff paints `Move`, where the Minimal painting leaves the kept arguments alone (the
  `x` values keep their column and are not painted). The `println!` tail (before line 14 `",
  point.x, point.y`, after lines 25-26 `", ` and the rewrapped `point.x, point.y` with its
  indentation, 55 bytes): the closing quote and arguments moved onto a new row when the call was
  wrapped, and omnidiff paints them `Move` where the Minimal painting does not. `renderer`.
- Full, 61 bytes. The Full painting accounts for the struct-literal-to-call rewrite byte by byte
  (`Delete` of the spaces and braces between the values, `Move` of `1, `/`3, `/`5, `), which
  omnidiff leaves unpainted (`None -> Delete`, `None -> Move`) or paints the trailing `,`
  `Move`; and the `println!` `", `, `);` and the new line's indentation are painted `Move` where
  the Full painting moves only `point.x, point.y`. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` `identical_or_move`: Minimal displacement exemptions for
  call arguments whose neighbours changed and for arguments rewrapped onto a new line
- Ground truth questionable: no
