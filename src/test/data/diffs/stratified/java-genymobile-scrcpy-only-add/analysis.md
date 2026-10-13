# Accuracy analysis: java-genymobile-scrcpy-only-add

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 511 nodes, 511 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 1865: renderer 0, matcher 0 |
| Painting, Full | 34 bytes wrong of 1865: renderer 34, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In scrcpy's `DisplayInfo`, a `dpi` field is added, with a constructor parameter `int dpi`, its
assignment and a `getDpi()` getter. One painting is on file, Minimal-style, so it scores both
presets.

## Painting

Full only, 34 bytes, all renderer-owned and all `grading-artifact`:

- **Indentation, 28 bytes (`Insert -> None`).** Full paints the leading whitespace of the inserted
  field (line 9), assignment (line 19) and getter (lines 42-44) as Insert. The Minimal-style
  painting starts each insertion at the first character.
- **`) {`, 6 bytes (`Move -> None`).** The inserted `, int dpi` pushes the constructor's `) {`
  right, and the inserted field line pushes it down a row (line 12 -> 13). Full paints such a
  displaced span Move by design (`RenderOptions::paint_displaced_moves`), and the painting leaves
  it.

## Verdict

- Primary cause: `grading-artifact`
- Other causes: none
- Where a fix would go: none: these are Full conventions graded against a single Minimal-style
  painting
- Ground truth questionable: no
