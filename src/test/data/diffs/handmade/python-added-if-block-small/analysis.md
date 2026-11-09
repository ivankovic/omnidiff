# Accuracy analysis: python-added-if-block-small

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 62 nodes, 62 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 141: renderer 0, matcher 0 |
| Painting, Full | 10 bytes wrong of 141: renderer 10, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A handmade fixture (no README). `print("Zero")` (line 3) is wrapped in a new `if numer == 0:`. The
wrapped line goes from 4 to 6 spaces of indentation (after lines 3-4).

## Painting

Both runs are the same with the human mapping, so they are the renderer's. Minimal agrees exactly.

- **None -> Update, before line 3, 4 bytes (the old indentation), Full, `renderer`.**
- **None -> Insert/Update, after line 4, 6 bytes (the new indentation), Full, `renderer`.**

The Full painting maps the old 4-space indentation to columns 4-6 of after line 4 (a Match with
different text, so Update) and inserts columns 0-4. omnidiff's Full rendering paints the `print`
as moved, as the painting does, but leaves the indentation of the re-indented line unpainted on
both sides. This is the same gap as `python-added-if-block`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full rendering of re-indented (wrapped/moved) lines in src/diff/text.rs:
  paint the leading indentation of a line whose first node is painted Move
- Ground truth questionable: no
