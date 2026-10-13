# Accuracy analysis: java-defects4j-jacksondatabind-17-objectmapper

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 28226 nodes, 28226 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 244393: renderer 0, matcher 0 |
| Painting, Full | 8 bytes wrong of 244393: renderer 8, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Line 180, the second line of a condition: `|| TreeNode.class.isAssignableFrom(t.getRawClass()))`
becomes `&& !TreeNode.class.isAssignableFrom(t.getRawClass()))`, re-indented by 8 columns.

## Painting

Minimal matches exactly (Update `||` -> `&&`, Insert `!`). Full, 8 bytes, renderer (same with the
human mapping), line 180:

- `None -> Insert`, after side, 6 bytes: the Full painting paints six of the eight added indentation
  spaces (columns 27-32) Insert. omnidiff's Full rendering does not paint indentation growth on a
  row whose content it maps.
- `None -> Move`, 1 byte each side: the space after the operator, shifted 8 columns right, which the
  Full painting marks Move and omnidiff leaves unpainted.

`renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs Full painting of indentation growth and shifted separator
  whitespace
- Ground truth questionable: no
