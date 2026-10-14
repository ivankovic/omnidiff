# Accuracy analysis: java-defects4j-chart-24-graypaintscale

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 932 nodes, 932 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 10682: renderer 4, matcher 0 |
| Painting, Full | 0 bytes wrong of 10682: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Chart-24: `GrayPaintScale.getPaint` (line 126) computed the grey level from the raw
`value` instead of the clamped `v`; the fix replaces `value` with `v` in that expression.

## Painting

No mapping mismatches; Full agrees with its painting. The one Minimal run also appears with the
human mapping, so it is the renderer's:

- Minimal, 4 bytes, Update -> Delete: `alue` of `value` on before line 126. omnidiff narrows the
  `value` -> `v` Update to the differing characters and labels the leftover `alue` Update. The
  Minimal painting reads it as a kept `v` with `alue` deleted. When the narrowed after side is
  empty, the painting's label is Delete. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` Update narrowing under `MINIMAL`: the label of a narrowed
  run whose counterpart is empty
- Ground truth questionable: no
