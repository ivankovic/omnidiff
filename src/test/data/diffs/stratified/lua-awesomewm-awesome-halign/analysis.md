# Accuracy analysis: lua-awesomewm-awesome-halign

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 366 nodes, 366 graded, N:M floor 0 visible |
| Painting, Minimal | 1 byte wrong of 1652: renderer 1, matcher 0 |
| Painting, Full | 0 bytes wrong of 1652: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In the example `tests/examples/wibox/widget/progressbar/text.lua`, the commit renames one table
field from `align  = "center"` to `halign = "center"` (line 22).

## Painting

- **Update -> Insert, after line 22, 1 byte (`h`), Minimal, `renderer`.** The Minimal painting
  paints only the added `h` of `halign`, as an Insert. omnidiff maps the identifier as an update
  (as do the ground truth and the Full painting, which paints `align` -> `halign` as Match). In
  Minimal it narrows the update to the one new byte, but labels it Update, not Insert. The run is
  the same with the human mapping, so the cause is the renderer. When an update only adds
  characters, the Minimal painting calls the added characters an Insert.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Minimal narrowing of an Update in src/diff/text.rs: label the remainder
  Insert when the narrowed update is a pure insertion
- Ground truth questionable: no
