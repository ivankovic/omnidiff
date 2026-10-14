# Accuracy analysis: lua-awesomewm-awesome-align-to-halign-2

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 566 nodes, 566 graded, N:M floor 0 visible |
| Painting, Minimal | 1 bytes wrong of 1758: renderer 1, matcher 0 |
| Painting, Full | 0 bytes wrong of 1758: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

An awesome WM test example renames a table field: `align  = "center",` becomes `halign = "center",`
on line 24 (the alignment padding absorbs the extra character).

## Painting

All runs are the same with the human mapping.

- Minimal, 1 byte, Update -> Insert: the `h` in front of `align` (after line 24). omnidiff maps the
  field name Update and, under Minimal, narrows the update to the inserted `h`, which it labels
  Update. The Minimal painting labels it Insert. Under Full both omnidiff and the painting mark the
  whole identifier `align` -> `halign` Update, and agree. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: narrow Update rendering in src/diff/text.rs: when one side of the narrowed
  span is empty, label it Insert/Delete.
- Ground truth questionable: no
