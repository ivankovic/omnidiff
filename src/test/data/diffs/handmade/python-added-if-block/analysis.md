# Accuracy analysis: python-added-if-block

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 349 nodes, 349 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 1409: renderer 0, matcher 0 |
| Painting, Full | 12 bytes wrong of 1409: renderer 12, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A handmade fixture (no README). In the `__main__` block, `print(f"Indices: {result}")` (line 21) is
wrapped in a new `if result != [0, 1]:` and re-indented by four spaces (after lines 21-22).

## Painting

Both runs are the same with the human mapping, so they are the renderer's. Minimal agrees exactly.

- **None -> Move, before line 21, 4 bytes (the old indentation), Full, `renderer`.**
- **None -> Insert/Move, after line 22, 8 bytes (the new indentation), Full, `renderer`.**

The Full painting paints the wrapped line's whitespace too:

- the old 4-space indentation as moved to columns 4-8 of after line 22 (Match);
- the new outer 4 columns as Insert.

omnidiff's Full rendering paints the `print(...)` itself as moved (agreeing with the painting) and
the `if` line's indentation as Insert. But it leaves the indentation of the re-indented line
unpainted on both sides. Full "accounts for every byte whose role changed, padding included", so
this is a Full rendering gap, not a convention mismatch.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full rendering of re-indented (`WrapGrowth`/moved) lines in
  src/diff/text.rs: paint the leading indentation of a line whose first node is painted Move (old
  indent as Move, the added columns as Insert)
- Ground truth questionable: no
