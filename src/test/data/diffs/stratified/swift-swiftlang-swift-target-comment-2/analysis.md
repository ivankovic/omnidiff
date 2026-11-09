# Accuracy analysis: swift-swiftlang-swift-target-comment-2

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 138 nodes, 138 graded, N:M floor 0 visible |
| Painting, Minimal | 1 bytes wrong of 1028: renderer 1, matcher 0 |
| Painting, Full | 0 bytes wrong of 1028: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In `test/embedded/string-switch.swift`, the commit appends ` %target-embedded-posix-shim` to the
compiler options of the `// RUN:` comment (line 1). The fixture has one painting (`Only one
solution`), which is scored under both presets.

## Painting

- **None -> Insert, after line 1, 1 byte (the space before `%target-embedded-posix-shim`), Minimal,
  `grading-artifact`.** The painting's Insert starts with the separating space. omnidiff's Minimal
  rendering trims whitespace at the ends of every range (painting rule 4), so it leaves the space
  unpainted. The Full rendering keeps it and agrees exactly. The run is the same with the human
  mapping. The single painting is in the padded style, so only Minimal's deliberate trimming
  disagrees with it. Neither the matcher nor the renderer is wrong on its own terms. This is the
  same case as `swift-swiftlang-swift-add-target-comment` (same commit).

## Verdict

- Primary cause: `grading-artifact`
- Other causes: none
- Where a fix would go: none (Minimal trimming is the intended convention; a single-solution
  painting that includes the space cannot match both presets)
- Ground truth questionable: no
