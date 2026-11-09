# Accuracy analysis: swift-swiftlang-swift-insert-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 229 nodes, 229 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 1646: renderer 0, matcher 0 |
| Painting, Full | 2 bytes wrong of 1646: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In `test/embedded/availability-macos.swift`, the commit adds a `// RUN:` line (after line 6) and
public declarations (`protocol P`, `struct S`, `enum E`, after lines 12-15, and six more
functions/types after lines 18-23). It prefixes the two existing `func f`/`func g` lines with
`public ` (before lines 11-12, after lines 16-17).

## Painting

- **None -> Insert, after lines 16 and 17, 2 bytes (the space between the new `public` and
  `func`), Full, `renderer`.** The Full painting paints `public` and the following space as
  inserted, and the existing declaration as a moved Match. omnidiff paints `public` Insert and the
  declaration as the painting does, but leaves the one-byte gap between them unpainted. That gap
  belongs to neither node. The runs are the same with the human mapping. Minimal leaves the space
  out by convention and agrees.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full gap-text painting in src/diff/text.rs: whitespace between an inserted
  leading modifier and the unchanged node it attaches to belongs to the insertion
- Ground truth questionable: no
