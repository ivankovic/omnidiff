# Accuracy analysis: python-add-remove-block

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 315 nodes, 315 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 1014: renderer 0, matcher 0 |
| Painting, Full | 1 bytes wrong of 1014: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A handmade fixture. The docstring of `process_data` gains ` and filtering` (line 2), and a new
filtering block (`if len(result) > 0:` with a nested `for`/`if` and an early `return filtered`) is
appended to the function (after lines 11-18). description.md describes it as a new code block that
changes the function's behaviour.

## Painting

- **Insert -> None, after line 2, 1 byte (the space before `and filtering`), Full, `renderer`.**
  Both paintings insert `and filtering` without the separating space. omnidiff's Full rendering of
  the docstring update paints ` and filtering`, including the leading space. Minimal trims
  whitespace at range ends (painting rule 4), so it agrees there. The run is the same with the human
  mapping. Full paints padding by design, so this is a one-byte difference in convention between
  the Full renderer and this hand painting. The Full painting is not wrong on its own terms.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full update narrowing in src/diff/text.rs: whether an inserted word's
  separating space belongs to the insertion (convention; could equally stay as is)
- Ground truth questionable: no
